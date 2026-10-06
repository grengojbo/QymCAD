//! THE DOCUMENTATION SITE TELLS ONE STORY IN EVERY LANGUAGE, and tells it in the words the window uses.
//!
//! The guide to modelling with Claude is an mdBook a language under `site/`, written by hand in each and published
//! together. Books written apart drift apart: a chapter added to one, a section left out of another, a menu item
//! renamed in the window and not on the page. And the guide is first written in a notes app, whose own marks - a
//! wiki link, a callout, a "decide" left for later - mean nothing on a published page.
#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    fn site() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../site")
    }

    fn read(p: &std::path::Path) -> String {
        std::fs::read_to_string(p).unwrap_or_else(|e| panic!("{} must be readable: {e}", p.display()))
    }

    /// The languages of the site: every folder of `site/` with a book in it.
    fn languages() -> Vec<String> {
        let mut out: Vec<String> =
            std::fs::read_dir(site()).expect("site/ reads").flatten().filter(|e| e.path().join("book.toml").is_file()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        out.sort();
        out
    }

    /// The chapters a book's table of contents names, in its order.
    fn chapters(lang: &str) -> Vec<String> {
        read(&site().join(lang).join("src/SUMMARY.md")).lines().filter_map(|l| l.split_once("](").and_then(|(_, rest)| rest.split_once(')')).map(|(file, _)| file.to_string())).collect()
    }

    fn chapter(lang: &str, file: &str) -> String {
        read(&site().join(lang).join("src").join(file))
    }

    #[test]
    fn every_book_has_the_same_chapters_and_sections() {
        let langs = languages();
        assert_eq!(langs, ["en", "uk"], "the site's books are not the two it was written in");
        let mut wrong = Vec::new();
        let first = chapters(&langs[0]);
        assert!(first.len() > 5, "the {} book names {} chapters", langs[0], first.len());
        for lang in &langs[1..] {
            if chapters(lang) != first {
                wrong.push(format!("{lang} has chapters {:?}, {} has {first:?}", chapters(lang), langs[0]));
            }
        }
        for file in &first {
            let sections: Vec<usize> = langs.iter().map(|l| chapter(l, file).lines().filter(|x| x.starts_with("## ")).count()).collect();
            if sections.windows(2).any(|w| w[0] != w[1]) {
                wrong.push(format!("{file}: sections {sections:?} in {langs:?}"));
            }
        }
        assert!(wrong.is_empty(), "the books have drifted apart:\n{}", wrong.join("\n"));
    }

    /// The marks of a draft, one a line, from a file of their own: some are Ukrainian words.
    const DRAFT_MARKS: &str = include_str!("site_draft_marks.txt");

    /// NOTHING OF THE NOTES APP reaches a page: no wiki link, no callout, no question left for later.
    #[test]
    fn no_page_carries_the_marks_of_a_draft() {
        let mut found = Vec::new();
        for lang in languages() {
            for file in chapters(&lang) {
                for (i, line) in chapter(&lang, &file).lines().enumerate() {
                    for mark in DRAFT_MARKS.lines().filter(|m| !m.is_empty()) {
                        if line.contains(mark) {
                            found.push(format!("{lang}/{file}:{}: {line}", i + 1));
                        }
                    }
                }
            }
        }
        assert!(found.is_empty(), "a draft's marks reached the site:\n{}", found.join("\n"));
    }

    /// THE GUIDE NAMES THE MENU AS THE WINDOW DOES: the way to connect Claude is written in each book with the words
    /// the catalogue of that language gives the menu and the item. Renamed in one and not the other, a person looks for
    /// a menu item that is not there.
    #[test]
    fn the_guide_names_the_menu_in_the_windows_words() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let word = |lang: &str, key: &str| {
            let catalogue = read(&root.join("i18n").join(lang).join("main.ftl"));
            catalogue.lines().find_map(|l| l.strip_prefix(&format!("{key} = ")).map(str::to_string)).unwrap_or_else(|| panic!("no {key} in the {lang} catalogue"))
        };
        let mut wrong = Vec::new();
        for lang in languages() {
            let install = chapter(&lang, "install.md");
            let path = format!("{} \u{2192} {}", word(&lang, "menu-help"), word(&lang, "help-connect-claude"));
            if !install.contains(&path) {
                wrong.push(format!("{lang}/install.md does not say \"{path}\""));
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }

    /// EVERY BOOK IS BUILT AND PUBLISHED: the build script makes each language, and the publishing run uses that
    /// script with mdBook at a pinned version.
    #[test]
    fn every_book_is_built_and_published() {
        let build = read(&site().join("build.sh"));
        let mut wrong = Vec::new();
        for lang in languages() {
            if !build.split_whitespace().any(|w| w.trim_end_matches(';') == lang) {
                wrong.push(format!("site/build.sh does not build the {lang} book"));
            }
        }
        let pages = read(&site().join("../.github/workflows/pages.yml"));
        if !pages.contains("bash site/build.sh") {
            wrong.push("pages.yml does not build with site/build.sh".into());
        }
        if !pages.lines().any(|l| l.trim().starts_with("MDBOOK: \"") && l.chars().any(|c| c.is_ascii_digit())) {
            wrong.push("pages.yml does not pin the version of mdBook".into());
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
