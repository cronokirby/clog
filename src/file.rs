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

use crate::frontmatter::FrontMatter;

/// Represents a file we process in our blog engine.
pub struct File<'a> {
    rel_path: &'a Path,
    frontmatter: FrontMatter,
    ast: mdast::Node,
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
        let ast = make_mdast(&contents)?;
        let frontmatter = {
            let yaml = find_yaml_frontmatter(&ast);
            FrontMatter::try_from_yaml(full, yaml)?
        };
        Ok(Self {
            rel_path,
            frontmatter,
            ast,
        })
    }

    /// Process this file, creating an HTML file in the output path.
    ///
    /// - `out_dir` should be the root of the final site.
    /// - `template` will be used to render the markdown.
    pub fn write(self, out_dir: &Path, template: Template<'_, '_>) -> anyhow::Result<()> {
        if self.frontmatter.draft {
            return Ok(());
        }
        let out_path = out_dir.join(self.rel_path.with_extension("html"));
        fs::create_dir_all(out_path.parent().ok_or_else(|| anyhow!("missing parent"))?)?;
        let file = fs::File::create(&out_path)?;
        let mut writer = BufWriter::new(file);
        let body = {
            let mut buffer = Vec::with_capacity(1 << 14);
            write_md_ast(&mut buffer, &self.ast)?;
            String::from_utf8(buffer)?
        };
        let ctx = context! {
          body => body,
          title => self.frontmatter.title,
          date => self.frontmatter.date,
          authors => self.frontmatter.authors,
          published => self.frontmatter.published,
          link => self.frontmatter.link,
          tags => self.frontmatter.tags,
        };
        template.render_to_write(ctx, &mut writer)?;
        writer.flush()?;
        Ok(())
    }
}
