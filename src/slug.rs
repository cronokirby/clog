use std::path::{Path, PathBuf};

use unidecode::unidecode_char;

/// Convert a string into a URL-safe slug.
///
/// This function:
/// - Converts to lowercase
/// - Replaces spaces and underscores with hyphens
/// - Removes characters that aren't alphanumeric, hyphens, or periods
/// - Collapses consecutive hyphens into one
/// - Trims leading/trailing hyphens from each path segment
///
/// # Examples
///
/// ```
/// assert_eq!(slugify("Hello World"), "hello-world");
/// assert_eq!(slugify("What's Up?"), "whats-up");
/// assert_eq!(slugify("foo--bar"), "foo-bar");
/// ```
pub fn slugify(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut prev_was_hyphen = true; // Start true to trim leading hyphens

    for c in s.chars() {
        for c in unidecode_char(c).chars() {
            match c {
                // Keep alphanumeric characters (lowercased)
                c if c.is_ascii_alphanumeric() => {
                    result.push(c.to_ascii_lowercase());
                    prev_was_hyphen = false;
                }
                // Keep periods (for file extensions)
                '.' => {
                    result.push('.');
                    prev_was_hyphen = false;
                }
                // Convert spaces and underscores to hyphens
                ' ' | '_' => {
                    if !prev_was_hyphen {
                        result.push('-');
                        prev_was_hyphen = true;
                    }
                }
                // Keep existing hyphens (but collapse consecutive ones)
                '-' => {
                    if !prev_was_hyphen {
                        result.push('-');
                        prev_was_hyphen = true;
                    }
                }
                // Remove all other characters (apostrophes, quotes, colons, etc.)
                _ => {}
            }
        }
    }

    // Trim trailing hyphen
    if result.ends_with('-') {
        result.pop();
    }

    result
}

/// Slugify each component of a path.
///
/// This preserves the path structure but slugifies each segment.
pub fn slugify_path(path: &Path) -> PathBuf {
    path.iter()
        .map(|component| {
            let s = component.to_string_lossy();
            slugify(&s)
        })
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn basic_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("hello world"), "hello-world");
        assert_eq!(slugify("HELLO WORLD"), "hello-world");
    }

    #[test]
    fn special_characters() {
        assert_eq!(slugify("What's Up?"), "whats-up");
        assert_eq!(slugify("Hello: World!"), "hello-world");
        assert_eq!(slugify("foo \"bar\" baz"), "foo-bar-baz");
        assert_eq!(slugify("test@example#hash"), "testexamplehash");
    }

    #[test]
    fn underscores_and_hyphens() {
        assert_eq!(slugify("foo_bar"), "foo-bar");
        assert_eq!(slugify("foo-bar"), "foo-bar");
        assert_eq!(slugify("foo--bar"), "foo-bar");
        assert_eq!(slugify("foo___bar"), "foo-bar");
        assert_eq!(slugify("foo - bar"), "foo-bar");
    }

    #[test]
    fn leading_trailing() {
        assert_eq!(slugify(" hello "), "hello");
        assert_eq!(slugify("-hello-"), "hello");
        assert_eq!(slugify("--hello--"), "hello");
    }

    #[test]
    fn preserves_periods() {
        assert_eq!(slugify("file.html"), "file.html");
        assert_eq!(slugify("My File.html"), "my-file.html");
    }

    #[test]
    fn numbers() {
        assert_eq!(slugify("Post 123"), "post-123");
        assert_eq!(slugify("2024-01-15"), "2024-01-15");
    }

    #[test]
    fn path_slugify() {
        let path = Path::new("Posts/My Cool Post.html");
        assert_eq!(slugify_path(path), PathBuf::from("posts/my-cool-post.html"));

        let path = Path::new("Category Name/Sub Category/File Name.html");
        assert_eq!(
            slugify_path(path),
            PathBuf::from("category-name/sub-category/file-name.html")
        );
    }
}
