pub(crate) fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|word| {
            let normalized = normalize_word(word);
            (!normalized.is_empty()).then_some(normalized)
        })
        .collect()
}

fn normalize_word(word: &str) -> String {
    word.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

pub(super) fn contains_word_sequence(words: &[String], sequence: &[String]) -> bool {
    if sequence.is_empty() {
        return true;
    }
    if sequence.len() > words.len() {
        return false;
    }

    words
        .windows(sequence.len())
        .any(|window| window == sequence)
}

pub(super) fn word_ngrams(words: &[String], size: usize) -> Vec<String> {
    if words.len() < size {
        return Vec::new();
    }

    words.windows(size).map(|window| window.join(" ")).collect()
}
