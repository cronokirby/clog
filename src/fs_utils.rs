use std::{fs, path::Path};

/// Copy the files in a directory to another directory.
///
/// This does not follow symlinks, or enter sub-directories.
pub fn copy_dir(in_dir: &Path, out_dir: &Path) -> anyhow::Result<()> {
    let mut out_path = out_dir.to_path_buf();
    fs::create_dir_all(&out_path)?;
    for entry in fs::read_dir(in_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_file() {
            out_path.push(entry.file_name());
            fs::copy(entry.path(), &out_path)?;
            out_path.pop();
        }
    }
    Ok(())
}
