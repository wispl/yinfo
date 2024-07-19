/// Return the substring between the two patterns in the hay.
pub fn between<'a>(hay: &'a str, start_pattern: &'a str, end_pattern: &'a str) -> &'a str {
    let start = hay.find(start_pattern);
    if start.is_some() {
        let start_pos = start.unwrap() + start_pattern.len();
        let substr = &hay[start_pos..];
        let end_pos = substr.find(end_pattern).unwrap_or_default();
        return &substr[..end_pos];
    }
    // TODO: option instead?
    ""
}
