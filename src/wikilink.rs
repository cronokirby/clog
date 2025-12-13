use regex::Regex;
use std::{iter, sync::LazyLock};

static RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\|\[\]]+)\|?([^\|\[\]]+)?\]\]").unwrap());

/// A Link like `[[Foo]]` in a post.
///
/// "WikiLink" is the term Obsidian uses themselves a lot.
#[derive(Clone, Debug, PartialEq)]
pub struct WikiLink<'a> {
    pub display: Option<&'a str>,
    pub name: &'a str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Segment<'a> {
    Normal(&'a str),
    Link(WikiLink<'a>),
}

impl<'a> WikiLink<'a> {
    pub fn display_or_name(&self) -> &'a str {
        self.display.unwrap_or(self.name)
    }

    /// Extract all of the links from a string.
    pub fn extract(data: &'a str) -> impl Iterator<Item = Self> {
        RE.captures_iter(data).map(|capture| {
            let name = capture.get(1).unwrap().as_str();
            let display = capture.get(2).map(|x| x.as_str());
            Self { display, name }
        })
    }

    /// Segment data into normal spans and links.
    ///
    /// Useful when generating HTML, where you want to convert wikilinks into refs.
    pub fn segment(data: &'a str) -> impl Iterator<Item = Segment<'a>> {
        let mut locs = RE.capture_locations();
        iter::successors(Some((0usize, [None, None])), move |(pos, queue)| {
            // Yield the second item in the queue, if there was one.
            match queue {
                [_, rest @ Some(_)] => return Some((*pos, [rest.clone(), None])),
                [_, None] => {}
            }
            // Our current position in the string, which we'll mutate
            let mut pos = *pos;
            let mut queue = [None, None];
            // Index into the next item in the queue. Push by incrementing.
            let mut q_i = 0;
            // We rely on the fact that this function accepts `pos = data.len()`, returning
            // none since the end of the string is reached.
            match RE.captures_read_at(&mut locs, data, pos) {
                None => {
                    // We've reached the end of the string.
                    if pos >= data.len() {
                        return None;
                    }
                    queue[q_i] = Some(Segment::Normal(&data[pos..]));
                    pos = data.len();
                }
                Some(capture) => {
                    let (c_start, c_end) = (capture.start(), capture.end());
                    // If the position isn't at the start of the capture, there's
                    // some data we need to push.
                    if pos < c_start {
                        queue[q_i] = Some(Segment::Normal(&data[pos..c_start]));
                        q_i += 1;
                    }
                    // Regardless, move the position past the end of the capture.
                    pos = c_end;
                    let link = {
                        let name = locs.get(1).map(|(s, e)| &data[s..e]).unwrap();
                        let display = locs.get(2).map(|(s, e)| &data[s..e]);
                        Self { display, name }
                    };
                    queue[q_i] = Some(Segment::Link(link));
                }
            }
            Some((pos, queue))
        })
        .filter_map(|(_, [head, _])| head)
    }
}

#[cfg(test)]
mod test {
    use crate::wikilink::{Segment, WikiLink};

    #[test]
    fn extract() {
        let data = "[[One]] [[Two|TWO]]\n[[Three|THREE]] [[Four Five]]";
        let captures = WikiLink::extract(data).collect::<Vec<_>>();
        assert_eq!(
            captures,
            vec![
                WikiLink {
                    name: "One",
                    display: None,
                },
                WikiLink {
                    name: "Two",
                    display: Some("TWO"),
                },
                WikiLink {
                    name: "Three",
                    display: Some("THREE"),
                },
                WikiLink {
                    name: "Four Five",
                    display: None,
                }
            ]
        )
    }

    #[test]
    fn segment() {
        fn work(data: &str) -> Vec<Segment<'_>> {
            WikiLink::segment(data).collect()
        }
        assert_eq!(work("nothing"), vec![Segment::Normal("nothing")]);
        assert_eq!(
            work("start [[link 1]] middle [[link 2]] end"),
            vec![
                Segment::Normal("start "),
                Segment::Link(WikiLink {
                    display: None,
                    name: "link 1"
                }),
                Segment::Normal(" middle "),
                Segment::Link(WikiLink {
                    display: None,
                    name: "link 2"
                }),
                Segment::Normal(" end"),
            ]
        );
    }
}
