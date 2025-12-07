use anyhow::anyhow;
use markdown::to_html;
use minijinja::{Template, context};
use std::fs;
use std::io::{BufWriter, Write as _};
use std::path::Path;

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
        let body = to_html(&self.contents);
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
