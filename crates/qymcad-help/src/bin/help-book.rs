//! LAY THE HELP OF ONE LANGUAGE INTO THE SITE'S BOOK: `help-book <lang> <src of the book>`. Run by `site/build.sh`
//! for each language of the site, before mdBook builds it.
fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [lang, src] = args.as_slice() else {
        eprintln!("usage: help-book <lang> <src of the book>");
        std::process::exit(2);
    };
    if let Err(e) = qymcad_help::site::write(lang, std::path::Path::new(src)) {
        eprintln!("the help of {lang} was not laid into the book:\n{e}");
        std::process::exit(1);
    }
}
