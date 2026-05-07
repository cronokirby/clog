/// Prettify text by replacing certain patterns with proper Unicode characters.
pub fn prettify(text: &str) -> String {
    text.replace("---", "—")
}
