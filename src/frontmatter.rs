use anyhow::anyhow;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::{path::Path, sync::LazyLock, time::SystemTime};
use time::{OffsetDateTime, UtcOffset, format_description::well_known::Iso8601};

fn systemtime_to_date_str(t: SystemTime) -> anyhow::Result<String> {
    let dt = OffsetDateTime::from(t).to_offset(UtcOffset::UTC);
    Ok(dt.format(&Iso8601::DATE)?)
}

fn mtime_date(path: &Path) -> anyhow::Result<String> {
    let meta = std::fs::metadata(path)?;
    let mtime = meta.modified()?;
    let date = systemtime_to_date_str(mtime)?;
    Ok(date)
}

fn date_prefix(s: &str) -> Option<&str> {
    // Matches YYYY-MM-DD at the start of the string
    // \d{4}-\d{2}-\d{2}
    static DATE_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^(\d{4}-\d{2}-\d{2})").unwrap());

    DATE_RE
        .captures(s)
        .and_then(|caps| caps.get(1).map(|m| m.as_str()))
}

/// Allow `authors` to be:
/// - missing/null  → None
/// - "Alice"       → Some(vec!["Alice"])
/// - ["Alice","Bob"] → Some(vec!["Alice","Bob"])
fn opt_string_or_vec<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    // First try to deserialize as Option<...>
    let opt = Option::<StringOrVec>::deserialize(deserializer)?;

    Ok(opt.map(|v| match v {
        StringOrVec::One(s) => vec![s],
        StringOrVec::Many(v) => v,
    }))
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrVec {
    One(String),
    Many(Vec<String>),
}

#[derive(Default, Serialize, Deserialize)]
struct Raw {
    title: Option<String>,
    date: Option<String>,
    modified: Option<String>,
    created: Option<String>,
    published: Option<String>,
    #[serde(default, deserialize_with = "opt_string_or_vec")]
    authors: Option<Vec<String>>,
    draft: Option<String>,
    link: Option<String>,
    #[serde(default, deserialize_with = "opt_string_or_vec")]
    tags: Option<Vec<String>>,
}

impl Raw {
    fn date(&self, path: &Path) -> anyhow::Result<String> {
        match self
            .modified
            .as_ref()
            .and_then(|x| date_prefix(x))
            .or(self.created.as_ref().and_then(|x| date_prefix(x)))
            .or(self.date.as_ref().and_then(|x| date_prefix(x)))
        {
            Some(x) => Ok(x.to_owned()),
            None => mtime_date(path),
        }
    }

    fn title(&self, path: &Path) -> anyhow::Result<String> {
        match self.title.as_ref() {
            Some(x) => Ok(x.to_owned()),
            None => {
                let stem = path
                    .file_stem()
                    .ok_or_else(|| anyhow!("failed to get file stem"))?;
                Ok(stem.to_string_lossy().into_owned())
            }
        }
    }

    fn draft(&self) -> bool {
        self.draft
            .as_ref()
            .map(|x| x.to_lowercase() == "true")
            .unwrap_or(false)
    }

    fn authors(&self) -> Vec<String> {
        self.authors.clone().unwrap_or_default()
    }

    fn published(&self) -> Option<String> {
        self.published
            .as_ref()
            .and_then(|x| date_prefix(x))
            .map(|x| x.to_owned())
    }

    fn link(&self) -> Option<String> {
        self.link.clone()
    }

    fn tags(&self) -> Vec<String> {
        self.tags.clone().unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct FrontMatter {
    pub title: String,
    pub draft: bool,
    pub date: String,
    pub authors: Vec<String>,
    pub published: Option<String>,
    pub link: Option<String>,
    pub tags: Vec<String>,
}

impl FrontMatter {
    /// Attempt to parse a yaml string into this front matter.
    ///
    /// We use the full path to populate some of the missing fields.
    ///
    /// In detail, if not present in the frontmatter:
    /// - the title will be extracted from the end of the path,
    /// - the date will be extracted from the modified time of the file.
    pub fn try_from_yaml(path: &Path, yaml: Option<&str>) -> anyhow::Result<Self> {
        let raw: Raw = match yaml {
            Some(y) => serde_yaml::from_str(y)?,
            None => Default::default(),
        };
        Ok(Self {
            title: raw.title(path)?,
            draft: raw.draft(),
            date: raw.date(path)?,
            authors: raw.authors(),
            published: raw.published(),
            link: raw.link(),
            tags: raw.tags(),
        })
    }
}
