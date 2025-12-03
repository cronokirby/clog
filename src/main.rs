use anyhow::anyhow;
use markdown::to_html;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

mod fs_utils;
use fs_utils::copy_dir;

/// A static string for usage errors.
const USAGE: &str = "usage: clog <input_dir> <output_dir>";

// 1 MiB for input and output file buffers.

/// The starting capacity of the input buffer.
const INPUT_CAPACITY: usize = 1 << 20;

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
    output_dir: PathBuf,
    input_buf: String,
}

impl Processor {
    fn new(args: Args) -> Self {
        Self {
            content_dir: args.input_dir.join("content"),
            static_dir: args.input_dir.join("static"),
            output_dir: args.output_dir,
            input_buf: String::with_capacity(INPUT_CAPACITY),
        }
    }

    fn run(mut self) -> anyhow::Result<()> {
        if self.static_dir.is_dir() {
            copy_dir(&self.static_dir, &self.output_dir.join("static"))?;
        }
        for entry in fs::read_dir(&self.content_dir)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_file() {
                self.process_file(&entry.path())?;
            }
        }
        Ok(())
    }

    fn process_file(&mut self, path: &Path) -> anyhow::Result<()> {
        // Skip non-markdown files.
        if !path.extension().map(|x| x == "md").unwrap_or(true) {
            return Ok(());
        }
        let output_path = self
            .output_dir
            .join(path.strip_prefix(&self.content_dir)?.with_extension("html"));
        let mut file = fs::File::open(path)?;
        self.input_buf.clear();
        file.read_to_string(&mut self.input_buf)?;
        let html = to_html(&self.input_buf);
        fs::write(output_path, html)?;
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse()?;
    let processor = Processor::new(args);
    processor.run()
}
