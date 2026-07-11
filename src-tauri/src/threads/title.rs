pub(super) fn validated_title(title: &str) -> Result<&str, String> {
    let title = title.trim();
    if title.is_empty() {
        return Err("Thread title cannot be empty".to_string());
    }
    if title.chars().count() > 120 {
        return Err("Thread title must stay under 120 characters".to_string());
    }
    Ok(title)
}

pub(super) fn normalized_external_title(title: &str) -> String {
    let title = title.trim().chars().take(120).collect::<String>();
    if title.is_empty() {
        "Untitled thread".to_string()
    } else {
        title
    }
}

#[cfg(test)]
mod tests {
    use super::validated_title;

    #[test]
    fn trims_and_validates_titles() {
        assert_eq!(
            validated_title("  Design review  ").unwrap(),
            "Design review"
        );
        assert!(validated_title("   ").is_err());
        assert!(validated_title(&"x".repeat(121)).is_err());
    }

    #[test]
    fn external_titles_are_trimmed_and_bounded() {
        let title = super::normalized_external_title(&format!("  {}  ", "x".repeat(140)));
        assert_eq!(title.chars().count(), 120);
        assert_eq!(super::normalized_external_title("  "), "Untitled thread");
    }
}
