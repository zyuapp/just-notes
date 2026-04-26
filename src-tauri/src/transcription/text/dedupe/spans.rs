pub(super) struct WordSpan {
    pub(super) start: usize,
    pub(super) end: usize,
}

pub(super) fn longest_common_word_span(
    candidate_words: &[String],
    recent_words: &[String],
    keep: &[bool],
    min_trim_words: usize,
) -> Option<WordSpan> {
    let mut best = None;
    for start in 0..candidate_words.len() {
        if !keep[start] {
            continue;
        }

        for recent_start in 0..recent_words.len() {
            let length =
                matching_word_count(candidate_words, recent_words, keep, start, recent_start);
            if length >= min_trim_words && should_replace_span(&best, start, length) {
                best = Some(WordSpan {
                    start,
                    end: start + length,
                });
            }
        }
    }

    best
}

fn matching_word_count(
    candidate_words: &[String],
    recent_words: &[String],
    keep: &[bool],
    start: usize,
    recent_start: usize,
) -> usize {
    let mut length = 0usize;
    while start + length < candidate_words.len()
        && recent_start + length < recent_words.len()
        && keep[start + length]
        && candidate_words[start + length] == recent_words[recent_start + length]
    {
        length += 1;
    }
    length
}

fn should_replace_span(best: &Option<WordSpan>, start: usize, length: usize) -> bool {
    best.as_ref()
        .map(|span| length > span.end - span.start)
        .unwrap_or(true)
        && length > 0
        && start + length > start
}
