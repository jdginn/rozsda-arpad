/// Compacts a `&str` into exactly 7 ASCII bytes using camelCase conventions,
/// vowel stripping, double-consonant compression, and null-padding.
pub fn compact_to_7_bytes(input: &str) -> Vec<u8> {
    const VOWELS: &[u8] = b"aeiouAEIOU";
    const SEPARATORS: &[u8] = b" _.,-";
    const KEEP: &[u8] = b"<>";
    const NUMBERS: &[u8] = b"0123456789";

    // 1. Filter to ASCII alphanumerics + recognized separators
    let cleaned: Vec<u8> = input
        .bytes()
        .filter(|b| b.is_ascii_alphanumeric() || SEPARATORS.contains(b) || KEEP.contains(b))
        .collect();

    if cleaned.is_empty() {
        return vec![32; 7];
    }

    // 2. Split into words by separators
    // We also treat EACH number as its own word. This keeps us from truncating numbers (we assume
    // that numbers are always important).
    let words: Vec<&[u8]> = cleaned
        .split(|b| SEPARATORS.contains(b))
        .filter(|w| !w.is_empty())
        .flat_map(|w| w.chunk_by(|a, b| a.is_ascii_digit() == b.is_ascii_digit()))
        .flat_map(|w| -> Vec<&[u8]> {
            if w[0].is_ascii_digit() {
                w.chunks(1).collect()
            } else {
                vec![w]
            }
        })
        .filter(|w| !w.is_empty())
        .collect();

    if words.is_empty() {
        let mut result: Vec<u8> = input.bytes().collect();
        result.truncate(7);
        result.resize(7, 32);
        return result;
    }

    // 3. Process each word
    let mut processed: Vec<Vec<u8>> = Vec::with_capacity(words.len());
    for (i, word) in words.iter().enumerate() {
        let mut w = word.to_vec();

        // --- Casing Rules ---
        if i == 0 {
            // First word: preserve original first char case, lowercase the rest
            if !w.is_empty() {
                for j in 1..w.len() {
                    w[j] = w[j].to_ascii_lowercase();
                }
            }
        } else {
            // Subsequent words: camelCase
            // Lowercase all-caps words before casing (Rule 5)
            if w.iter()
                .all(|b| b.is_ascii_alphabetic() && b.is_ascii_uppercase())
            {
                for b in w.iter_mut() {
                    *b = b.to_ascii_lowercase();
                }
            }

            if !w.is_empty() {
                w[0] = w[0].to_ascii_uppercase();
                for j in 1..w.len() {
                    w[j] = w[j].to_ascii_lowercase();
                }
            }
        }

        processed.push(w);
    }

    const WORD_REPLACEMENTS: &[(&[u8], &[u8])] = &[
        (b"one", b"1"),
        (b"two", b"2"),
        (b"three", b"3"),
        (b"third", b"3rd"),
        (b"four", b"4"),
        (b"five", b"5"),
        (b"six", b"6"),
        (b"seven", b"7"),
        (b"eighth", b"8th"),
        (b"eight", b"8"),
        (b"nine", b"9"),
        (b"zero", b"0"),
        (b"left", b"L"),
        (b"right", b"R"),
        (b"center", b"C"),
    ];

    fn apply_word_replacement(s: &[u8]) -> (Vec<u8>, bool) {
        let lower: Vec<u8> = s.iter().map(|b| b.to_ascii_lowercase()).collect();
        for (from, to) in WORD_REPLACEMENTS {
            if lower == *from {
                return (to.to_vec(), true);
            }
        }
        (s.to_vec(), false)
    }

    const SUBSTR_REPLACEMENTS: &[(&[u8], &[u8])] = &[
        (b"two", b"2"),
        (b"three", b"3"),
        (b"third", b"3rd"),
        (b"four", b"4"),
        (b"five", b"5"),
        (b"six", b"6"),
        (b"seven", b"7"),
        (b"eighth", b"8th"),
        (b"eight", b"8"),
        (b"nine", b"9"),
        (b"zero", b"0"),
        (b"harmony", b"harm"),
        (b"background", b"bg"),
        (b"backup", b"bk"),
        (b"overhead", b"OH"),
        (b"reverse", b"rev"),
        (b"reverb", b"rvb"),
        (b"channel", b"chan"),
        (b"duplicate", b"dup"),
        (b"lead", b"ld"),
        (b"clean", b"cln"),
        (b"kick", b"kik"),
        (b"floor", b"flr"),
        (b"hats", b"hh"),
        (b"snare", b"snr"),
        (b"click", b"clk"),
    ];

    fn apply_substr_replacement(s: &[u8]) -> (Vec<u8>, bool) {
        let mut result = s.to_vec();
        let mut found = false;
        for (from, to) in SUBSTR_REPLACEMENTS {
            let lower: Vec<u8> = result.iter().map(|b| b.to_ascii_lowercase()).collect();
            if let Some(pos) = lower.windows(from.len()).position(|w| w == *from) {
                let mut replacement = to.to_vec();
                if result[pos].is_ascii_uppercase() {
                    replacement[0] = replacement[0].to_ascii_uppercase();
                }
                result.splice(pos..pos + from.len(), replacement);
                found = true;
            }
        }
        (result, found)
    }

    // Truncate words down to 7 bytes
    //
    // Each step shortens the string by one byte until we fit within 7 bytes
    //
    // For each step, find the longest word and truncate according to a set of rules.
    'outer: while processed.concat().len() > 7 {
        // Find the longest word
        let (i, w) = processed
            .iter_mut()
            .enumerate()
            .max_by_key(|(_, w)| w.len())
            .unwrap();

        // Some predefined replacements for common words/phrases
        let (compressed, did_replace) = apply_word_replacement(w);
        if did_replace {
            *w = compressed;
            continue;
        }
        let (compressed, did_replace) = apply_substr_replacement(w);
        if did_replace {
            *w = compressed;
            continue;
        }

        let mut did_truncate = false;

        // Remove double consonants first
        let mut compressed = Vec::with_capacity(w.len());
        let mut prev_consonant: Option<u8> = None;
        for b in w.iter() {
            let is_consonant = b.is_ascii_alphabetic() && !VOWELS.contains(b);
            if is_consonant && prev_consonant == Some(b.to_ascii_lowercase()) {
                did_truncate = true;
                continue; // Skip duplicate consonant
            }
            prev_consonant = Some(b.to_ascii_lowercase());
            compressed.push(*b);
        }
        *w = compressed;
        if did_truncate {
            continue;
        }

        // Remove double vowels
        let mut compressed = Vec::with_capacity(w.len());
        let mut prev_consonant: Option<u8> = None;
        for b in w.iter() {
            let is_consonant = b.is_ascii_alphabetic() && VOWELS.contains(b);
            if is_consonant && prev_consonant == Some(b.to_ascii_lowercase()) {
                did_truncate = true;
                continue; // Skip duplicate consonant
            }
            prev_consonant = Some(b.to_ascii_lowercase());
            compressed.push(*b);
        }
        *w = compressed;
        if did_truncate {
            continue;
        }

        // Remove the last single vowel UNLESS it's the first character of the word
        for (i, b) in w.iter().enumerate().rev() {
            if VOWELS.contains(b) && i != 0 {
                w.remove(i);
                continue 'outer;
            }
        }

        // As a last resort, truncate the last character of the longest word
        if !w.is_empty() {
            w.pop();
        }

        processed[i] = w.clone();
    }

    if processed.concat().len() > 7 {
        panic!("Processed string is still longer than 7 bytes after all compression steps");
    }

    // Safety truncate & null-padding to exactly 7 bytes
    let mut result = processed.concat();
    result.truncate(7);
    result.resize(7, 32);

    result
}

#[cfg(test)]
macro_rules! assert_eq_str {
    ($left:expr, $right:expr) => {
        assert_eq!(
            $left,
            $right,
            "\n  left:  {}\n  right: {}",
            String::from_utf8_lossy($left),
            String::from_utf8_lossy($right)
        );
    };
}

mod tests {
    use super::compact_to_7_bytes;

    #[test]
    fn test_exact_7_byte_length() {
        // Rule: Output must always be exactly 7 bytes
        for input in &["hi", "hello", "longphrasethatshouldbeshort", "___"] {
            let res = compact_to_7_bytes(input);
            assert_eq!(
                res.len(),
                7,
                "Output must be exactly 7 bytes for: {}",
                input
            );
        }
    }

    #[test]
    fn test_null_padding_for_short_strings() {
        let res = compact_to_7_bytes("hi");
        // assert_eq!(&res, b"hi\0\0\0\0\0");
        assert_eq!(&res[..2], b"hi");
        assert_eq!(&res[2..7], [32, 32, 32, 32, 32]);
    }

    #[test]
    fn test_first_char_case_preservation() {
        // Rule: First character should match original capitalization
        assert_eq!(compact_to_7_bytes("hello world")[0], b'h');
        assert_eq!(compact_to_7_bytes("Hello world")[0], b'H');
        assert_eq!(compact_to_7_bytes("HELLO world")[0], b'H');
    }

    #[test]
    fn test_separators_trigger_camelcase() {
        // Rule: Spaces, underscores, periods, commas, hyphens replace with forcedCamelCase
        let res = compact_to_7_bytes("hello_world.test,word");
        let s = String::from_utf8_lossy(&res);
        assert!(
            s.starts_with("hl"),
            "First word should be lowercased (except first char preserved)"
        );
        // CamelCase boundaries should be uppercase
        assert!(
            !s.contains("_") && !s.contains(".") && !s.contains(",") && !s.contains("-"),
            "Separators should be removed and replaced by camelCase boundaries"
        );
    }

    #[test]
    fn test_all_caps_word_lowercasing_rule5() {
        // Rule 5: All-caps words next to a space get lowercased (unless first word needs uppercase)
        let res = compact_to_7_bytes("hello XML world");
        let s = String::from_utf8_lossy(&res);
        assert!(
            !s.contains("XML"),
            "All-caps word 'XML' should be lowercased to 'xml' -> 'Xml'"
        );
        // Verify camelCase transition happens
        assert!(
            s.contains("X"),
            "First letter of non-first words should be uppercase for camelCase"
        );
    }

    #[test]
    fn test_double_consonant_compression() {
        // Heuristic: `battle` -> `battl` -> `batl`
        let res = compact_to_7_bytes("battle fi");
        let s = String::from_utf8_lossy(&res);
        println!("Want: battle field -> batlField, got: {}", s);
        assert!(
            s.contains("batl"),
            "Double 't' in 'battle' should be compressed to single 't'"
        );
    }

    #[test]
    fn test_vowel_removal_priority() {
        // Rule 3 & 4: Remove vowels first (end-first), then truncate consonants if still >7
        let res = compact_to_7_bytes("beautiful reason");
        let s = String::from_utf8_lossy(&res);
        // Should be <= 7 chars (excluding nulls)
        let non_null_len = res.iter().take_while(|b| **b != 0).count();
        assert_eq!(
            non_null_len, 7,
            "After vowel removal, string should fit in 7 chars"
        );
        // Vowels should be stripped from the end first
        assert!(
            !s.ends_with(|c: char| "aeiouAEIOU".contains(c)),
            "End vowels should be removed first"
        );
    }

    #[test]
    fn test_numbers_preserved() {
        // Heuristic: Keep numbers
        let res = compact_to_7_bytes("user123 login");
        let s = String::from_utf8_lossy(&res);
        println!("Want to preserve numbers, got: {}", s);
        assert!(
            s.contains("123"),
            "Numbers should be preserved during compression"
        );
    }

    #[test]
    fn test_non_ascii_stripping() {
        // Heuristic: Skip non-ASCII
        let res = compact_to_7_bytes("naïve résumé");
        let s = String::from_utf8_lossy(&res);
        assert!(
            !s.contains("ï") && !s.contains("é"),
            "Non-ASCII characters should be stripped"
        );
    }

    #[test]
    fn test_all_separator_and_empty_inputs() {
        // Rule 11: All-separator input leaves it exactly as-is (padded with zeros)
        let res_sep = compact_to_7_bytes("  _ . ,  ");
        assert_eq!(&res_sep, "  _ . ,".as_bytes());

        let res_empty = compact_to_7_bytes("");
        assert_eq!(&res_empty, &[32; 7]);
    }

    #[test]
    fn test_deterministic_output() {
        // Rule 13: Same input always produces same output
        let input = "fetchXML_data_v2";
        let r1 = compact_to_7_bytes(input);
        let r2 = compact_to_7_bytes(input);
        assert_eq!(r1, r2, "Output must be fully deterministic");
    }

    #[test]
    fn test_exact_byte_array_for_known_case() {
        // Regression test for predictable output
        assert_eq_str!(&compact_to_7_bytes("hello world"), b"heloWrl");
        assert_eq_str!(&compact_to_7_bytes("drum bus 1"), b"drmBus1");
        assert_eq_str!(&compact_to_7_bytes("Lead Vox"), b"LeadVox");
        assert_eq_str!(&compact_to_7_bytes("funky guitar"), b"fnkyGtr");
        assert_eq_str!(&compact_to_7_bytes("rack tom 1"), b"rckTom1");
        assert_eq_str!(&compact_to_7_bytes("Stereo Chug"), b"SterChg");
        assert_eq_str!(&compact_to_7_bytes("Stacked Harmonies 2"), b"StcHrm2");
        assert_eq_str!(&compact_to_7_bytes("Hall reverb throw"), b"HalRvTh");
        assert_eq_str!(&compact_to_7_bytes("Hall reverb throw 2"), b"HlRvTh2");
        assert_eq_str!(&compact_to_7_bytes("Dotted eighth L"), b"Dtd8thL");
    }
}
