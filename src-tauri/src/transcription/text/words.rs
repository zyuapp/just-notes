pub(super) struct TranscriptWord {
    pub(super) original: String,
    pub(super) normalized: String,
}

pub(super) fn transcript_words(text: &str) -> Vec<TranscriptWord> {
    text.split_whitespace()
        .filter_map(|word| {
            let normalized = normalize_word(word);
            (!normalized.is_empty()).then(|| TranscriptWord {
                original: word.to_string(),
                normalized,
            })
        })
        .collect()
}

pub(super) fn normalized_words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .filter_map(|word| {
            let normalized = normalize_word(word);
            (!normalized.is_empty()).then_some(normalized)
        })
        .collect()
}

pub(super) fn normalize_word(word: &str) -> String {
    word.chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

pub(super) fn sentence_ends_after(word: &str) -> bool {
    word.ends_with('.') || word.ends_with('!') || word.ends_with('?')
}

pub(super) fn join_original_words(words: &[TranscriptWord]) -> String {
    words
        .iter()
        .map(|word| word.original.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn join_original_word_refs(words: &[&TranscriptWord]) -> String {
    words
        .iter()
        .map(|word| word.original.as_str())
        .collect::<Vec<_>>()
        .join(" ")
}

pub(super) fn remove_internal_repeated_sentences(text: &str, min_words_to_dedupe: usize) -> String {
    let sentences = split_transcript_sentences(text);
    let mut seen = Vec::<Vec<String>>::new();
    let mut kept = Vec::new();

    for sentence in sentences {
        let normalized = normalized_words(&sentence);
        if normalized
            .iter()
            .all(|word| word.chars().all(|character| character.is_ascii_digit()))
        {
            continue;
        }
        if normalized.len() >= min_words_to_dedupe && seen.contains(&normalized) {
            continue;
        }
        if !normalized.is_empty() {
            seen.push(normalized);
        }
        kept.push(sentence);
    }

    kept.join(" ")
}

pub(super) fn split_transcript_sentences(text: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut start = 0usize;

    for (index, character) in text.char_indices() {
        if character == '.' || character == '!' || character == '?' {
            let end = index + character.len_utf8();
            let sentence = text[start..end].trim();
            if !sentence.is_empty() {
                sentences.push(sentence.to_string());
            }
            start = end;
        }
    }

    let tail = text[start..].trim();
    if !tail.is_empty() {
        sentences.push(tail.to_string());
    }

    sentences
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
