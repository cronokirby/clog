use regex::Regex;
use std::sync::LazyLock;

/// A Link like `[[Foo]]` in a post.
///
/// "WikiLink" is the term Obsidian uses themselves a lot.
#[derive(Debug, PartialEq)]
struct WikiLink {
    pub display: Option<String>,
    pub name: String,
}

impl WikiLink {
    /// Extract all of the links from a string.
    pub fn extract(data: &str) -> impl Iterator<Item = Self> {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\[\[([^\|\[\]]+)\|?([^\|\[\]]+)?\]\]").unwrap());
        RE.captures_iter(data).map(|capture| {
            let name = capture[1].to_string();
            let display = capture.get(2).map(|x| x.as_str().to_string());
            Self { display, name }
        })
    }
}

#[cfg(test)]
mod test {
    use crate::wikilink::WikiLink;

    #[test]
    fn extract() {
        let data = "[[One]] [[Two|TWO]]\n[[Three|THREE]] [[Four Five]]";
        let captures = WikiLink::extract(data).collect::<Vec<_>>();
        assert_eq!(
            captures,
            vec![
                WikiLink {
                    name: "One".to_string(),
                    display: None,
                },
                WikiLink {
                    name: "Two".to_string(),
                    display: Some("TWO".to_string()),
                },
                WikiLink {
                    name: "Three".to_string(),
                    display: Some("THREE".to_string()),
                },
                WikiLink {
                    name: "Four Five".to_string(),
                    display: None,
                }
            ]
        )
    }
}
