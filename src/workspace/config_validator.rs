use anyhow::Result;
use console::style;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use super::config::{Repository, WorkspaceConfig};

#[derive(Debug, Clone)]
pub struct DuplicateRepository {
    pub repositories: Vec<Repository>,
    pub conflict_type: DuplicateType,
    pub recommended_action: RecommendedAction,
}

#[derive(Debug, Clone)]
pub enum DuplicateType {
    /// Same URL, different paths/names
    SameUrl,
    /// Same path, different URLs/names  
    SamePath,
    /// Same name, different URLs/paths
    SameName,
}

#[derive(Debug, Clone)]
pub enum RecommendedAction {
    /// Keep the first entry, remove others
    KeepFirst,
    /// Keep the entry with the most complete information
    KeepMostComplete,
    /// Keep the entry that exists on filesystem
    KeepExisting,
    /// Manual review required
    ManualReview,
}

#[derive(Debug)]
pub struct ValidationReport {
    pub duplicates: Vec<DuplicateRepository>,
    pub warnings: Vec<String>,
    pub total_repositories: usize,
    pub unique_repositories: usize,
}

impl ValidationReport {
    pub fn has_issues(&self) -> bool {
        !self.duplicates.is_empty() || !self.warnings.is_empty()
    }

    pub fn print_report(&self) {
        if !self.has_issues() {
            println!("{} No config issues found", style("✅").green());
            return;
        }

        println!("{} Config Validation Report", style("⚠️").yellow().bold());
        println!("{}", "─".repeat(50));

        if !self.duplicates.is_empty() {
            println!("{} Duplicate Repositories Found", style("🔍").blue());
            println!();

            for (i, duplicate) in self.duplicates.iter().enumerate() {
                println!(
                    "{}. {} ({:?})",
                    i + 1,
                    style("Conflict").red().bold(),
                    duplicate.conflict_type
                );

                for (j, repo) in duplicate.repositories.iter().enumerate() {
                    let marker = if j == 0 { "→" } else { " " };
                    println!(
                        "  {} {}: {} ({})",
                        marker,
                        style(&repo.name).cyan(),
                        repo.path.display(),
                        repo.url.as_deref().unwrap_or("no URL")
                    );
                }

                match duplicate.recommended_action {
                    RecommendedAction::KeepFirst => {
                        println!("  {} Keep first entry, remove others", style("💡").yellow());
                    }
                    RecommendedAction::KeepMostComplete => {
                        println!("  {} Keep most complete entry", style("💡").yellow());
                    }
                    RecommendedAction::KeepExisting => {
                        println!(
                            "  {} Keep entry that exists on filesystem",
                            style("💡").yellow()
                        );
                    }
                    RecommendedAction::ManualReview => {
                        println!("  {} Manual review required", style("⚠️").yellow());
                    }
                }
                println!();
            }
        }

        if !self.warnings.is_empty() {
            println!("{} Warnings", style("⚠️").yellow());
            for warning in &self.warnings {
                println!("  • {warning}");
            }
            println!();
        }

        println!("{} Summary:", style("📊").blue());
        println!("  Total entries: {}", self.total_repositories);
        println!("  Unique repositories: {}", self.unique_repositories);
        println!("  Duplicates found: {}", self.duplicates.len());
    }
}

pub fn validate_config(
    config: &WorkspaceConfig,
    workspace_root: &Path,
) -> Result<ValidationReport> {
    let mut duplicates = Vec::new();
    let mut warnings = Vec::new();

    // Group repositories by URL, path, and name
    let mut by_url: HashMap<String, Vec<&Repository>> = HashMap::new();
    let mut by_path: HashMap<String, Vec<&Repository>> = HashMap::new();
    let mut by_name: HashMap<String, Vec<&Repository>> = HashMap::new();

    for repo in &config.repositories {
        // Group by URL (if present)
        if let Some(url) = &repo.url {
            let normalized_url = normalize_git_url(url);
            by_url.entry(normalized_url).or_default().push(repo);
        }

        // Group by normalized path
        let full_path = workspace_root.join(&repo.path);
        let path_key = normalize_path_for_comparison(&full_path);
        by_path.entry(path_key).or_default().push(repo);

        // Group by name
        by_name.entry(repo.name.clone()).or_default().push(repo);
    }

    // Detect URL duplicates
    for (_url, repos) in by_url {
        if repos.len() > 1 {
            let recommended_action = determine_url_duplicate_action(&repos, workspace_root);
            duplicates.push(DuplicateRepository {
                repositories: repos.into_iter().cloned().collect(),
                conflict_type: DuplicateType::SameUrl,
                recommended_action,
            });
        }
    }

    // Detect path duplicates (but not already found in URL duplicates)
    for (_, repos) in by_path {
        if repos.len() > 1 {
            // Check if this is already covered by URL duplicates
            if !is_already_in_duplicates(&repos, &duplicates) {
                let recommended_action = determine_path_duplicate_action(&repos, workspace_root);
                duplicates.push(DuplicateRepository {
                    repositories: repos.into_iter().cloned().collect(),
                    conflict_type: DuplicateType::SamePath,
                    recommended_action,
                });
            }
        }
    }

    // Detect name duplicates (but only if they're not path/URL duplicates)
    for (_, repos) in by_name {
        if repos.len() > 1 && !is_already_in_duplicates(&repos, &duplicates) {
            duplicates.push(DuplicateRepository {
                repositories: repos.into_iter().cloned().collect(),
                conflict_type: DuplicateType::SameName,
                recommended_action: RecommendedAction::ManualReview,
            });
        }
    }

    // Generate warnings
    for repo in &config.repositories {
        if repo.url.is_none() {
            warnings.push(format!("Repository '{}' has no URL", repo.name));
        }

        if repo.path.is_absolute() {
            warnings.push(format!(
                "Repository '{}' uses absolute path (consider relative)",
                repo.name
            ));
        }
    }

    let unique_count = calculate_unique_repositories(&config.repositories);

    Ok(ValidationReport {
        duplicates,
        warnings,
        total_repositories: config.repositories.len(),
        unique_repositories: unique_count,
    })
}

pub fn deduplicate_config(
    config: &mut WorkspaceConfig,
    workspace_root: &Path,
) -> Result<ValidationReport> {
    let report = validate_config(config, workspace_root)?;

    if report.duplicates.is_empty() {
        return Ok(report);
    }

    let mut to_remove = HashSet::new();

    for duplicate in &report.duplicates {
        let keep_names = match duplicate.recommended_action {
            RecommendedAction::KeepFirst => {
                vec![duplicate.repositories[0].name.clone()]
            }
            RecommendedAction::KeepMostComplete => {
                let best_repo = find_most_complete_repository(&duplicate.repositories);
                vec![best_repo.name.clone()]
            }
            RecommendedAction::KeepExisting => {
                let repo_refs: Vec<&Repository> = duplicate.repositories.iter().collect();
                let existing = find_existing_repositories(&repo_refs, workspace_root);
                existing.iter().map(|r| r.name.clone()).collect()
            }
            RecommendedAction::ManualReview => {
                // For manual review, keep the first one for now
                vec![duplicate.repositories[0].name.clone()]
            }
        };

        // Mark others for removal
        for repo in &duplicate.repositories {
            if !keep_names.contains(&repo.name) {
                to_remove.insert(repo.name.clone());
            }
        }
    }

    // Remove duplicates
    config
        .repositories
        .retain(|repo| !to_remove.contains(&repo.name));

    // Generate new report after deduplication
    validate_config(config, workspace_root)
}

fn normalize_git_url(url: &str) -> String {
    // Normalize git URLs for comparison
    let mut normalized = url.to_lowercase();

    // Remove trailing .git
    if normalized.ends_with(".git") {
        normalized = normalized[..normalized.len() - 4].to_string();
    }

    // Normalize SSH vs HTTPS
    if normalized.starts_with("git@github.com:") {
        normalized = normalized.replace("git@github.com:", "https://github.com/");
    }
    if normalized.starts_with("git@gitlab.com:") {
        normalized = normalized.replace("git@gitlab.com:", "https://gitlab.com/");
    }

    // Remove trailing slash
    normalized.trim_end_matches('/').to_string()
}

fn normalize_path_for_comparison(path: &Path) -> String {
    // Try to canonicalize, fall back to string representation
    match path.canonicalize() {
        Ok(canonical) => canonical.to_string_lossy().to_string(),
        Err(_) => path.to_string_lossy().to_string(),
    }
}

fn determine_url_duplicate_action(
    repos: &[&Repository],
    workspace_root: &Path,
) -> RecommendedAction {
    // Check which ones exist on filesystem
    let existing_repos = find_existing_repositories(repos, workspace_root);

    if existing_repos.len() == 1 {
        RecommendedAction::KeepExisting
    } else if existing_repos.is_empty() {
        RecommendedAction::KeepMostComplete
    } else {
        RecommendedAction::ManualReview
    }
}

fn determine_path_duplicate_action(
    repos: &[&Repository],
    workspace_root: &Path,
) -> RecommendedAction {
    // For path duplicates, keep the one with the most complete info
    let existing_repos = find_existing_repositories(repos, workspace_root);

    if existing_repos.len() == 1 {
        RecommendedAction::KeepExisting
    } else {
        RecommendedAction::KeepMostComplete
    }
}

fn find_existing_repositories<'a>(
    repos: &'a [&'a Repository],
    workspace_root: &Path,
) -> Vec<&'a Repository> {
    repos
        .iter()
        .filter(|repo| {
            let full_path = workspace_root.join(&repo.path);
            full_path.join(".git").exists()
        })
        .copied()
        .collect()
}

fn find_most_complete_repository(repos: &[Repository]) -> &Repository {
    repos
        .iter()
        .max_by_key(|repo| {
            let mut score = 0;
            if repo.url.is_some() {
                score += 3;
            }
            if repo.branch.is_some() {
                score += 1;
            }
            if !repo.apps.is_empty() {
                score += 2;
            }
            if !repo.path.as_os_str().is_empty() {
                score += 1;
            }
            score
        })
        .unwrap_or(&repos[0])
}

fn is_already_in_duplicates(repos: &[&Repository], duplicates: &[DuplicateRepository]) -> bool {
    let repo_names: HashSet<_> = repos.iter().map(|r| &r.name).collect();

    duplicates.iter().any(|duplicate| {
        let duplicate_names: HashSet<_> = duplicate.repositories.iter().map(|r| &r.name).collect();
        !repo_names.is_disjoint(&duplicate_names)
    })
}

fn calculate_unique_repositories(repos: &[Repository]) -> usize {
    let mut unique_urls = HashSet::new();

    for repo in repos {
        if let Some(url) = &repo.url {
            unique_urls.insert(normalize_git_url(url));
        } else {
            // If no URL, use path as identifier
            unique_urls.insert(repo.path.to_string_lossy().to_string());
        }
    }

    unique_urls.len()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_normalize_git_url() {
        assert_eq!(
            normalize_git_url("https://github.com/user/repo.git"),
            "https://github.com/user/repo"
        );
        assert_eq!(
            normalize_git_url("git@github.com:user/repo.git"),
            "https://github.com/user/repo"
        );
        assert_eq!(
            normalize_git_url("https://github.com/user/repo/"),
            "https://github.com/user/repo"
        );
    }

    #[test]
    fn test_find_most_complete_repository() {
        let repo1 = Repository::new("test1".to_string(), PathBuf::from("path1"));
        let repo2 = Repository::new("test2".to_string(), PathBuf::from("path2"))
            .with_url("https://github.com/test/repo".to_string())
            .with_branch("main".to_string());

        let repos = vec![repo1, repo2];
        let most_complete = find_most_complete_repository(&repos);

        assert_eq!(most_complete.name, "test2");
    }
}
