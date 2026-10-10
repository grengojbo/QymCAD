//! The channel over a named pipe: Windows.
//!
//! A PIPE HAS NO FOLDER, so its owner's folder cannot keep others out the way it does for a Unix socket. The pipe
//! lives in one namespace for the whole machine, and two things are made explicit here instead:
//!
//! - the window's pipe is created with an owner and an access list naming the person alone (the default list lets
//!   every user of the machine read it), refuses clients from other machines, and is refused itself when a pipe of
//!   that name already exists - a pipe another user made first is not joined;
//! - the program checks, before it sends a call, that the pipe it reached is owned by the person who started it: a
//!   pipe of the right name made by another user would otherwise read every call and answer them.

use std::fs::File;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_ACCESS_DENIED, ERROR_FILE_NOT_FOUND, ERROR_PIPE_BUSY, ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE,
};
use windows_sys::Win32::Security::Authorization::{ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1, SE_KERNEL_OBJECT};
use windows_sys::Win32::Security::{EqualSid, GetTokenInformation, TokenUser, OWNER_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER};
use windows_sys::Win32::Storage::FileSystem::{CreateFileW, FILE_FLAG_FIRST_PIPE_INSTANCE, OPEN_EXISTING, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PeekNamedPipe, WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};
use windows_sys::Win32::System::IO::CancelSynchronousIo;

use crate::wire::{self, Answering, Answers, Call, Heard, Link};
use crate::{LinkError, Opened, Wait, Wake};

/// The size of each direction's buffer in the pipe, bytes: an answer with a picture runs to some hundreds of kB, and
/// the buffer is only a hint - a longer line passes in parts.
const BUFFER: u32 = 64 * 1024;

/// How long the program waits for a free instance of the pipe, ms, when the window is between two clients.
const FREE_WITHIN: u32 = 2000;

/// How often the program looks for an answer in the pipe while it waits. A call to the window takes from some
/// milliseconds to minutes; a look every 2 ms costs nothing beside that.
const LOOK_EVERY: Duration = Duration::from_millis(2);

/// How long the window waits for a thread of its channel to give up its pipe when the channel closes; a thread
/// woken from its read ends in a millisecond.
const LET_GO_WITHIN: Duration = Duration::from_secs(2);

/// THE PIPE FOR `path`: the folder a socket would lie in names the pipe, through a digest of its path, so each person's
/// folder - and each check's - has a pipe of its own in the machine's one namespace.
pub fn pipe_name(path: &Path) -> String {
    // FNV-1a over the path as written, case folded: Windows paths do not tell case apart
    let mut digest: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in path.to_string_lossy().to_lowercase().bytes() {
        digest ^= u64::from(byte);
        digest = digest.wrapping_mul(0x0100_0000_01b3);
    }
    format!(r"\\.\pipe\qymcad-{digest:016x}")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

fn last_error() -> u32 {
    // SAFETY: reads the calling thread's last error; no arguments.
    unsafe { GetLastError() }
}

/// THE SID OF THE PERSON WHO STARTED THIS PROCESS, in the buffer the system wrote it to.
struct User {
    /// `TOKEN_USER` and the SID it points into; `u64` for the alignment of the pointer at its head.
    buffer: Vec<u64>,
}

impl User {
    fn of_this_process() -> Result<User, String> {
        let mut token: HANDLE = std::ptr::null_mut();
        // SAFETY: the pseudo-handle of this process needs no closing; `token` is written on success and closed below.
        if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
            return Err(format!("the token of this process could not be opened (error {})", last_error()));
        }
        let mut length = 0u32;
        // SAFETY: a first call with no buffer only asks for the length.
        unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut length) };
        let mut buffer = vec![0u64; (length as usize).div_ceil(8).max(1)];
        // SAFETY: `buffer` holds at least `length` bytes.
        let read = unsafe { GetTokenInformation(token, TokenUser, buffer.as_mut_ptr().cast(), length, &mut length) };
        let error = last_error();
        // SAFETY: `token` was opened above and is not used after this.
        unsafe { CloseHandle(token) };
        if read == 0 {
            return Err(format!("the user of this process could not be read (error {error})"));
        }
        Ok(User { buffer })
    }

    fn sid(&self) -> PSID {
        // SAFETY: the buffer holds the `TOKEN_USER` the system wrote, aligned for it.
        unsafe { (*self.buffer.as_ptr().cast::<TOKEN_USER>()).User.Sid }
    }

    /// The SID as text, `S-1-5-21-...`.
    fn text(&self) -> Result<String, String> {
        let mut text: *mut u16 = std::ptr::null_mut();
        // SAFETY: `sid` points into the live buffer; the system allocates `text`, freed below.
        if unsafe { ConvertSidToStringSidW(self.sid(), &mut text) } == 0 {
            return Err(format!("the user's SID could not be written as text (error {})", last_error()));
        }
        // SAFETY: `text` is a zero-ended UTF-16 string from the system.
        let length = (0..).take_while(|i| unsafe { *text.add(*i) } != 0).count();
        // SAFETY: `length` units were counted above.
        let out = String::from_utf16_lossy(unsafe { std::slice::from_raw_parts(text, length) });
        // SAFETY: allocated by the system with LocalAlloc.
        unsafe { LocalFree(text.cast()) };
        Ok(out)
    }
}

/// A SECURITY DESCRIPTOR naming the person as the owner and as the one user let in, freed when dropped.
struct OwnerOnly {
    descriptor: PSECURITY_DESCRIPTOR,
}

impl OwnerOnly {
    fn new(user: &User) -> Result<OwnerOnly, String> {
        let sid = user.text()?;
        // O: the owner; D:P a protected access list, inheriting nothing; A;;GA the one entry: all rights, this user
        let sddl = wide(&format!("O:{sid}D:P(A;;GA;;;{sid})"));
        let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: `sddl` is zero-ended; the system allocates `descriptor`, freed on drop.
        if unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), SDDL_REVISION_1, &mut descriptor, std::ptr::null_mut()) } == 0 {
            return Err(format!("the access list of the pipe could not be made (error {})", last_error()));
        }
        Ok(OwnerOnly { descriptor })
    }
}

impl Drop for OwnerOnly {
    fn drop(&mut self) {
        // SAFETY: allocated by the system with LocalAlloc, used by nobody after this.
        unsafe { LocalFree(self.descriptor.cast()) };
    }
}

/// Whether an instance is the first of its name: the first refuses a name that already exists.
#[derive(Clone, Copy, PartialEq)]
enum Instance {
    First,
    Further,
}

/// ONE INSTANCE OF THE PIPE, for one client to connect to.
fn instance(name: &[u16], owner: &OwnerOnly, which: Instance) -> Result<File, u32> {
    let attributes = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: owner.descriptor, bInheritHandle: 0 };
    let first = if which == Instance::First { FILE_FLAG_FIRST_PIPE_INSTANCE } else { 0 };
    // SAFETY: `name` is zero-ended and `attributes` lives across the call; the handle is owned by the `File` made below.
    let handle = unsafe {
        CreateNamedPipeW(
            name.as_ptr(),
            PIPE_ACCESS_DUPLEX | first,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER,
            BUFFER,
            0,
            &attributes,
        )
    };
    if handle == INVALID_HANDLE_VALUE {
        return Err(last_error());
    }
    // SAFETY: a fresh handle this function alone holds.
    Ok(unsafe { File::from_raw_handle(handle) })
}

/// THE WINDOW'S END: the pipe, the queue of calls the window takes from, and the threads holding the pipe's instances.
pub struct Listener {
    name: Vec<u16>,
    queue: Receiver<Call>,
    stop: Arc<AtomicBool>,
    accepting: Option<JoinHandle<()>>,
    served: Served,
}

/// The threads answering clients, each holding one instance of the pipe.
type Served = Arc<Mutex<Vec<JoinHandle<()>>>>;

impl Listener {
    /// OPEN THE PIPE FOR `path`. A pipe of that name that already exists - another window's, or another user's - is
    /// refused. The pipe is the person's alone. `wake` is called each time a call is queued.
    pub fn open(path: &Path, wait: Wait, wake: Wake) -> Result<Listener, Opened> {
        let name = wide(&pipe_name(path));
        let user = User::of_this_process().map_err(Opened::Io)?;
        let owner = OwnerOnly::new(&user).map_err(Opened::Io)?;
        let first = match instance(&name, &owner, Instance::First) {
            Ok(f) => f,
            Err(ERROR_ACCESS_DENIED) => return Err(Opened::Taken),
            Err(e) => return Err(Opened::Io(format!("{}: error {e}", pipe_name(path)))),
        };
        let (queue_in, queue) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let served: Served = Arc::new(Mutex::new(Vec::new()));
        let accepting = Accepting { name: name.clone(), owner, queue: queue_in, answering: Answering { wait, wake }, served: served.clone(), stop: stop.clone() };
        let accepting = std::thread::spawn(move || accept(first, &accepting));
        Ok(Listener { name, queue, stop, accepting: Some(accepting), served })
    }

    /// THE NEXT CALL TO DO, or nothing when none waits. Never blocks: the window asks once a frame.
    pub fn next(&self) -> Option<Call> {
        wire::next(&self.queue)
    }
}

impl Drop for Listener {
    /// CLOSE THE WINDOW'S END, EVERY INSTANCE OF THE PIPE WITH IT. A pipe's name lives while any of its instances is
    /// open, and an instance a connected client held kept it: the same window switched off and on again was refused
    /// its own name as taken. So the queue goes first - a call waiting in it is dropped unanswered and read on the
    /// program's end as the window gone - then the thread waiting for a client and each thread answering one is
    /// woken from its wait and let end, closing its instance.
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        drop(std::mem::replace(&mut self.queue, mpsc::channel().1));
        if let Ok(f) = open_pipe(&self.name) {
            drop(f);
        }
        if let Some(t) = self.accepting.take() {
            let_go(t);
        }
        let served = std::mem::take(&mut *self.served.lock().unwrap_or_else(std::sync::PoisonError::into_inner));
        for t in served {
            let_go(t);
        }
    }
}

/// WAKE THE THREAD `t` FROM A READ OR A WAIT ON ITS PIPE and let it end, for at most `LET_GO_WITHIN`. A wait for a
/// client or a read of a pipe opened without overlapping has no timeout; cancelling the thread's waiting call is the
/// one way to end it from outside.
fn let_go(t: JoinHandle<()>) {
    use std::os::windows::io::AsRawHandle as _;
    let until = std::time::Instant::now() + LET_GO_WITHIN;
    while !t.is_finished() && std::time::Instant::now() < until {
        // SAFETY: the handle of a thread that has not been joined, so it is alive; a thread not waiting is not touched.
        unsafe { CancelSynchronousIo(t.as_raw_handle() as HANDLE) };
        std::thread::sleep(Duration::from_millis(1));
    }
    if t.is_finished() {
        let _ = t.join();
    }
}

/// What the thread accepting clients holds: the pipe's name and access list for each new instance, how a client is
/// answered, the list the threads answering them go to, and the sign to stop.
struct Accepting {
    name: Vec<u16>,
    owner: OwnerOnly,
    queue: Sender<Call>,
    answering: Answering,
    served: Served,
    stop: Arc<AtomicBool>,
}

// SAFETY: the descriptor is read by the system only while an instance is created, on this thread alone, and freed when
// the thread ends.
unsafe impl Send for Accepting {}

/// ACCEPT CLIENTS until the window's end closes: wait for a client on the instance in hand, lay the next instance
/// before answering it, and answer it on a thread of its own.
fn accept(first: File, accepting: &Accepting) {
    let mut waiting = first;
    loop {
        // SAFETY: a live handle owned by `waiting`; no overlapped structure, so the call blocks until a client comes
        // or the window's end cancels it.
        let joined = unsafe { ConnectNamedPipe(waiting.as_raw_handle() as HANDLE, std::ptr::null_mut()) } != 0 || last_error() == ERROR_PIPE_CONNECTED;
        if accepting.stop.load(Ordering::Acquire) {
            return;
        }
        let Ok(next) = instance(&accepting.name, &accepting.owner, Instance::Further) else { return };
        let client = std::mem::replace(&mut waiting, next);
        if !joined {
            continue;
        }
        let Ok(read) = client.try_clone() else { continue };
        let (queue, answering) = (accepting.queue.clone(), accepting.answering.clone());
        let t = std::thread::spawn(move || wire::serve(read, client, &queue, &answering));
        let mut served = accepting.served.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        served.retain(|t| !t.is_finished());
        served.push(t);
    }
}

/// Open the pipe `name` as a client.
fn open_pipe(name: &[u16]) -> Result<File, u32> {
    for _ in 0..3 {
        // SAFETY: `name` is zero-ended; the handle is owned by the `File` made below.
        let handle = unsafe { CreateFileW(name.as_ptr(), GENERIC_READ | GENERIC_WRITE, 0, std::ptr::null(), OPEN_EXISTING, 0, std::ptr::null_mut()) };
        if handle != INVALID_HANDLE_VALUE {
            // SAFETY: a fresh handle this function alone holds.
            return Ok(unsafe { File::from_raw_handle(handle) });
        }
        let error = last_error();
        if error != ERROR_PIPE_BUSY {
            return Err(error);
        }
        // every instance is taken for a moment: the window lays the next one as soon as a client joins
        // SAFETY: `name` is zero-ended.
        unsafe { WaitNamedPipeW(name.as_ptr(), FREE_WITHIN) };
    }
    Err(ERROR_PIPE_BUSY)
}

/// WHETHER THE PIPE `pipe` IS OWNED BY THE PERSON WHO STARTED THIS PROCESS.
fn owned_by_this_user(pipe: &File) -> Result<bool, String> {
    let user = User::of_this_process()?;
    let mut owner: PSID = std::ptr::null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: a live handle; `owner` points into `descriptor`, which the system allocates and is freed below.
    let error = unsafe {
        GetSecurityInfo(pipe.as_raw_handle() as HANDLE, SE_KERNEL_OBJECT, OWNER_SECURITY_INFORMATION, &mut owner, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), &mut descriptor)
    };
    if error != 0 {
        return Err(format!("the owner of the pipe could not be read (error {error})"));
    }
    // SAFETY: both SIDs are live: the user's in its buffer, the owner's in `descriptor`.
    let same = unsafe { EqualSid(owner, user.sid()) } != 0;
    // SAFETY: allocated by the system with LocalAlloc, not used after this.
    unsafe { LocalFree(descriptor.cast()) };
    Ok(same)
}

/// THE ANSWERS OVER A PIPE: what has come is looked at without waiting (`PeekNamedPipe`) and only that much is read,
/// so the read never blocks the pipe; past the time the answer is late. A pipe has no timeout for a read, and a read
/// waiting on a thread of its own held the next write of the same pipe for ever.
struct PipeAnswers {
    pipe: File,
    /// What came after the last whole line.
    pending: Vec<u8>,
}

impl Answers for PipeAnswers {
    fn line_within(&mut self, late: Duration) -> Heard {
        use std::io::Read;
        let until = std::time::Instant::now() + late;
        loop {
            if let Some(end) = self.pending.iter().position(|b| *b == b'\n') {
                let line: Vec<u8> = self.pending.drain(..=end).collect();
                return Heard::Line(String::from_utf8_lossy(&line).into_owned());
            }
            let mut waiting = 0u32;
            // SAFETY: a live handle; nothing is copied out, only the count of bytes waiting is written.
            let looked = unsafe { PeekNamedPipe(self.pipe.as_raw_handle() as HANDLE, std::ptr::null_mut(), 0, std::ptr::null_mut(), &mut waiting, std::ptr::null_mut()) } != 0;
            if !looked {
                return Heard::End;
            }
            if waiting > 0 {
                let mut chunk = vec![0u8; waiting as usize];
                match self.pipe.read(&mut chunk) {
                    Ok(0) | Err(_) => return Heard::End,
                    Ok(n) => self.pending.extend_from_slice(&chunk[..n]),
                }
                continue;
            }
            if std::time::Instant::now() >= until {
                return Heard::Late;
            }
            std::thread::sleep(LOOK_EVERY);
        }
    }
}

/// CONNECT TO THE WINDOW FOR `path`; an answer is waited for at most `late`.
pub fn connect(path: &Path, late: Duration) -> Result<Link, LinkError> {
    let pipe = open_pipe(&wide(&pipe_name(path))).map_err(|e| match e {
        ERROR_FILE_NOT_FOUND => LinkError::NoWindow,
        e => LinkError::Broken(format!("{}: error {e}", pipe_name(path))),
    })?;
    if !owned_by_this_user(&pipe).map_err(LinkError::Broken)? {
        return Err(LinkError::Broken(format!("{} belongs to another user of this machine; no call is sent to it", pipe_name(path))));
    }
    let read = pipe.try_clone().map_err(|e| LinkError::Broken(e.to_string()))?;
    Ok(Link::over(PipeAnswers { pipe: read, pending: Vec::new() }, pipe, late))
}
