use std::{
    ffi::OsStr,
    fs, io,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
};

fn sorted_entries(directory: &Path, keep_entry: fn(&Path) -> bool) -> io::Result<Vec<PathBuf>> {
    let mut entry_paths = fs::read_dir(directory)?
        .map(|entry| entry.map(|entry| entry.path()))
        .filter(|path| path.as_ref().map_or(true, |path| keep_entry(path)))
        .collect::<io::Result<Vec<_>>>()?;
    entry_paths.sort();
    Ok(entry_paths)
}

pub fn regular_files(directory: &Path) -> io::Result<Vec<PathBuf>> {
    sorted_entries(directory, Path::is_file)
}

pub fn subdirectories(directory: &Path) -> io::Result<Vec<PathBuf>> {
    sorted_entries(directory, Path::is_dir)
}

pub fn files_with_suffix(directory: &Path, suffix: &str) -> io::Result<Vec<PathBuf>> {
    Ok(regular_files(directory)?
        .into_iter()
        .filter(|path| path.to_string_lossy().ends_with(suffix))
        .collect())
}

pub fn file_names(paths: &[PathBuf]) -> Vec<&OsStr> {
    paths.iter().filter_map(|path| path.file_name()).collect()
}

pub fn copy_writable_file(source: &Path, destination: &Path) -> io::Result<()> {
    fs::copy(source, destination)?;
    let mut permissions = fs::metadata(destination)?.permissions();
    permissions.set_mode(permissions.mode() | 0o200);
    fs::set_permissions(destination, permissions)
}

pub fn copy_writable_tree(source: &Path, destination: &Path) -> io::Result<()> {
    fs::create_dir_all(destination)?;
    fs::read_dir(source)?.try_for_each(|entry| {
        let entry_path = entry?.path();
        let target = destination.join(entry_path.file_name().unwrap_or_default());
        if entry_path.is_dir() {
            copy_writable_tree(&entry_path, &target)
        } else {
            copy_writable_file(&entry_path, &target)
        }
    })
}
