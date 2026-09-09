pub(crate) fn parse(text: &str) -> Vec<char> {
    let mut characters = text
        .chars()
        .filter(|character| !matches!(character, '\r' | '\n'))
        .collect::<Vec<_>>();
    characters.sort_unstable();
    characters.dedup();
    characters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_space_but_discards_file_line_endings() {
        assert_eq!(parse("ก ข\r\nข"), vec![' ', 'ก', 'ข']);
    }
}
