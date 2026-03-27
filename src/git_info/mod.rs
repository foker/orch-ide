use std::path::{Path, PathBuf};
use std::process::Command;
use serde_json;

#[derive(Debug, Clone)]
pub enum PrStatus {
    None,
    Open(String),    // PR #number
    Merged(String),  // PR #number (merged)
}

#[derive(Debug, Clone)]
pub struct SubRepo {
    pub name: String,
    pub branch: String,
    pub dirty_files: u32,
    pub pr: PrStatus,
}

pub struct GitInfo {
    pub branch: String,     // main repo branch (or first found)
    pub dirty_files: u32,   // total across all repos
    pub has_remote: bool,
    pub sub_repos: Vec<SubRepo>,
}

/// Get git info — scans for nested git repos up to depth 3
/// check_pr: if true, also queries GitHub API for PR status (slow!)
pub fn get_git_info(repo_path: &Path) -> Option<GitInfo> {
    get_git_info_impl(repo_path, false)
}

pub fn get_git_info_with_pr(repo_path: &Path) -> Option<GitInfo> {
    get_git_info_impl(repo_path, true)
}

fn get_git_info_impl(repo_path: &Path, check_pr: bool) -> Option<GitInfo> {
    let mut sub_repos = Vec::new();

    // Find all git repos: the path itself + nested dirs up to depth 3
    let git_dirs = find_git_repos(repo_path, 3);

    if git_dirs.is_empty() {
        return None;
    }

    let mut total_dirty = 0u32;
    let mut main_branch = String::from("n/a");
    let mut has_remote = false;

    for git_dir in &git_dirs {
        let branch = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(git_dir)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_else(|| "n/a".to_string());

        let dirty = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(git_dir)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| {
                String::from_utf8_lossy(&o.stdout)
                    .lines()
                    .filter(|l| !l.is_empty())
                    .count() as u32
            })
            .unwrap_or(0);

        let name = git_dir.strip_prefix(repo_path)
            .ok()
            .map(|p| {
                let s = p.to_string_lossy().to_string();
                if s.is_empty() { ".".to_string() } else { s }
            })
            .unwrap_or_else(|| ".".to_string());

        total_dirty += dirty;

        // Check PR status via gh CLI (only when requested, and only for non-main branches)
        let pr = if check_pr && branch != "main" && branch != "master" && branch != "n/a" {
            get_pr_status(git_dir, &branch)
        } else {
            PrStatus::None
        };

        sub_repos.push(SubRepo {
            name,
            branch: branch.clone(),
            dirty_files: dirty,
            pr,
        });
    }

    // Main branch = first repo's branch
    if let Some(first) = sub_repos.first() {
        main_branch = first.branch.clone();
    }

    // Check remote for root repo
    has_remote = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "@{upstream}"])
        .current_dir(&git_dirs[0])
        .output()
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    Some(GitInfo {
        branch: main_branch,
        dirty_files: total_dirty,
        has_remote,
        sub_repos,
    })
}

/// Check PR status for a branch using `gh` CLI
fn get_pr_status(repo_path: &Path, branch: &str) -> PrStatus {
    // gh pr list --head <branch> --json number,state --limit 1
    let output = Command::new("gh")
        .args(["pr", "list", "--head", branch, "--json", "number,state", "--limit", "1", "--state", "all"])
        .current_dir(repo_path)
        .output();

    let Ok(out) = output else { return PrStatus::None; };
    if !out.status.success() { return PrStatus::None; }

    let text = String::from_utf8_lossy(&out.stdout);
    // Parse JSON array: [{"number":42,"state":"OPEN"}]
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        if let Some(pr) = arr.first() {
            let number = pr.get("number").and_then(|n| n.as_u64()).unwrap_or(0);
            let state = pr.get("state").and_then(|s| s.as_str()).unwrap_or("");
            let label = format!("#{}", number);
            return match state {
                "OPEN" => PrStatus::Open(label),
                "MERGED" => PrStatus::Merged(label),
                _ => PrStatus::None,
            };
        }
    }

    PrStatus::None
}

/// Find directories containing `.git` up to max_depth
fn find_git_repos(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    let mut repos = Vec::new();

    // Check root itself
    if root.join(".git").exists() {
        repos.push(root.to_path_buf());
    }

    // Scan subdirs
    if max_depth > 0 {
        scan_for_git(root, 0, max_depth, &mut repos);
    }

    repos
}

fn scan_for_git(dir: &Path, depth: usize, max_depth: usize, repos: &mut Vec<PathBuf>) {
    if depth >= max_depth {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else { return; };

    for entry in entries.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        // Skip hidden dirs (except .git check), node_modules, target, etc.
        if name.starts_with('.') || matches!(name.as_str(), "node_modules" | "target" | "dist" | "build" | "__pycache__" | ".git") {
            continue;
        }

        // Check if this dir is a git repo
        if path.join(".git").exists() {
            repos.push(path.clone());
            // Don't recurse into nested repos
            continue;
        }

        // Recurse deeper
        scan_for_git(&path, depth + 1, max_depth, repos);
    }
}
