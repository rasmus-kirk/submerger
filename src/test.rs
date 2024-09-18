#[cfg(test)]
mod tests {
    use crate::get_sub_path_regex;
    use regex::Regex;

    #[test]
    fn test_get_sub_regex() {
        // Test case 1: Basic test for 'en' and 'ja' with both srt and vtt files.
        let regex_str = get_sub_path_regex(&"en".to_string(), &"ja".to_string(), true);
        let subtitle_pattern = Regex::new(&regex_str).unwrap();

        let test_cases = vec![
            // Matching cases (correctly formatted filenames)
            ("movie.en.srt", Some("en"), false, "srt"),
            ("movie.ja.srt", Some("ja"), false, "srt"),
            ("movie.en.vtt", Some("en"), false, "vtt"),
            ("movie.ja.vtt", Some("ja"), false, "vtt"),
            ("song.ja.hi.vtt", Some("ja"), true, "vtt"),
            ("song.en.hi.srt", Some("en"), true, "srt"),
            // Non-matching cases (invalid formats)
            ("movie.de.srt", None, false, ""),
            ("movie.srt", None, false, ""),
            ("movie.ja.txt", None, false, ""),
            ("movie.enhi.vtt", None, false, ""), // Missing dot for 'hi'
            ("movie.en.hisrt", None, false, ""), // Missing dot between hi and srt
            ("movie..en.srt", None, false, ""),
        ];

        for (filename, expected_lang, expected_hi, expected_ext) in test_cases {
            let result = subtitle_pattern.captures(filename);
            match result {
                Some(captures) => {
                    let lang = captures.name("lang").map(|m| m.as_str());
                    let hearing = captures.name("hearing").is_some();
                    let ext = captures.name("ext").map(|m| m.as_str());

                    assert_eq!(lang, expected_lang, "Failed on lang for: {}", filename);
                    assert_eq!(hearing, expected_hi, "Failed on hi for: {}", filename);
                    assert_eq!(ext, Some(expected_ext), "Failed on ext for: {}", filename);
                }
                None => {
                    assert_eq!(expected_lang, None, "Unexpected match for: {}", filename);
                }
            }
        }
    }

    #[test]
    fn test_get_regex_no_vtt() {
        // Test case 2: Test where only srt files should match, not vtt.
        let regex_str = get_sub_path_regex(&"en".to_string(), &"ja".to_string(), false);
        let subtitle_pattern = Regex::new(&regex_str).unwrap();

        let test_cases = vec![
            // Matching cases (correctly formatted filenames)
            ("movie.en.srt", Some("en"), false, "srt"),
            ("movie.ja.srt", Some("ja"), false, "srt"),
            ("song.ja.hi.srt", Some("ja"), true, "srt"),
            ("song.en.hi.srt", Some("en"), true, "srt"),
            // Non-matching cases (vtt should not match)
            ("movie.en.vtt", None, false, ""),
            ("movie.ja.vtt", None, false, ""),
        ];

        for (filename, expected_lang, expected_hi, expected_ext) in test_cases {
            let result = subtitle_pattern.captures(filename);
            match result {
                Some(captures) => {
                    let lang = captures.name("lang").map(|m| m.as_str());
                    let hearing = captures.name("hearing").is_some();
                    let ext = captures.name("ext").map(|m| m.as_str());

                    assert_eq!(lang, expected_lang, "Failed on lang for: {}", filename);
                    assert_eq!(hearing, expected_hi, "Failed on hi for: {}", filename);
                    assert_eq!(ext, Some(expected_ext), "Failed on ext for: {}", filename);
                }
                None => {
                    assert_eq!(expected_lang, None, "Unexpected match for: {}", filename);
                }
            }
        }
    }
}
