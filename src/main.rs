use anyhow::anyhow;
use minijinja::{Environment, context};
use std::{
    fs::{self},
    io::{BufWriter, Write},
    path::PathBuf,
};

mod frontmatter;
mod fs_utils;
mod markdown;
mod sitemap;

use fs_utils::copy_dir;
use sitemap::SiteMap;

use crate::markdown::{make_mdast, write_md_ast};

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
        let site_map = SiteMap::build(&self.content_dir, &self.output_dir)?;
        for file in site_map.statics() {
            if let Some(parent) = file.out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&file.in_path, &file.out_path)?;
        }
        let mut buf = Vec::with_capacity(1 << 14);
        for page in site_map.pages() {
            let content = fs::read_to_string(&page.in_path)?;
            let md = make_mdast(&content)?;
            let body = {
                buf.clear();
                write_md_ast(&mut buf, &md)?;
                String::from_utf8_lossy(&buf)
            };
            if let Some(parent) = page.out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = fs::File::create(&page.out_path)?;
            let mut writer = BufWriter::new(file);
            let ctx = context! {
              body => body,
              title => page.front_matter.title,
              date => page.front_matter.date,
              authors => page.front_matter.authors,
              published => page.front_matter.published,
              link => page.front_matter.link,
              tags => page.front_matter.tags,
            };
            template.render_to_write(ctx, &mut writer)?;
            writer.flush()?;
        }
        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse()?;
    let processor = Processor::new(args);
    processor.run()
}
