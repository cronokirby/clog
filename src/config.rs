use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf};

/// Configuration for how to generate the site.
#[derive(Serialize, Deserialize, Default)]
pub struct Config {
    /// Which folders to ignore.
    ///
    /// These should be relative to `content`, e.g. if your site has:
    ///
    /// >  `content/Ignored/A`
    ///
    /// then the config should contain
    ///
    /// ```
    /// ignored_folders: ["Ignored/A"]
    /// ```
    pub ignored_folders: HashSet<PathBuf>,
    /// Folders to generate list pages for.
    ///
    /// The template for this should be `list.html`.
    pub list_folders: HashSet<PathBuf>,
}

impl Config {
    /// Parse the config from a YAML string.
    pub fn try_from_yaml(yaml: &str) -> anyhow::Result<Self> {
        serde_yaml::from_str(yaml).map_err(|e| anyhow!("failed to parse config: {e}"))
    }
}
