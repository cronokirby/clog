use anyhow::anyhow;
use markdown::{ParseOptions, mdast, to_mdast};
use std::io;

mod counter;

use counter::Sequential;

use crate::prettify::prettify;
use crate::sitemap::SiteMap;
use crate::wikilink::{Segment, WikiLink};

#[derive(Default)]
pub struct Log {
    pub math: bool,
}

impl Log {
    pub fn merge(&mut self, other: &Self) {
        self.math |= other.math;
    }
}

pub fn make_mdast(data: &str) -> anyhow::Result<mdast::Node> {
    make_mdast_with_math(data, true)
}

pub fn make_mdast_with_math(data: &str, math: bool) -> anyhow::Result<mdast::Node> {
    let options = {
        let mut out = ParseOptions::gfm();
        out.constructs.math_text = math;
        out.constructs.math_flow = math;
        out.constructs.frontmatter = true;
        out
    };
    let ast = to_mdast(data, &options).map_err(|e| anyhow!("failed to parse markdown: {e}"))?;
    Ok(ast)
}

fn katex_settings(display_mode: bool) -> katex::Settings {
    let settings = katex::Settings {
        display_mode,
        ..Default::default()
    };
    {
        let mut macros = settings.macros.borrow_mut();
        for (name, expansion) in [
            ("\\sk", "\\mathsf{sk}"),
            ("\\ck", "\\mathsf{ck}"),
            ("\\tk", "\\mathsf{tk}"),
            ("\\Fp", "\\mathbb{F}_p"),
            ("\\poly", "\\operatorname{poly}"),
            ("\\secp", "\\lambda"),
            ("\\p", "p"),
        ] {
            macros.insert(
                name.to_owned(),
                katex::macros::MacroDefinition::String(expansion.to_owned()),
            );
        }
    }
    settings
}

fn table_cell_open_tag(header: bool, align: Option<mdast::AlignKind>) -> String {
    let tag = if header { "th" } else { "td" };
    match align {
        Some(mdast::AlignKind::Left) => format!("\n<{tag} style=\"text-align: left\">"),
        Some(mdast::AlignKind::Right) => format!("\n<{tag} style=\"text-align: right\">"),
        Some(mdast::AlignKind::Center) => format!("\n<{tag} style=\"text-align: center\">"),
        Some(mdast::AlignKind::None) | None => format!("\n<{tag}>"),
    }
}

pub fn write_md_ast<'root>(
    writer: &mut impl io::Write,
    site_map: &SiteMap,
    katex_ctx: &katex::KatexContext,
    ast: &'root mdast::Node,
) -> anyhow::Result<Log> {
    let mut log = Log::default();
    enum Work<'a> {
        Node(&'a mdast::Node),
        TableRow {
            node: &'a mdast::Node,
            header: bool,
            align: &'a [mdast::AlignKind],
        },
        TableCell {
            node: &'a mdast::Node,
            header: bool,
            align: Option<mdast::AlignKind>,
        },
        Lit(&'static str),
        Str(String),
    }

    let mut footnote_ids = Sequential::<&'root str>::default();
    let mut footnote_defs =
        Vec::<Option<(&'root str, &'root [mdast::Node])>>::with_capacity(1 << 6);

    // Work contains unprocessed nodes.
    //
    // The capacity should be the largest amount of children or nesting we expect to see.
    let mut q: Vec<Work<'_>> = Vec::with_capacity(1 << 8);
    macro_rules! children {
        ($x:expr) => {
            q.extend($x.iter().rev().map(Work::Node))
        };
    }
    macro_rules! lit {
        ($x:expr) => {
            q.push(Work::Lit($x))
        };
    }
    macro_rules! fmt {
        ($fmt:literal, $($xs:expr),*) => {
            q.push(Work::Str(
                format!($fmt $(, $xs)*)
            ))
        };
    }
    q.push(Work::Node(ast));
    while let Some(work) = q.pop() {
        let node =
            match work {
                Work::Str(s) => {
                    writer.write_all(s.as_bytes())?;
                    continue;
                }
                Work::Lit(s) => {
                    writer.write_all(s.as_bytes())?;
                    continue;
                }
                Work::TableRow {
                    node,
                    header,
                    align,
                } => {
                    let mdast::Node::TableRow(n) = node else {
                        q.push(Work::Node(node));
                        continue;
                    };
                    q.push(Work::Lit("\n</tr>"));
                    q.extend(n.children.iter().enumerate().rev().map(|(i, node)| {
                        Work::TableCell {
                            node,
                            header,
                            align: align.get(i).copied(),
                        }
                    }));
                    q.push(Work::Lit("\n<tr>"));
                    continue;
                }
                Work::TableCell {
                    node,
                    header,
                    align,
                } => {
                    let mdast::Node::TableCell(n) = node else {
                        q.push(Work::Node(node));
                        continue;
                    };
                    let tag = if header { "th" } else { "td" };
                    q.push(Work::Str(format!("</{tag}>")));
                    q.extend(n.children.iter().rev().map(Work::Node));
                    q.push(Work::Str(table_cell_open_tag(header, align)));
                    continue;
                }
                Work::Node(node) => node,
            };
        use mdast::Node::*;
        match node {
            Root(n) => {
                children!(n.children);
            }
            Paragraph(n) => {
                lit!("</p>");
                children!(n.children);
                lit!("\n<p>");
            }
            Blockquote(n) => {
                lit!("</blockquote>");
                children!(n.children);
                lit!("\n<blockquote>");
            }
            FootnoteDefinition(n) => {
                let id = footnote_ids.value(&n.identifier);
                let id_usize = id as usize;
                footnote_defs.resize(footnote_defs.len().max(id_usize + 1), None);
                footnote_defs[id_usize] = Some((n.identifier.as_str(), n.children.as_slice()));
            }
            List(n) => {
                if n.ordered {
                    lit!("</ol>");
                    children!(n.children);
                    lit!("\n<ol>");
                } else {
                    lit!("</ul>");
                    children!(n.children);
                    lit!("\n<ul>");
                }
            }
            ListItem(n) => {
                lit!("</li>");
                match (n.spread, n.children.as_slice()) {
                    (false, [Paragraph(inner)]) => {
                        children!(inner.children);
                    }
                    (_, children) => {
                        children!(children);
                    }
                }
                lit!("<li>");
            }
            Yaml(_) => {
                // Ignore front matter
            }
            Break(_) => {
                lit!("\n<br/>");
            }
            InlineCode(n) => {
                fmt!("<code>{}</code>", &n.value);
            }
            Delete(n) => {
                lit!("</del>");
                children!(n.children);
                lit!("<del>");
            }
            Emphasis(n) => {
                lit!("</em>");
                children!(n.children);
                lit!("<em>");
            }
            FootnoteReference(n) => {
                let id = footnote_ids.value(&n.identifier);
                fmt!(
                    "<sup><a href=\"#fn-{}\">{}</a></sup>",
                    &n.identifier,
                    id + 1
                );
            }
            Html(n) => {
                fmt!("{}", n.value);
            }
            Image(n) => {
                let title = n
                    .title
                    .as_ref()
                    .map(|x| format!("title={x}"))
                    .unwrap_or_default();
                fmt!("\n<img src={} alt={} {}/>", n.url, n.alt, title);
            }
            Strong(n) => {
                lit!("</strong>");
                children!(n.children);
                lit!("<strong>");
            }
            Link(n) => {
                lit!("</a>");
                children!(n.children);
                fmt!("<a href={}>", n.url);
            }
            Code(n) => {
                fmt!("\n<pre><code>{}</code></pre>", n.value);
            }
            InlineMath(n) => {
                log.math = true;
                match katex::render_to_string(katex_ctx, &n.value, &katex_settings(false)) {
                    Err(e) => {
                        eprintln!("WARN: {e}");
                        write!(writer, "<code>${}$</code>", n.value)?;
                    }
                    Ok(math) => {
                        write!(writer, "<span class=\"katex-wrapper\">{}</span>", math)?;
                    }
                }
            }
            Math(n) => {
                log.math = true;
                match katex::render_to_string(katex_ctx, &n.value, &katex_settings(true)) {
                    Err(e) => {
                        eprintln!("WARN: {e}");
                        write!(writer, "<pre><code>$${}$$</code></pre>", n.value)?;
                    }
                    Ok(math) => {
                        write!(writer, "<span class=\"katex-wrapper\">{}</span>", math)?;
                    }
                }
            }
            Text(n) => {
                let prettified = prettify(&n.value);
                for segment in WikiLink::segment(&prettified) {
                    use Segment::*;
                    match segment {
                        Normal(t) => {
                            writer.write_all(t.as_bytes())?;
                        }
                        Link(link) => match site_map.page_by_name(link.name) {
                            None => {
                                // If the reference doesn't exist, use emphasis nonetheless.
                                write!(writer, "<em>{}</em>", link.display_or_name())?;
                            }
                            Some(page) => {
                                write!(
                                    writer,
                                    "<a href=\"{}\">{}</a>",
                                    page.link,
                                    link.display_or_name()
                                )?;
                            }
                        },
                    }
                }
            }
            ThematicBreak(_) => {
                lit!("\n<hr />");
            }
            Table(n) => {
                lit!("\n</table>");
                q.extend(
                    n.children
                        .iter()
                        .enumerate()
                        .rev()
                        .map(|(i, node)| Work::TableRow {
                            node,
                            header: i == 0,
                            align: &n.align,
                        }),
                );
                lit!("\n<table>");
            }
            TableRow(n) => {
                lit!("\n</tr>");
                q.extend(n.children.iter().rev().map(|node| Work::TableCell {
                    node,
                    header: false,
                    align: None,
                }));
                lit!("\n<tr>");
            }
            TableCell(n) => {
                lit!("</td>");
                children!(n.children);
                lit!("\n<td>");
            }
            Heading(n) => {
                fmt!("</h{}>", n.depth);
                children!(n.children);
                fmt!("\n<h{}>", n.depth);
            }
            MdxJsxFlowElement(_) => unimplemented!("MdxJsxFlowElement"),
            MdxjsEsm(_) => unimplemented!("MdxjsEsm"),
            Toml(_) => unimplemented!("Toml"),
            MdxTextExpression(_) => unimplemented!("MdxTextExpression"),
            ImageReference(_) => unimplemented!("ImageReference"),
            MdxJsxTextElement(_) => unimplemented!("MdxJsxTextElement"),
            LinkReference(_) => unimplemented!("LinkReference"),
            MdxFlowExpression(_) => unimplemented!("MdxFlowExpression"),
            Definition(_) => unimplemented!("Definition"),
        }
    }
    if footnote_defs.iter().any(Option::is_some) {
        write!(writer, "<section class=\"footnotes\">\n<ol>\n")?;
        for def in footnote_defs.into_iter() {
            match def {
                None => {
                    write!(writer, "<li>???</li>\n")?;
                }
                Some((identifier, children)) => {
                    write!(writer, "<li id=\"fn-{identifier}\">")?;
                    for n in children {
                        let child_log = write_md_ast(writer, site_map, katex_ctx, n)?;
                        log.merge(&child_log);
                    }
                    write!(writer, "</li>\n")?;
                }
            }
        }
        write!(writer, "</ol>\n</section>")?;
    }
    write!(writer, "\n")?;
    Ok(log)
}

pub fn find_yaml_frontmatter<'root>(ast: &'root mdast::Node) -> Option<&'root str> {
    let mut q = vec![ast];
    while let Some(n) = q.pop() {
        use mdast::Node::*;
        match n {
            Root(n) => {
                q.extend(n.children.iter());
            }
            Yaml(n) => return Some(&n.value),
            _ => {}
        }
    }
    None
}

pub fn extract_description(ast: &mdast::Node, max_len: usize) -> String {
    let mut out = String::with_capacity(max_len + 50);
    let mut q = vec![ast];
    'outer: while let Some(n) = q.pop() {
        use mdast::Node::*;
        match n {
            Root(n) => q.extend(n.children.iter().rev()),
            Blockquote(n) => q.extend(n.children.iter().rev()),
            Paragraph(n) => q.extend(n.children.iter().rev()),
            Heading(n) => q.extend(n.children.iter().rev()),
            Emphasis(n) => q.extend(n.children.iter().rev()),
            Strong(n) => q.extend(n.children.iter().rev()),
            Delete(n) => q.extend(n.children.iter().rev()),
            Link(n) => q.extend(n.children.iter().rev()),
            List(n) => q.extend(n.children.iter().rev()),
            ListItem(n) => q.extend(n.children.iter().rev()),
            Text(n) => {
                for c in n.value.chars() {
                    if out.len() >= max_len {
                        break 'outer;
                    }
                    out.push(if c.is_whitespace() { ' ' } else { c });
                }
            }
            InlineCode(n) => {
                for c in n.value.chars() {
                    if out.len() >= max_len {
                        break 'outer;
                    }
                    out.push(if c.is_whitespace() { ' ' } else { c });
                }
            }
            _ => {}
        }
    }
    out.truncate(out.trim_end().len());
    out
}
