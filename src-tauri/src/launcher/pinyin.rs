use ::pinyin::ToPinyin;

pub fn normalize(value: &str) -> String {
    value
        .chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(char::to_lowercase)
        .collect()
}

/// Called only while indexing; typing never performs pinyin conversion.
pub fn precompute(name: &str) -> (String, String) {
    let mut full = String::new();
    let mut initials = String::new();
    let mut in_latin_word = false;
    for ch in name.chars() {
        if let Some(syllable) = ch.to_pinyin() {
            let syllable = syllable.plain();
            full.push_str(syllable);
            if let Some(first) = syllable.chars().next() {
                initials.push(first);
            }
            in_latin_word = false;
        } else if ch.is_alphanumeric() {
            for lower in ch.to_lowercase() {
                full.push(lower);
                if !in_latin_word {
                    initials.push(lower);
                }
            }
            in_latin_word = true;
        } else {
            in_latin_word = false;
        }
    }
    (full, initials)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn chinese_and_mixed_names_are_precomputed() {
        assert_eq!(precompute("微信"), ("weixin".into(), "wx".into()));
        assert_eq!(
            precompute("Visual Studio Code"),
            ("visualstudiocode".into(), "vsc".into())
        );
        assert_eq!(
            precompute("微信 Windows"),
            ("weixinwindows".into(), "wxw".into())
        );
    }
}
