//! THE NAMED PIPE IS THE PERSON'S ALONE (Windows). A pipe has no folder to keep others out: its owner and its access
//! list are the guard, and the program checks the owner before it sends a call. Here the window's pipe is read back as
//! the system holds it - the owner is the person, the access list one entry naming them - a pipe of that name made by
//! someone else first is not joined by the window, and the program sends nothing to it; and the pipe's name follows
//! the folder it stands for, case aside.
#![cfg(windows)]

use std::os::windows::io::AsRawHandle;
use std::time::Duration;

use qymcad_bridge::{pipe_name, Link, LinkError, Listener, Opened, Wait};
use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, GetSecurityInfo, SDDL_REVISION_1, SE_KERNEL_OBJECT};
use windows_sys::Win32::Security::{
    EqualSid, GetAce, GetAclInformation, GetTokenInformation, TokenUser, AclSizeInformation, ACCESS_ALLOWED_ACE, ACL, ACL_SIZE_INFORMATION, DACL_SECURITY_INFORMATION, OWNER_SECURITY_INFORMATION,
    PSECURITY_DESCRIPTOR, PSID, SECURITY_ATTRIBUTES, TOKEN_QUERY, TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::PIPE_ACCESS_DUPLEX;
use windows_sys::Win32::System::Pipes::{CreateNamedPipeW, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

const SOON: Wait = Wait { answer_within: Duration::from_secs(10) };

/// A folder of the check's own, named for it and this run; a pipe is named for its path.
fn place(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("qymcad-pipe-{name}-{}", std::process::id())).join("mcp.sock")
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// The `TOKEN_USER` of this process, its SID inside.
fn this_user() -> Vec<u64> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: the pseudo-handle of this process; `token` closed below.
    assert!(unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } != 0, "the token opens");
    let mut length = 0u32;
    // SAFETY: a first call asking only for the length.
    unsafe { GetTokenInformation(token, TokenUser, std::ptr::null_mut(), 0, &mut length) };
    let mut buffer = vec![0u64; (length as usize).div_ceil(8)];
    // SAFETY: `buffer` holds `length` bytes.
    assert!(unsafe { GetTokenInformation(token, TokenUser, buffer.as_mut_ptr().cast(), length, &mut length) } != 0, "the user is read");
    // SAFETY: opened above.
    unsafe { CloseHandle(token) };
    buffer
}

fn sid_of(user: &[u64]) -> PSID {
    // SAFETY: the buffer holds a `TOKEN_USER`.
    unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid }
}

#[test]
fn the_pipe_is_its_owners_alone() {
    let path = place("owner");
    let _window = Listener::open(&path, SOON, std::sync::Arc::new(|| {})).unwrap_or_else(|e| panic!("the pipe did not open: {e:?}"));
    let pipe = std::fs::OpenOptions::new().read(true).write(true).open(pipe_name(&path)).expect("the pipe takes a client");
    let user = this_user();
    let mut owner: PSID = std::ptr::null_mut();
    let mut dacl: *mut ACL = std::ptr::null_mut();
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: a live handle; `owner` and `dacl` point into `descriptor`, freed below.
    let error = unsafe {
        GetSecurityInfo(
            pipe.as_raw_handle() as HANDLE,
            SE_KERNEL_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            &mut owner,
            std::ptr::null_mut(),
            &mut dacl,
            std::ptr::null_mut(),
            &mut descriptor,
        )
    };
    assert_eq!(error, 0, "the security of the pipe is not read");
    // SAFETY: both SIDs live.
    let owned = unsafe { EqualSid(owner, sid_of(&user)) } != 0;
    let mut size = ACL_SIZE_INFORMATION { AceCount: 0, AclBytesInUse: 0, AclBytesFree: 0 };
    // SAFETY: `dacl` lives in `descriptor`; `size` is the record asked for.
    let sized = unsafe { GetAclInformation(dacl, (&mut size as *mut ACL_SIZE_INFORMATION).cast(), std::mem::size_of::<ACL_SIZE_INFORMATION>() as u32, AclSizeInformation) } != 0;
    let mut ace: *mut core::ffi::c_void = std::ptr::null_mut();
    // SAFETY: as above; the first entry is read only when there is one.
    let the_one_is_the_user =
        size.AceCount == 1 && unsafe { GetAce(dacl, 0, &mut ace) } != 0 && unsafe { EqualSid((&raw mut (*ace.cast::<ACCESS_ALLOWED_ACE>()).SidStart).cast(), sid_of(&user)) } != 0;
    // SAFETY: allocated by the system.
    unsafe { LocalFree(descriptor.cast()) };
    assert!(owned, "the pipe is not owned by the person who opened it");
    assert!(sized && size.AceCount == 1, "the access list of the pipe lets in {} entries, not the person alone", size.AceCount);
    assert!(the_one_is_the_user, "the one entry of the access list is not the person");
}

/// A PIPE OF THE WINDOW'S NAME MADE FIRST BY SOMEONE ELSE - here owned by the Administrators group and open to
/// everybody, as a pipe another user made would be: the window does not join it, and the program sends it nothing.
#[test]
fn a_pipe_made_by_someone_else_is_neither_joined_nor_called() {
    let path = place("squat");
    let sddl = wide("O:BAD:(A;;GA;;;WD)");
    let mut descriptor: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
    // SAFETY: `sddl` is zero-ended; `descriptor` freed below.
    assert!(unsafe { ConvertStringSecurityDescriptorToSecurityDescriptorW(sddl.as_ptr(), SDDL_REVISION_1, &mut descriptor, std::ptr::null_mut()) } != 0, "the descriptor is made");
    let attributes = SECURITY_ATTRIBUTES { nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32, lpSecurityDescriptor: descriptor, bInheritHandle: 0 };
    let name = wide(&pipe_name(&path));
    // SAFETY: `name` zero-ended, `attributes` alive across the call; the handle is closed below.
    let squatter = unsafe { CreateNamedPipeW(name.as_ptr(), PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE | PIPE_WAIT, PIPE_UNLIMITED_INSTANCES, 4096, 4096, 0, &attributes) };
    // SAFETY: allocated by the system; the pipe keeps its own copy.
    unsafe { LocalFree(descriptor.cast()) };
    if squatter == INVALID_HANDLE_VALUE {
        // only a member of the Administrators group may hand a pipe to it; the check runs where that holds (CI)
        eprintln!("PASSED OVER: this account cannot make a pipe owned by the Administrators group");
        return;
    }
    match Listener::open(&path, SOON, std::sync::Arc::new(|| {})) {
        Err(Opened::Taken) => {}
        Err(e) => panic!("the window refused the taken name for the wrong reason: {e:?}"),
        Ok(_) => panic!("the window joined a pipe someone else made"),
    }
    match Link::connect(&path, Duration::from_secs(2)) {
        Err(LinkError::Broken(why)) => assert!(why.contains("another user"), "{why}"),
        Err(e) => panic!("the program refused the pipe for the wrong reason: {e:?}"),
        Ok(_) => panic!("the program connected to a pipe that is not the person's"),
    }
    // SAFETY: made above.
    unsafe { CloseHandle(squatter) };
}

#[test]
fn the_pipe_follows_its_folder_case_aside() {
    let a = pipe_name(std::path::Path::new(r"D:\profiles\first\qymcad\data\mcp.sock"));
    let b = pipe_name(std::path::Path::new(r"D:\profiles\second\qymcad\data\mcp.sock"));
    let a_upper = pipe_name(std::path::Path::new(r"D:\PROFILES\FIRST\qymcad\data\mcp.sock"));
    assert!(a.starts_with(r"\\.\pipe\qymcad-"), "{a}");
    assert_ne!(a, b, "two people's folders share a pipe");
    assert_eq!(a, a_upper, "the same folder written in another case names another pipe");
}
