//! THE HELP ON THE DOCUMENTATION SITE: the same articles the window shows, laid out as pages of the site's book.
//!
//! The window reads a link from the root of the help (`general/05-parameters`, `img/window.png`) and a folder of
//! frames as an animation (`img/part-fillet/`). A page of the book is read by a browser from where it lies, so every
//! link is rewritten relative to its page, an article gets its `.md`, and a folder of frames becomes its frames in a
//! row. A link that names nothing is refused here, before the book is built: a browser would only show it broken.
use std::path::Path;

use crate::{articles, frames, image, sections, set_lang, SECTION_ORDER, HELP};

/// The folder of the book the help is laid out in, under the book's `src`.
pub const FOLDER: &str = "help";

/// ONE PAGE OF THE HELP in the book: where it lies under `src` and what it says.
pub struct Page {
    pub path: String,
    pub text: String,
}

/// THE HELP OF ONE LANGUAGE as a part of the book: its pages, the images they show, and the lines of the table of
/// contents that list them.
pub struct HelpBook {
    pub pages: Vec<Page>,
    pub images: Vec<String>,
    pub contents: String,
}

/// THE HELP OF `lang` laid out for the book, or every link that names nothing, one a line.
///
/// # Errors
/// The links of the articles that do not lead to an article, an image or a folder of frames of the help.
pub fn book(lang: &str) -> Result<HelpBook, String> {
    set_lang(lang);
    let known: Vec<String> = articles(lang).into_iter().filter(|a| crate::visible(a)).collect();
    let mut pages = Vec::new();
    let mut images = Vec::new();
    let mut wrong = Vec::new();
    for a in &known {
        let Some(md) = HELP.get_file(format!("{lang}/{a}.md")).and_then(|f| f.contents_utf8()) else { continue };
        match rewrite(md, a, &known) {
            Ok(r) => {
                images.extend(r.images);
                pages.push(Page { path: format!("{FOLDER}/{a}.md"), text: r.text });
            }
            Err(e) => wrong.extend(e.into_iter().map(|e| format!("{lang}/{a}: {e}"))),
        }
    }
    if !wrong.is_empty() {
        return Err(wrong.join("\n"));
    }
    images.sort();
    images.dedup();
    Ok(HelpBook { pages, images, contents: contents(lang) })
}

/// WRITE THE HELP OF `lang` into the book whose sources are `src`: the pages and the images under `help/`, and its
/// part appended to the table of contents.
///
/// # Errors
/// A link that names nothing (see `book`), or a file that cannot be written.
pub fn write(lang: &str, src: &Path) -> Result<(), String> {
    let help = book(lang)?;
    let put = |rel: &str, bytes: &[u8]| {
        let p = src.join(rel);
        std::fs::create_dir_all(p.parent().unwrap_or(src)).and_then(|()| std::fs::write(&p, bytes)).map_err(|e| format!("{}: {e}", p.display()))
    };
    for page in &help.pages {
        put(&page.path, page.text.as_bytes())?;
    }
    for img in &help.images {
        let bytes = image(img).ok_or_else(|| format!("{img} is not in the help"))?;
        put(&format!("{FOLDER}/{img}"), bytes)?;
    }
    let summary = src.join("SUMMARY.md");
    let mut text = std::fs::read_to_string(&summary).map_err(|e| format!("{}: {e}", summary.display()))?;
    text.push('\n');
    text.push_str(&help.contents);
    std::fs::write(&summary, text).map_err(|e| format!("{}: {e}", summary.display()))
}

/// THE PART OF THE TABLE OF CONTENTS for the help: its title, the overview, then a section a chapter with its
/// articles under it, in the order and with the captions the window shows.
fn contents(lang: &str) -> String {
    let word = |key: &str| qymcad_i18n::tr_in(lang, key).unwrap_or_else(|| key.to_string());
    let title = |a: &str| crate::title_in(lang, a).unwrap_or_else(|| a.to_string());
    let link = |a: &str| format!("[{}]({FOLDER}/{a}.md)", title(a));
    let mut out = format!("# {}\n\n", word("help-title"));
    for (dir, items) in sections() {
        if dir.is_empty() {
            for a in &items {
                out.push_str(&format!("- {}\n", link(a)));
            }
            continue;
        }
        let caption = word(&format!("help-section-{dir}"));
        let overview = format!("{dir}/index");
        let rest: Vec<&String> = items.iter().filter(|a| **a != overview).collect();
        if items.contains(&overview) {
            out.push_str(&format!("- [{caption}]({FOLDER}/{overview}.md)\n"));
        } else {
            out.push_str(&format!("- [{caption}]()\n"));
        }
        for a in rest {
            out.push_str(&format!("  - {}\n", link(a)));
        }
    }
    out
}

/// An article rewritten for its page, and the images it shows.
struct Rewritten {
    text: String,
    images: Vec<String>,
}

/// REWRITE THE LINKS OF `md`, the article `at`, relative to its page; refuse the ones that name nothing.
fn rewrite(md: &str, at: &str, known: &[String]) -> Result<Rewritten, Vec<String>> {
    // an article in a section lies one folder deeper than the overview of the help
    let up = "../".repeat(at.matches('/').count());
    let mut out = String::with_capacity(md.len());
    let mut images = Vec::new();
    let mut wrong = Vec::new();
    let mut rest = md;
    while let Some(open) = rest.find("](") {
        let Some(close) = rest[open + 2..].find(')') else { break };
        let target = &rest[open + 2..open + 2 + close];
        let label_start = rest[..open].rfind('[').unwrap_or(open);
        let picture = label_start > 0 && rest[..label_start].ends_with('!');
        let head = if picture { label_start - 1 } else { label_start };
        out.push_str(&rest[..head]);
        let label = &rest[label_start + 1..open];
        let (anchor_less, anchor) = match target.split_once('#') {
            Some(split) => split,
            None => (target, ""),
        };
        let anchor = if anchor.is_empty() { String::new() } else { format!("#{anchor}") };
        let bang = if picture { "!" } else { "" };
        if target.starts_with("http://") || target.starts_with("https://") || target.starts_with("mailto:") || target.starts_with('#') {
            out.push_str(&format!("{bang}[{label}]({target})"));
        } else if anchor_less.starts_with("img/") && anchor_less.ends_with('/') {
            let shown = frames(anchor_less);
            if shown.is_empty() {
                wrong.push(format!("{target}: no frames in this folder"));
            }
            out.push_str("<div style=\"display:flex;flex-wrap:wrap;gap:8px\">");
            for f in &shown {
                out.push_str(&format!("<img src=\"{up}{f}\" alt=\"{}\" style=\"max-width:32%\">", label.replace('"', "&quot;")));
            }
            out.push_str("</div>");
            images.extend(shown);
        } else if anchor_less.starts_with("img/") {
            if image(anchor_less).is_none() {
                wrong.push(format!("{target}: no such image"));
            }
            out.push_str(&format!("{bang}[{label}]({up}{anchor_less}{anchor})"));
            images.push(anchor_less.to_string());
        } else if SECTION_ORDER.iter().any(|s| anchor_less.starts_with(&format!("{s}/"))) || anchor_less == "index" {
            if !known.iter().any(|a| a == anchor_less) {
                wrong.push(format!("{target}: no such article"));
            }
            out.push_str(&format!("{bang}[{label}]({up}{anchor_less}.md{anchor})"));
        } else {
            wrong.push(format!("{target}: a link of a form the site does not know"));
            out.push_str(&format!("{bang}[{label}]({target})"));
        }
        rest = &rest[open + 2 + close + 1..];
    }
    out.push_str(rest);
    if wrong.is_empty() {
        Ok(Rewritten { text: out, images })
    } else {
        Err(wrong)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> Vec<String> {
        articles("en")
    }

    /// EVERY LINK OF EVERY LANGUAGE LEADS SOMEWHERE on the site: to an article, an image or a folder of frames.
    #[test]
    fn every_link_of_every_language_leads_to_a_page_or_a_picture() {
        let mut wrong = Vec::new();
        for lang in crate::languages() {
            if let Err(e) = book(&lang) {
                wrong.push(e);
            }
        }
        assert!(wrong.is_empty(), "links of the help that lead nowhere on the site:\n{}", wrong.join("\n"));
    }

    /// A LINK THAT NAMES NOTHING IS REFUSED, and so is one of a form the site does not know: built, it would only be a
    /// broken link on a page.
    #[test]
    fn a_link_that_names_nothing_is_refused() {
        struct Case {
            md: &'static str,
            said: &'static str,
        }
        let cases = [
            Case { md: "see [this](part/99-nothing)", said: "no such article" },
            Case { md: "![x](img/nothing.png)", said: "no such image" },
            Case { md: "see [this](../elsewhere.md)", said: "form the site does not know" },
        ];
        for c in cases {
            let e = rewrite(c.md, "index", &known()).err().unwrap_or_default();
            assert!(e.iter().any(|e| e.contains(c.said)), "{:?} was not refused with {:?}: {e:?}", c.md, c.said);
        }
    }

    /// A LINK IS READ FROM WHERE ITS PAGE LIES: an article in a section reaches another one and the images one folder
    /// up, and a folder of frames becomes its frames.
    #[test]
    fn a_link_is_relative_to_its_page() {
        let md = "See [parameters](general/05-parameters), ![window](img/window.png) and ![fillet](img/part-fillet/).";
        let r = rewrite(md, "part/05-fillet", &known()).unwrap_or_else(|e| panic!("refused: {e:?}"));
        assert!(r.text.contains("[parameters](../general/05-parameters.md)"), "{}", r.text);
        assert!(r.text.contains("![window](../img/window.png)"), "{}", r.text);
        assert!(r.text.contains("<img src=\"../img/part-fillet/00.png\""), "{}", r.text);
        assert!(!r.text.contains("](img/part-fillet/)"), "the folder of frames is left as a link: {}", r.text);
        assert!(r.images.iter().any(|i| i == "img/part-fillet/00.png") && r.images.iter().any(|i| i == "img/window.png"), "{:?}", r.images);
        let top = rewrite("[parameters](general/05-parameters)", "index", &known()).unwrap_or_else(|e| panic!("refused: {e:?}"));
        assert!(top.text.contains("](general/05-parameters.md)"), "{}", top.text);
    }

    /// EVERY ARTICLE IS IN THE TABLE OF CONTENTS ONCE, so a page of the help is never one only a link reaches.
    #[test]
    fn every_article_is_in_the_contents_once() {
        let mut wrong = Vec::new();
        for lang in crate::languages() {
            let help = book(&lang).unwrap_or_else(|e| panic!("{e}"));
            for page in &help.pages {
                let n = help.contents.matches(&format!("]({})", page.path)).count();
                if n != 1 {
                    wrong.push(format!("{lang}: {} is in the contents {n} times", page.path));
                }
            }
        }
        assert!(wrong.is_empty(), "{}", wrong.join("\n"));
    }
}
