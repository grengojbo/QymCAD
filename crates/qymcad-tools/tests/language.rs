//! THE PERSON'S LANGUAGE, as the tools tell it: read from the settings file where the window keeps it - the code the
//! person picked, the system's when none was picked or the file cannot be read - and handed on with the account of
//! the document, so a model names the menus as the person's window shows them.
use qymcad_tools::person::language_in;
use qymcad_tools::tool::{self, Ctx};
use serde_json::json;

/// A settings file as the window writes it: a map of what it keeps, the settings among them as text.
fn settings_with(dir: &std::path::Path, name: &str, settings: &str) -> std::path::PathBuf {
    let file = dir.join(name);
    let kept = ron::to_string(&std::collections::HashMap::from([("settings".to_string(), settings.to_string()), ("window".to_string(), "()".to_string())])).expect("the map is written");
    std::fs::write(&file, kept).expect("the settings file is written");
    file
}

#[test]
fn the_language_is_read_where_the_window_keeps_it() {
    let dir = std::env::temp_dir().join(format!("qymcad-person-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder for the check");
    let system = qymcad_i18n::system_default();
    struct Case {
        what: &'static str,
        file: Option<std::path::PathBuf>,
        want: String,
    }
    let broken = dir.join("broken.ron");
    std::fs::write(&broken, "{ not ron").expect("a broken file");
    let cases = [
        Case { what: "picked Russian", file: Some(settings_with(&dir, "ru.ron", "(layout:[],language:\"ru\",scheme:\"dark\",claude_link:On)")), want: "ru".into() },
        Case { what: "picked Ukrainian", file: Some(settings_with(&dir, "uk.ron", "(language:\"uk\")")), want: "uk".into() },
        Case { what: "never picked", file: Some(settings_with(&dir, "empty.ron", "(language:\"\",scheme:\"dark\")")), want: system.clone() },
        Case { what: "a language this build lacks", file: Some(settings_with(&dir, "xx.ron", "(language:\"xx\")")), want: system.clone() },
        Case { what: "a file that is no settings", file: Some(broken), want: system.clone() },
        Case { what: "no file", file: Some(dir.join("absent.ron")), want: system.clone() },
        Case { what: "no folder for the program", file: None, want: system.clone() },
    ];
    let mut wrong = Vec::new();
    for c in &cases {
        let got = language_in(c.file.as_deref());
        if got != c.want {
            wrong.push(format!("{}: {got:?}, not {:?}", c.what, c.want));
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
    assert!(wrong.is_empty(), "the person's language is misread:\n  {}", wrong.join("\n  "));
}

#[test]
fn the_account_of_the_document_names_the_persons_language() {
    let mut ctx = Ctx::blank();
    ctx.language = "uk".into();
    let found = tool::find("get_document").expect("the tool");
    let reply = tool::call(&mut ctx, found, json!({}));
    assert_eq!(reply["structuredContent"]["language"], json!("uk"), "{reply}");
}
