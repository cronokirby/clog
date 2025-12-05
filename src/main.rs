use anyhow::{Context, anyhow};
use markdown::to_html;
use minijinja::{Environment, Template, context};
use std::{
    fs::{self},
    io::{BufWriter, Write},
    path::{Path, PathBuf},
};

mod fs_utils;
use fs_utils::copy_dir;

/// A static string for usage errors.
const USAGE: &str = "usage: clog <input_dir> <output_dir>";

struct File<'a> {
    rel_path: &'a Path,
    contents: String,
}

impl<'a> File<'a> {
    pub fn read(base: &'a Path, full: &'a Path) -> anyhow::Result<Self> {
        let rel_path = full.strip_prefix(base)?;
        let contents = fs::read_to_string(full)?;
        Ok(Self { rel_path, contents })
    }

    pub fn write(self, out_dir: &Path, template: Template<'_, '_>) -> anyhow::Result<()> {
        let body = to_html(&self.contents);
        let file = fs::File::create(&out_dir.join(self.rel_path.with_extension("html")))?;
        let mut writer = BufWriter::new(file);
        let ctx = context! {
          body => body
        };
        template.render_to_write(ctx, &mut writer)?;
        writer.flush()?;
        Ok(())
    }
}

/// Arguments to the program.
#[derive(Debug)]
struct Args {
    /// The input directory for the blog's files.
    pub input_dir: PathBuf,
    /// Where the site should be generated.
    pub output_dir: PathBuf,
}

impl Args {
    fn parse() -> anyhow::Result<Self> {
        let mut args = std::env::args().skip(1);
        Ok(Self {
            input_dir: args.next().ok_or_else(|| anyhow!(USAGE))?.into(),
            output_dir: args.next().ok_or_else(|| anyhow!(USAGE))?.into(),
        })
    }
}

struct Processor {
    content_dir: PathBuf,
    static_dir: PathBuf,
    template_dir: PathBuf,
    output_dir: PathBuf,
}

impl Processor {
    fn new(args: Args) -> Self {
        Self {
            content_dir: args.input_dir.join("content"),
            static_dir: args.input_dir.join("static"),
            template_dir: args.input_dir.join("templates"),
            output_dir: args.output_dir,
        }
    }

    fn run(self) -> anyhow::Result<()> {
        if self.static_dir.is_dir() {
            copy_dir(&self.static_dir, &self.output_dir.join("static"))?;
        }
        let env = Environment::new();
        let template_data = fs::read_to_string(self.template_dir.join("index.html"))?;
        let template = env.template_from_str(&template_data)?;
        for entry in fs::read_dir(&self.content_dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_file() {
                continue;
            }
            let path = entry.path();
            // Skip non-markdown files.
            if !path.extension().map(|x| x == "md").unwrap_or(true) {
                return Ok(());
            }
            File::read(&self.content_dir, &path)
                .with_context(|| format!("failed to process file: {:?}", &path))?
                .write(&self.output_dir, template.clone())?;
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse()?;
    let processor = Processor::new(args);
    processor.run()
}
