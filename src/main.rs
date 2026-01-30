use anyhow::anyhow;
use minijinja::{Environment, context};
use std::{
    borrow::Cow,
    fs::{self},
    io::{BufWriter, Write},
    path::PathBuf,
};

mod config;
mod frontmatter;
mod fs_utils;
mod markdown;
mod sitemap;
mod slug;
mod wikilink;

use fs_utils::copy_dir;
use sitemap::SiteMap;

use crate::{
    config::Config,
    markdown::{extract_description, make_mdast, write_md_ast},
    sitemap::Page,
    slug::{slugify, slugify_path},
};

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
    config_file: PathBuf,
    content_dir: PathBuf,
    static_dir: PathBuf,
    template_dir: PathBuf,
    output_dir: PathBuf,
}

impl Processor {
    fn new(args: Args) -> Self {
        Self {
            config_file: args.input_dir.join("config.yaml"),
            content_dir: args.input_dir.join("content"),
            static_dir: args.input_dir.join("static"),
            template_dir: args.input_dir.join("templates"),
            output_dir: args.output_dir,
        }
    }

    fn config(&self) -> anyhow::Result<Config> {
        if !fs::exists(&self.config_file)? {
            return Ok(Default::default());
        }
        let yaml = fs::read_to_string(&self.config_file)?;
        Config::try_from_yaml(&yaml)
    }

    fn copy_static_files(&self) -> anyhow::Result<()> {
        if self.static_dir.is_dir() {
            copy_dir(&self.static_dir, &self.output_dir.join("static"))?;
        }
        Ok(())
    }

    fn run(self) -> anyhow::Result<()> {
        let config = self.config()?;

        let env = Environment::new();

        let content_template_data = fs::read_to_string(self.template_dir.join("index.html"))?;
        let list_template_data = {
            let path = self.template_dir.join("list.html");
            if fs::exists(&path)? {
                Some(fs::read_to_string(&path)?)
            } else {
                None
            }
        };
        let content_template = env.template_from_str(&content_template_data)?;
        let list_template = list_template_data
            .as_ref()
            .map(|x| env.template_from_str(x))
            .transpose()?;

        let site_map = SiteMap::build(&config, &self.content_dir, &self.output_dir)?;

        for file in site_map.statics() {
            if let Some(parent) = file.out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&file.in_path, &file.out_path)?;
        }
        if let Some(list_template) = list_template {
            let work = site_map
                .folders()
                .map(|(folder, pages)| {
                    let slugified = slugify_path(folder);
                    let out_path = self.output_dir.join(&slugified).join("index.html");
                    let url = format!("/{}/", slugified.display());
                    let iter: Box<dyn Iterator<Item = &'_ Page>> = Box::new(pages);
                    (out_path, folder.to_string_lossy(), url, iter)
                })
                .chain(site_map.pages_by_tag().map(|(tag, pages)| {
                    let slugified_tag = slugify(tag);
                    let out_path = self
                        .output_dir
                        .join("tag")
                        .join(&slugified_tag)
                        .join("index.html");
                    let url = format!("/tag/{}/", slugified_tag);
                    let iter: Box<dyn Iterator<Item = &'_ Page>> = Box::new(pages);
                    (out_path, Cow::Owned(format!("Tag - #{tag}")), url, iter)
                }));
            for (out_path, title, url, pages) in work {
                let items = pages
                    .filter_map(|page| {
                        if page.front_matter.draft {
                            return None;
                        }
                        Some(context! {
                            title => page.front_matter.title,
                            date => page.front_matter.date,
                            link => page.link,
                            tags => page.front_matter.tags
                        })
                    })
                    .collect::<Vec<_>>();
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let file = fs::File::create(&out_path)?;
                let mut writer = BufWriter::new(file);
                let ctx = context! {
                  title => title,
                  items => items,
                  url => url
                };
                list_template.render_to_write(ctx, &mut writer)?;
                writer.flush()?;
            }
        }

        let mut buf = Vec::with_capacity(1 << 14);
        let katex_ctx = katex::KatexContext::default();
        for page in site_map.pages() {
            let content = fs::read_to_string(&page.in_path)?;
            let md = make_mdast(&content)?;
            let (log, body) = {
                buf.clear();
                let log = write_md_ast(&mut buf, &site_map, &katex_ctx, &md)?;
                (log, String::from_utf8_lossy(&buf))
            };
            let backlinks = site_map
                .backlinks(page)
                .map(|linking_page| {
                    context! {
                        title => linking_page.front_matter.title,
                        link => linking_page.link
                    }
                })
                .collect::<Vec<_>>();
            if let Some(parent) = page.out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let file = fs::File::create(&page.out_path)?;
            let mut writer = BufWriter::new(file);
            let description = extract_description(&md, 160);
            let ctx = context! {
              body => body,
              math => log.math,
              title => page.front_matter.title,
              date => page.front_matter.date,
              authors => page.front_matter.authors,
              published => page.front_matter.published,
              link => page.front_matter.link,
              tags => page.front_matter.tags,
              backlinks => backlinks,
              url => page.link,
              description => description
            };
            content_template.render_to_write(ctx, &mut writer)?;
            writer.flush()?;
        }

        self.copy_static_files()?;

        if let Some(base_url) = &config.base_url {
            let mut sitemap = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n");
            for page in site_map.pages() {
                if page.front_matter.draft {
                    continue;
                }
                sitemap.push_str(&format!(
                    "<url><loc>{}{}</loc><lastmod>{}</lastmod></url>\n",
                    base_url, page.link, page.front_matter.date
                ));
            }
            sitemap.push_str("</urlset>\n");
            fs::write(self.output_dir.join("sitemap.xml"), sitemap)?;
        }

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse()?;
    let processor = Processor::new(args);
    processor.run()
}
