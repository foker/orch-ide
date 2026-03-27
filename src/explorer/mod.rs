use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub enum FileEntryKind {
    Directory,
    File,
}

#[derive(Debug, Clone)]
pub enum GitStatus {
    Unmodified,
    Modified,
    Added,
    Deleted,
    Untracked,
}

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub kind: FileEntryKind,
    pub depth: usize,
    pub expanded: bool,
    pub git_status: GitStatus,
}

/// Read directory entries (non-recursive, top level only until expanded)
pub fn read_directory(path: &Path, depth: usize) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    let Ok(read_dir) = std::fs::read_dir(path) else {
        return entries;
    };

    let mut items: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            // Skip hidden files except some important ones
            !name.starts_with('.') || name == ".env" || name == ".claude"
        })
        .collect();

    // Sort: directories first, then alphabetical
    items.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    for item in items {
        let name = item.file_name().to_string_lossy().to_string();
        let is_dir = item.file_type().map(|t| t.is_dir()).unwrap_or(false);

        // Skip node_modules, target, .git etc
        if is_dir && matches!(name.as_str(), "node_modules" | "target" | ".git" | "dist" | "build" | "__pycache__" | ".next" | "vendor" | "coverage" | ".turbo" | ".cache") {
            continue;
        }

        entries.push(FileEntry {
            name,
            path: item.path(),
            kind: if is_dir {
                FileEntryKind::Directory
            } else {
                FileEntryKind::File
            },
            depth,
            expanded: false,
            git_status: GitStatus::Unmodified,
        });
    }

    entries
}
