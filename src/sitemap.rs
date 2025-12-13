use crate::{
    config::Config,
    frontmatter::FrontMatter,
    markdown::{find_yaml_frontmatter, make_mdast},
};
use anyhow::anyhow;
use std::{
    borrow::Cow,
    cmp::Reverse,
    collections::HashMap,
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
};

const STATIC_EXTENSIONS: [&str; 2] = ["png", "jpg"];

fn is_static_extension(e: &OsStr) -> bool {
    STATIC_EXTENSIONS.iter().any(|&x| x == e)
}

/// Translate a path, moving it from having in_path as parent, to out_path.
fn translate(in_path: &Path, out_path: &Path, path: &Path) -> anyhow::Result<PathBuf> {
    Ok(out_path.join(path.strip_prefix(in_path)?))
}

fn read_front_matter(path: &Path) -> anyhow::Result<FrontMatter> {
    let contents = fs::read_to_string(path)?;
    let ast = make_mdast(&contents)?;
    let yaml = find_yaml_frontmatter(&ast);
    let fm = FrontMatter::try_from_yaml(&path, yaml)?;
    Ok(fm)
}

/// A Static file, like an image.
///
/// This is still contained inside of the content folder.
#[derive(Debug)]
pub struct Static {
    pub in_path: PathBuf,
    pub out_path: PathBuf,
}

/// A page with actual markdown content.
#[derive(Clone, Debug)]
pub struct Page {
    pub name: String,
    pub link: String,
    pub front_matter: FrontMatter,
    pub in_path: PathBuf,
    pub out_path: PathBuf,
}

impl Page {
    pub fn folder(&self, base: &Path) -> anyhow::Result<Option<PathBuf>> {
        let Some(parent) = self.in_path.parent() else {
            return Ok(None);
        };
        Ok(Some(parent.strip_prefix(base)?.to_path_buf()))
    }
}

type PageIndex = usize;

fn sort_page_indices(pages: &[Page], indices: &mut [PageIndex]) {
    indices.sort_by_key(|&i| {
        (
            Reverse(&pages[i].front_matter.date),
            Reverse(&pages[i].front_matter.title),
        )
    });
}

#[derive(Debug)]
pub struct SiteMap {
    statics: Vec<Static>,
    pages: Vec<Page>,
    pages_by_name: HashMap<String, Vec<usize>>,
    pages_by_tag: HashMap<String, Vec<usize>>,
    folders: HashMap<PathBuf, Vec<usize>>,
}

impl SiteMap {
    pub fn build(config: &Config, in_path: &Path, out_path: &Path) -> anyhow::Result<Self> {
        let mut statics: Vec<Static> = Vec::with_capacity(128);
        let mut pages: Vec<Page> = Vec::with_capacity(1024);
        let mut q = vec![Cow::Borrowed(in_path)];
        while let Some(dir) = q.pop() {
            let rel_path = dir.strip_prefix(in_path)?;
            if config.ignored_folders.contains(rel_path) {
                continue;
            }
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let file_type = entry.file_type()?;
                if file_type.is_dir() {
                    q.push(Cow::Owned(entry.path()));
                    continue;
                }
                if !file_type.is_file() {
                    continue;
                }
                let path = entry.path();
                if path.to_str().is_none() {
                    continue;
                }
                let Some(extension) = path.extension() else {
                    continue;
                };
                if is_static_extension(extension) {
                    statics.push(Static {
                        out_path: translate(in_path, out_path, &path)?,
                        in_path: path,
                    });
                    continue;
                }
                if extension != "md" {
                    continue;
                }
                let front_matter = read_front_matter(&path)?;
                let name = path
                    .file_stem()
                    .and_then(|x| x.to_str())
                    .ok_or_else(|| anyhow!("failed to get file stem"))?
                    .to_string();
                let link = {
                    let rel_path = path.strip_prefix(in_path)?.with_extension("html");
                    let out_segment = rel_path.to_str().unwrap();
                    let mut out = String::with_capacity(1 + out_segment.len());
                    out.push('/');
                    out.push_str(out_segment);
                    out
                };
                pages.push(Page {
                    name,
                    link,
                    front_matter,
                    out_path: translate(in_path, out_path, &path.with_extension("html"))?,
                    in_path: path,
                });
            }
        }
        let mut pages_by_name = {
            let mut out = HashMap::<_, Vec<_>>::new();
            for (i, page) in pages.iter().enumerate() {
                out.entry(page.name.clone()).or_default().push(i);
            }
            out
        };
        let mut pages_by_tag = {
            let mut out = HashMap::<_, Vec<_>>::new();
            for (i, page) in pages.iter().enumerate() {
                for tag in &page.front_matter.tags {
                    out.entry(tag.clone()).or_default().push(i);
                }
            }
            out
        };
        // Generate warnings for duplicate names
        for (name, indices) in &pages_by_name {
            if indices.len() > 1 {
                eprintln!("WARNING: `{name}` has conflicts");
                for &i in indices {
                    eprintln!("\t{}", pages[i].in_path.to_string_lossy());
                }
            }
        }
        let mut folders = {
            let mut out = HashMap::<_, Vec<_>>::new();
            for (i, page) in pages.iter().enumerate() {
                if let Some(folder) = page.folder(in_path)? {
                    out.entry(folder).or_default().push(i);
                }
            }
            out
        };
        // Sort grouped pages.
        for list in pages_by_name.values_mut() {
            sort_page_indices(&pages, list);
        }
        for list in pages_by_tag.values_mut() {
            sort_page_indices(&pages, list);
        }
        for list in folders.values_mut() {
            sort_page_indices(&pages, list);
        }
        Ok(Self {
            statics,
            pages,
            pages_by_name,
            pages_by_tag,
            folders,
        })
    }

    /// Iterate over all the static files in the content directory.
    pub fn statics(&self) -> impl Iterator<Item = &Static> {
        self.statics.iter()
    }

    /// Iterate over all of the pages.
    pub fn pages(&self) -> impl Iterator<Item = &Page> {
        self.pages.iter()
    }

    /// Attempt to fetch a specific page by name.
    pub fn page_by_name(&self, name: &str) -> Option<&Page> {
        let i = *self.pages_by_name.get(name)?.first()?;
        Some(&self.pages[i])
    }

    /// Iterate over all of the folders
    pub fn folders<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a Path, impl Iterator<Item = &'a Page>)> {
        self.folders
            .iter()
            .map(|(path, indices)| (path.as_path(), indices.iter().map(|&i| &self.pages[i])))
    }

    /// Iterate over all pages in each tag.
    pub fn pages_by_tag<'a>(
        &'a self,
    ) -> impl Iterator<Item = (&'a str, impl Iterator<Item = &'a Page>)> {
        self.pages_by_tag
            .iter()
            .map(|(tag, indices)| (tag.as_str(), indices.iter().map(|&i| &self.pages[i])))
    }
}
