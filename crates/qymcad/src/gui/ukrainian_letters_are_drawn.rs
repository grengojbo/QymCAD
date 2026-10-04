//! EVERY LETTER OF THE UKRAINIAN CATALOGUE AND HELP HAS A GLYPH IN THE INTERFACE FONTS.
//!
//! A letter the font lacks is drawn as a box, and a language is only as readable as its rarest letter:
//! U+0491, U+0454, U+0456, U+0457 and the apostrophe are the ones a Russian-shaped font set drops first. The catalogue
//! guard in `qymcad-i18n` bans symbols; it cannot see a missing letter, because the question needs the real
//! fonts the window installs.
//!
//! The Phosphor icon font sits at the head of the family and answers for a private-use range only, so for
//! ordinary letters "the font has a glyph" is a true claim here - unlike for arrows, where it was the wrong
//! font that answered.
//!
//! The ordinary family is asked through egui. The bold family cannot be: egui answers "no glyph" for a named
//! family for every character, Latin included, so that question was measured to be blind. Its one font
//! file is read directly instead.
#[cfg(test)]
mod tests {
    /// The whole Ukrainian alphabet in both cases, from a file of its own: a check that reads its letters
    /// from the catalogue alone would stay green if a letter were never used there.
    const ALPHABET: &str = include_str!("ukrainian_alphabet.txt");

    /// The apostrophe the catalogue and the help are written with.
    const APOSTROPHE: char = '\u{2019}';

    /// The bold face, the same bytes the window installs.
    const BOLD_BYTES: &[u8] = include_bytes!("../../../../assets/fonts/LiberationSans-Bold.ttf");

    fn be16(b: &[u8], at: usize) -> usize {
        usize::from(u16::from_be_bytes([b[at], b[at + 1]]))
    }

    fn be32(b: &[u8], at: usize) -> usize {
        u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]) as usize
    }

    /// WHETHER A TRUETYPE FILE MAPS `c` TO A REAL GLYPH, from its `cmap` table (formats 4 and 12).
    fn font_file_draws(font: &[u8], c: char) -> bool {
        let code = c as usize;
        let tables = be16(font, 4);
        let Some(cmap) = (0..tables).map(|i| 12 + 16 * i).find(|rec| &font[*rec..*rec + 4] == b"cmap").map(|rec| be32(font, rec + 8)) else { return false };
        (0..be16(font, cmap + 2)).map(|i| cmap + be32(font, cmap + 4 + 8 * i + 4)).any(|sub| match be16(font, sub) {
            4 => {
                let seg_x2 = be16(font, sub + 6);
                (0..seg_x2 / 2).any(|i| {
                    let end = be16(font, sub + 14 + 2 * i);
                    let start = be16(font, sub + 16 + seg_x2 + 2 * i);
                    if code < start || code > end {
                        return false;
                    }
                    let delta = be16(font, sub + 16 + 2 * seg_x2 + 2 * i);
                    let range_at = sub + 16 + 3 * seg_x2 + 2 * i;
                    let range = be16(font, range_at);
                    let glyph = if range == 0 { code + delta } else { be16(font, range_at + range + 2 * (code - start)) };
                    glyph & 0xFFFF != 0 && (range == 0 || be16(font, range_at + range + 2 * (code - start)) != 0)
                })
            }
            12 => (0..be32(font, sub + 12)).any(|g| {
                let at = sub + 16 + 12 * g;
                code >= be32(font, at) && code <= be32(font, at + 4) && be32(font, at + 8) + (code - be32(font, at)) != 0
            }),
            _ => false,
        })
    }

    /// Every character of the Ukrainian catalogue and help that is not plain ASCII.
    fn used_characters() -> std::collections::BTreeSet<char> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut out = std::collections::BTreeSet::new();
        let mut stack = vec![root.join("i18n/uk"), root.join("docs/help/uk")];
        while let Some(dir) = stack.pop() {
            for e in std::fs::read_dir(&dir).expect("the Ukrainian directory reads").flatten() {
                let p = e.path();
                if p.is_dir() {
                    stack.push(p);
                } else if p.extension().is_some_and(|x| x == "ftl" || x == "md") {
                    out.extend(std::fs::read_to_string(&p).expect("the file reads").chars().filter(|c| !c.is_ascii()));
                }
            }
        }
        out
    }

    /// The characters of `wanted` the installed fonts do not draw, named by the face that lacks them.
    fn missing(wanted: &[char]) -> Vec<String> {
        let ctx = egui::Context::default();
        super::super::install_fonts(&ctx);
        // the fonts are applied by a frame, not by the call that sets them - and by a frame later than the first
        for _ in 0..2 {
            let _ = ctx.run_ui(egui::RawInput::default(), |_| {});
        }
        let ordinary = egui::FontId::proportional(14.0);
        let mut out = Vec::new();
        for c in wanted {
            if !ctx.fonts_mut(|f| f.has_glyph(&ordinary, *c)) {
                out.push(format!("{} (U+{:04X}) in the ordinary font", c, *c as u32));
            }
            if !font_file_draws(BOLD_BYTES, *c) {
                out.push(format!("{} (U+{:04X}) in the bold font", c, *c as u32));
            }
        }
        out
    }

    /// THE READER OF THE FONT FILE IS NOT BLIND: it finds a Latin letter and refuses a character no font has.
    #[test]
    fn the_font_file_reader_tells_a_glyph_from_none() {
        assert!(font_file_draws(BOLD_BYTES, 'A') && font_file_draws(BOLD_BYTES, '\u{436}'), "the reader cannot find letters the bold face certainly holds");
        assert!(!font_file_draws(BOLD_BYTES, '\u{FDFE}'), "the reader finds a glyph for a character no face holds");
    }

    /// THE CHECK ITSELF IS NOT BLIND: a character no face holds is reported for both faces.
    #[test]
    fn a_character_no_face_holds_is_reported_for_both() {
        let bad = missing(&['\u{FDFE}']);
        assert_eq!(bad.len(), 2, "a character no face holds must be reported by the ordinary and the bold face alike: {bad:?}");
    }

    /// THE ALPHABET AND THE APOSTROPHE ARE DRAWN.
    #[test]
    fn the_ukrainian_alphabet_and_the_apostrophe_have_glyphs() {
        let wanted: Vec<char> = ALPHABET.trim().chars().chain(std::iter::once(APOSTROPHE)).collect();
        let bad = missing(&wanted);
        assert!(bad.is_empty(), "the interface fonts cannot draw ({}): they would show as boxes\n{}", bad.len(), bad.join("\n"));
    }

    /// EVERYTHING THE CATALOGUE AND THE HELP ACTUALLY USE IS DRAWN, not only the alphabet: a degree sign, a
    /// diameter sign or a typographic dash is as fatal as a missing letter.
    #[test]
    fn every_character_the_ukrainian_texts_use_has_a_glyph() {
        let used = used_characters();
        assert!(used.contains(&APOSTROPHE), "the Ukrainian texts hold no apostrophe of the agreed shape - the check is looking at the wrong files");
        assert!(!used.contains(&'\u{02BC}'), "two shapes of the apostrophe are mixed in the Ukrainian texts: write only U+2019");
        let used: Vec<char> = used.into_iter().collect();
        let bad = missing(&used);
        assert!(bad.is_empty(), "the Ukrainian texts hold characters the interface fonts cannot draw ({}):\n{}", bad.len(), bad.join("\n"));
    }
}
