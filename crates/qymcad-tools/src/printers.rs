//! THE PRINTERS A CHECK KNOWS BY NAME: the room each builds in, mm - not its plate, which can be wider (a plate of 350
//! printing 325 with one nozzle and 300 with two). One name stands for a line of printers that build in one room.
//!
//! A resource: the built-in list is `assets/printers.json`, and a person's own `printers.json` in the program's data
//! folder adds printers or sets a name of the list anew, without rebuilding the program.

use serde::Deserialize;

/// A printer by name.
#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Printer {
    pub name: String,
    /// The printers of the line, as their makers name them.
    pub printers: Vec<String>,
    /// The room it builds in: x, y, z, mm.
    pub build: [f64; 3],
}

static BUILT_IN: &str = include_str!("../../../assets/printers.json");

/// The printers of a list written as `printers.json`; nothing for a list that does not read.
fn read(text: &str) -> Vec<Printer> {
    serde_json::from_str(text).unwrap_or_default()
}

/// Every printer known: the built-in list, a person's own over it by name.
pub fn all() -> Vec<Printer> {
    let mut out = read(BUILT_IN);
    let own = qymcad_paths::config("printers.json").and_then(|p| std::fs::read_to_string(p).ok()).map(|t| read(&t)).unwrap_or_default();
    for p in own {
        match out.iter_mut().find(|q| q.name == p.name) {
            Some(q) => *q = p,
            None => out.push(p),
        }
    }
    out
}

/// The printer named `name`, the case of its letters aside.
pub fn named(name: &str) -> Option<Printer> {
    all().into_iter().find(|p| p.name.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    /// THE BUILT-IN LIST READS, every name once, every room a real size.
    #[test]
    fn the_built_in_list_reads() {
        let list = super::read(super::BUILT_IN);
        assert!(list.len() >= 15, "the built-in list reads as {} printers", list.len());
        let mut names: Vec<&str> = list.iter().map(|p| p.name.as_str()).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), list.len(), "a name stands twice in the list");
        assert!(list.iter().all(|p| p.build.iter().all(|s| (100.0..=1000.0).contains(s))), "a room out of size in the list");
        assert_eq!(super::named("H2D").map(|p| p.build), Some([300.0, 320.0, 325.0]), "the room of two nozzles is not the plate's");
    }
}
