use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

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

/// Get git status map for a directory
fn get_git_statuses(path: &Path) -> HashMap<PathBuf, GitStatus> {
    let mut map = HashMap::new();

    let output = Command::new("git")
        .args(["status", "--porcelain", "-uall"])
        .current_dir(path)
        .output();

    let Ok(out) = output else { return map; };
    if !out.status.success() { return map; }

    // Find git root to resolve relative paths
    let git_root = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()))
        .unwrap_or_else(|| path.to_path_buf());

    for line in String::from_utf8_lossy(&out.stdout).lines() {
        if line.len() < 4 { continue; }
        let status_code = &line[..2];
        let file_path = line[3..].trim();

        let full_path = git_root.join(file_path);

        let status = match status_code.trim() {
            "M" | " M" | "MM" | "AM" => GitStatus::Modified,
            "A" | " A" => GitStatus::Added,
            "D" | " D" => GitStatus::Deleted,
            "??" => GitStatus::Untracked,
            _ => GitStatus::Modified,
        };

        map.insert(full_path, status);
    }
    map
}

/// Read directory entries (non-recursive, top level only until expanded)
pub fn read_directory(path: &Path, depth: usize) -> Vec<FileEntry> {
    let mut entries = Vec::new();

    let Ok(read_dir) = std::fs::read_dir(path) else {
        return entries;
    };

    // Get git statuses for coloring
    let statuses = if depth == 0 { get_git_statuses(path) } else { HashMap::new() };

    let mut items: Vec<_> = read_dir
        .filter_map(|e| e.ok())
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !name.starts_with('.') || name == ".env" || name == ".claude"
        })
        .collect();

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

        if is_dir && matches!(name.as_str(), "node_modules" | "target" | ".git" | "dist" | "build" | "__pycache__" | ".next" | "vendor" | "coverage" | ".turbo" | ".cache") {
            continue;
        }

        let item_path = item.path();
        let git_status = statuses.get(&item_path)
            .cloned()
            .unwrap_or(GitStatus::Unmodified);

        entries.push(FileEntry {
            name,
            path: item_path,
            kind: if is_dir { FileEntryKind::Directory } else { FileEntryKind::File },
            depth,
            expanded: false,
            git_status,
        });
    }

    entries
}
