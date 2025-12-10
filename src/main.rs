use anyhow::{Context, anyhow};
use minijinja::Environment;
use std::{
    borrow::Cow,
    fs::{self},
    path::PathBuf,
};

mod frontmatter;

mod file;
use file::File;

mod fs_utils;
use fs_utils::copy_dir;

/// A static string for usage errors.
const USAGE: &str = "usage: clog <input_dir> <output_dir>";

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
        let mut dirs = vec![Cow::Borrowed(&self.content_dir)];
        while let Some(dir) = dirs.pop() {
            for entry in fs::read_dir(dir.as_path())? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                if !file_type.is_file() {
                    if file_type.is_dir() {
                        dirs.push(Cow::Owned(entry.path()));
                    }
                    continue;
                }
                let path = entry.path();
                let Some(extension) = path.extension() else {
                    continue;
                };
                // Copy any images "in place".
                // Contains won't work because of the need to cast.
                if ["png", "jpg"].into_iter().any(|x| x == extension) {
                    let rel_path = path.strip_prefix(&self.content_dir)?;
                    let out_path = self.output_dir.join(rel_path);
                    if let Some(parent) = out_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::copy(&path, &self.output_dir.join(rel_path))?;
                    continue;
                }
                // Skip non-markdown files.
                if !path.extension().map(|x| x == "md").unwrap_or(true) {
                    continue;
                }
                File::read(&self.content_dir, &path)
                    .with_context(|| format!("failed to read file: {:?}", &path))?
                    .write(&self.output_dir, template.clone())
                    .with_context(|| format!("failed to write file: {:?}", &path))?;
            }
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse()?;
    let processor = Processor::new(args);
    processor.run()
}
