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
    pub unpushed_commits: u32,
    pub pr: PrStatus,
    pub repo_url: String,
}

#[derive(Debug, Clone)]
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

        // Count unpushed commits
        let unpushed = Command::new("git")
            .args(["rev-list", "--count", "@{upstream}..HEAD"])
            .current_dir(git_dir)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().ok())
            .unwrap_or(0);

        // Get repo URL for PR links
        let repo_url = Command::new("gh")
            .args(["repo", "view", "--json", "url", "-q", ".url"])
            .current_dir(git_dir)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();

        sub_repos.push(SubRepo {
            name,
            branch: branch.clone(),
            dirty_files: dirty,
            unpushed_commits: unpushed,
            pr,
            repo_url,
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
    // Compatible with gh 2.0+: get all PRs and filter by headRefName
    let output = Command::new("gh")
        .args(["pr", "list", "--state", "all", "--json", "number,state,headRefName", "--limit", "50"])
        .current_dir(repo_path)
        .output();

    let Ok(out) = output else { return PrStatus::None; };
    if !out.status.success() { return PrStatus::None; }

    let text = String::from_utf8_lossy(&out.stdout);
    if let Ok(arr) = serde_json::from_str::<Vec<serde_json::Value>>(&text) {
        // Find PR matching our branch
        for pr in &arr {
            let head = pr.get("headRefName").and_then(|s| s.as_str()).unwrap_or("");
            if head == branch {
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
    }

    PrStatus::None
}

/// Fetch deployments for a branch via gh API
pub fn get_deployments(repo_path: &Path) -> Vec<(String, String, String)> {
    // (env, state, url)
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(repo_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if branch.is_empty() || branch == "main" || branch == "master" { return Vec::new(); }

    let repo_name = Command::new("gh")
        .args(["repo", "view", "--json", "nameWithOwner", "-q", ".nameWithOwner"])
        .current_dir(repo_path)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();

    if repo_name.is_empty() { return Vec::new(); }

    // Get recent deployments and match by branch name in environment/description
    let endpoint = format!("repos/{}/deployments?per_page=30", repo_name);
    let output = Command::new("gh")
        .args(["api", &endpoint])
        .current_dir(repo_path)
        .output();

    let Ok(out) = output else { return Vec::new(); };
    if !out.status.success() { return Vec::new(); }

    let text = String::from_utf8_lossy(&out.stdout);
    let Ok(deployments) = serde_json::from_str::<Vec<serde_json::Value>>(&text) else { return Vec::new(); };

    // Match by exact branch name in description or ref
    let branch_lower = branch.to_lowercase();

    let mut results = Vec::new();
    let mut seen_envs = std::collections::HashSet::new();
    for d in &deployments {
        let env = d.get("environment").and_then(|v| v.as_str()).unwrap_or("").to_string();
        let description = d.get("description").and_then(|v| v.as_str()).unwrap_or("").to_lowercase();
        let ref_field = d.get("ref").and_then(|v| v.as_str()).unwrap_or("");

        // Match by exact branch name in description or ref field
        let matches = description.contains(&branch_lower) || ref_field == branch;

        if !matches || seen_envs.contains(&env) { continue; }
        seen_envs.insert(env.clone());

        let statuses_url = d.get("statuses_url").and_then(|v| v.as_str()).unwrap_or("");

        // Fetch status
        if !statuses_url.is_empty() {
            // Extract path from URL for gh api
            let path = statuses_url.replace("https://api.github.com/", "");
            if let Ok(st_out) = Command::new("gh").args(["api", &path]).current_dir(repo_path).output() {
                if st_out.status.success() {
                    let st_text = String::from_utf8_lossy(&st_out.stdout);
                    if let Ok(statuses) = serde_json::from_str::<Vec<serde_json::Value>>(&st_text) {
                        if let Some(s) = statuses.first() {
                            let state = s.get("state").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
                            let url = s.get("environment_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            results.push((env, state, url));
                            continue;
                        }
                    }
                }
            }
        }
        results.push((env, "unknown".to_string(), String::new()));
    }
    results
}

/// Find directories containing `.git` up to max_depth (public wrapper)
pub fn find_git_repos_pub(root: &Path, max_depth: usize) -> Vec<PathBuf> {
    find_git_repos(root, max_depth)
}

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
