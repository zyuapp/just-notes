const SCHEME: &str = "https://";

const CONFERENCING_HOSTS: &[&str] = &[
    "zoom.us",
    "zoom.com",
    "zoomgov.com",
    "meet.google.com",
    "teams.microsoft.com",
    "teams.live.com",
    "webex.com",
    "whereby.com",
    "meet.jit.si",
    "chime.aws",
    "gotomeeting.com",
    "meet.goto.com",
    "bluejeans.com",
    "facetime.apple.com",
];

/// The first HTTPS conferencing link found across `sources`, searched in order.
pub(super) fn join_link(sources: &[Option<&str>]) -> Option<String> {
    sources
        .iter()
        .flatten()
        .find_map(|text| find_join_link(text))
}

fn find_join_link(text: &str) -> Option<String> {
    // ASCII lowercasing keeps byte offsets, so matches index the original text.
    let lowercase = text.to_ascii_lowercase();
    lowercase.match_indices(SCHEME).find_map(|(start, _)| {
        let candidate = &text[start..];
        let end = candidate
            .find(|c: char| c.is_whitespace() || matches!(c, '<' | '>' | '"' | '\'' | '`'))
            .unwrap_or(candidate.len());
        let url = candidate[..end].trim_end_matches(['.', ',', ';', ':', ')', ']']);
        is_conferencing_url(url).then(|| url.to_string())
    })
}

fn is_conferencing_url(url: &str) -> bool {
    let authority = url[SCHEME.len()..]
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default();
    if authority.contains('@') {
        return false;
    }
    let host = authority
        .split(':')
        .next()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    CONFERENCING_HOSTS.iter().any(|domain| {
        host == *domain
            || host
                .strip_suffix(domain)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

#[cfg(test)]
mod tests {
    use super::join_link;

    #[test]
    fn finds_links_in_notes_and_trims_wrapping_punctuation() {
        let notes = "Join Zoom Meeting\n<https://us02web.zoom.us/j/123?pwd=abc>.\nThanks";
        assert_eq!(
            join_link(&[None, None, Some(notes)]).as_deref(),
            Some("https://us02web.zoom.us/j/123?pwd=abc")
        );
        assert_eq!(
            join_link(&[Some("(see https://meet.google.com/abc-defg-hij).")]).as_deref(),
            Some("https://meet.google.com/abc-defg-hij")
        );
    }

    #[test]
    fn prefers_earlier_sources_and_skips_non_conferencing_links() {
        let notes =
            "Agenda: https://docs.google.com/doc then https://teams.microsoft.com/l/meetup-join/1";
        assert_eq!(
            join_link(&[None, Some("Room 4"), Some(notes)]).as_deref(),
            Some("https://teams.microsoft.com/l/meetup-join/1")
        );
        assert_eq!(
            join_link(&[
                Some("https://meet.google.com/first"),
                None,
                Some("https://zoom.us/j/second"),
            ])
            .as_deref(),
            Some("https://meet.google.com/first")
        );
    }

    #[test]
    fn matches_scheme_and_host_case_insensitively() {
        assert_eq!(
            join_link(&[Some("HTTPS://Company.Zoom.US/j/9")]).as_deref(),
            Some("HTTPS://Company.Zoom.US/j/9")
        );
    }

    #[test]
    fn rejects_lookalike_hosts_credentials_and_plain_http() {
        for text in [
            "https://notzoom.us/j/1",
            "https://zoom.us.example.com/j/1",
            "https://meet.google.com@evil.example/abc",
            "http://zoom.us/j/1",
            "zoommtg://zoom.us/join?confno=1",
        ] {
            assert_eq!(join_link(&[Some(text)]), None, "{text}");
        }
    }
}
