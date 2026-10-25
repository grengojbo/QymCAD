//! THE WORDS OF THE NUMBERS OF A FEATURE.
//!
//! A feature stores its numbers under keys (`height`, `dx2`, `diameter.2`): the keys a formula and a driver name, the
//! same for every language. A person reads them under words; the words are picked here, one entry of the catalogue for
//! each key, the number of a direction of a pattern and of a group made exact said after the word.

/// THE WORDS FOR THE NUMBER `key` of a feature, in the language of the person; the key itself for one the catalogue
/// has no words for.
pub fn dim_word(key: &str) -> String {
    // a group made exact past the first: `diameter.2`
    if let Some((base, n)) = key.rsplit_once('.').filter(|(_, n)| n.parse::<u32>().is_ok()) {
        return crate::tr2("fnum-group", "word", &dim_word(base), "n", n);
    }
    // the second and third directions of a pattern: `dx2`, `count3`, `step2`, `cstep3`
    for base in ["dx", "dy", "dz", "count", "step", "cstep"] {
        if let Some(n) = key.strip_prefix(base).filter(|n| matches!(*n, "2" | "3")) {
            return crate::tr2("fnum-direction", "word", &dim_word(base), "n", n);
        }
    }
    let entry = match key {
        "height" | "h" => "fnum-height",
        "down" => "fnum-down",
        "angle" | "cangle" => "fnum-angle",
        "dx" => "fnum-dx",
        "dy" => "fnum-dy",
        "dz" => "fnum-dz",
        "r" | "radius" => "fnum-radius",
        "r1" => "fnum-r1",
        "r2" => "fnum-r2",
        "major" => "fnum-major",
        "minor" => "fnum-minor",
        "n" => "fnum-sides",
        "dist" | "gap" => "fnum-dist",
        "d2" => "fnum-d2",
        "rim" => "fnum-rim",
        "base_rim" => "fnum-base-rim",
        "thickness" => "fnum-thickness",
        "tol" => "fnum-tol",
        "offset" => "fnum-offset",
        "count" => "fnum-count",
        "step" | "cstep" => "fnum-step",
        "diameter" => "fnum-diameter",
        "depth" => "fnum-depth",
        "dia2" => "fnum-dia2",
        "depth2" => "fnum-depth2",
        "off_u" => "fnum-off-u",
        "off_v" => "fnum-off-v",
        "length" => "fnum-length",
        "lead_in" => "fnum-lead-in",
        "lead_out" => "fnum-lead-out",
        "nominal" => "fnum-nominal",
        "pitch" => "fnum-pitch",
        "fit" => "fnum-fit",
        "crest_r" => "fnum-crest-r",
        "root_r" => "fnum-root-r",
        "starts" => "fnum-starts",
        "outer" => "fnum-outer",
        "edge_r" => "fnum-edge-r",
        "sharp" => "fnum-sharp",
        "simplify" => "fnum-simplify",
        _ => return key.to_string(),
    };
    crate::tr(entry)
}

#[cfg(test)]
mod tests {
    use super::dim_word;

    /// EVERY KEY A FEATURE STORES A NUMBER UNDER HAS WORDS, in every language of the catalogue: the keys of
    /// `FeatureKind::dims`, of a pattern of parts and of the features made exact.
    #[test]
    fn every_key_of_a_feature_has_words() {
        let keys = [
            "height", "down", "angle", "dx", "dy", "dz", "r", "h", "r1", "r2", "major", "minor", "n", "radius", "dist", "d2", "gap", "rim", "base_rim", "thickness", "tol", "offset", "count", "step",
            "cstep", "cangle", "diameter", "depth", "dia2", "depth2", "off_u", "off_v", "length", "lead_in", "lead_out", "nominal", "pitch", "fit", "crest_r", "root_r", "starts", "outer", "edge_r",
            "sharp", "simplify", "dx2", "dz3", "count2", "step3", "cstep2", "diameter.2", "dist.3",
        ];
        for (lang, _) in crate::available() {
            crate::set_language(&lang);
            for key in keys {
                let word = dim_word(key);
                assert!(word != key && !word.contains("fnum") && !word.is_empty(), "{lang}: the number {key} has no words: {word:?}");
            }
        }
        crate::set_language("en");
    }

    /// The number of a group or a direction is said after the word of its kind.
    #[test]
    fn a_group_and_a_direction_carry_their_number() {
        crate::set_language("en");
        assert_eq!(dim_word("diameter.2"), crate::tr2("fnum-group", "word", &crate::tr("fnum-diameter"), "n", "2"));
        assert_eq!(dim_word("dx3"), crate::tr2("fnum-direction", "word", &crate::tr("fnum-dx"), "n", "3"));
        assert_eq!(dim_word("r2"), crate::tr("fnum-r2"), "the top radius of a cone is no second direction");
    }
}
