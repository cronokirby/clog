use anyhow::anyhow;
use markdown::ParseOptions;
use markdown::mdast;
use markdown::to_mdast;
use minijinja::{Template, context};
use std::fs;
use std::io;
use std::io::BufWriter;
use std::io::Write as _;
use std::path::Path;

mod counter;
use counter::Sequential;

fn write_md_ast<'root>(writer: &mut impl io::Write, ast: &'root mdast::Node) -> anyhow::Result<()> {
    enum Work<'a> {
        Node(&'a mdast::Node),
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
        let node = match work {
            Work::Str(s) => {
                writer.write_all(s.as_bytes())?;
                continue;
            }
            Work::Lit(s) => {
                writer.write_all(s.as_bytes())?;
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
                fmt!("\n<pre>\n<code>\n{}\n</code>\n</pre>", n.value);
            }
            InlineMath(n) => {
                fmt!("<code>${}$</code>", n.value);
            }
            Math(n) => {
                fmt!("\n<pre>\n<code>\n$$\n{}\n$$\n</code>\n</pre>", n.value);
            }
            Text(n) => {
                writer.write_all(n.value.as_bytes())?;
            }
            ThematicBreak(_) => {
                lit!("\n<hr />");
            }
            Table(n) => {
                lit!("\n</table>");
                children!(n.children);
                lit!("\n<table>");
            }
            TableRow(n) => {
                lit!("\n</tr>");
                children!(n.children);
                lit!("\n<tr>");
            }
            TableCell(n) => {
                lit!("</th>");
                children!(n.children);
                lit!("\n<th>");
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
    write!(writer, "<section class=\"footnotes\">\n<ol>\n")?;
    for def in footnote_defs.into_iter() {
        match def {
            None => {
                write!(writer, "<li>???</li>\n")?;
            }
            Some((identifier, children)) => {
                write!(writer, "<li id=\"fn-{identifier}\">")?;
                for n in children {
                    write_md_ast(writer, n)?;
                }
                write!(writer, "</li>\n")?;
            }
        }
    }
    write!(writer, "</ol>\n</section>")?;
    write!(writer, "\n")?;
    Ok(())
}

/// Represents a file we process in our blog engine.
pub struct File<'a> {
    rel_path: &'a Path,
    contents: String,
}

impl<'a> File<'a> {
    /// Attempt to read a file, given a base path, and a full path.
    ///
    /// - `base` is expected to be the root of all markdown files.
    ///   For example, `content/Posts/000.md` should have `content/` as the
    ///   base, and not `content/Posts/`.
    /// - `full` is expected to be the full path to the file. e.g. `content/Posts/000.md`
    ///   continuing with the previous example.
    pub fn read(base: &Path, full: &'a Path) -> anyhow::Result<Self> {
        let rel_path = full.strip_prefix(base)?;
        let contents = fs::read_to_string(full)?;
        Ok(Self { rel_path, contents })
    }

    /// Process this file, creating an HTML file in the output path.
    ///
    /// - `out_dir` should be the root of the final site.
    /// - `template` will be used to render the markdown.
    pub fn write(self, out_dir: &Path, template: Template<'_, '_>) -> anyhow::Result<()> {
        let options = {
            let mut out = ParseOptions::gfm();
            out.constructs.frontmatter = true;
            out
        };
        let body = {
            let ast = to_mdast(&self.contents, &options)
                .map_err(|e| anyhow!("failed to parse markdown: {e}"))?;
            let mut buffer = Vec::with_capacity(1 << 14);
            write_md_ast(&mut buffer, &ast)?;
            String::from_utf8(buffer)?
        };
        let out_path = out_dir.join(self.rel_path.with_extension("html"));
        fs::create_dir_all(out_path.parent().ok_or_else(|| anyhow!("missing parent"))?)?;
        let file = fs::File::create(&out_path)?;
        let mut writer = BufWriter::new(file);
        let ctx = context! {
          body => body
        };
        template.render_to_write(ctx, &mut writer)?;
        writer.flush()?;
        Ok(())
    }
}
