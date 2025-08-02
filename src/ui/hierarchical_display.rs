use console::style;
use std::collections::HashMap;

use crate::workspace::operations::get_git_status;
use crate::workspace::repo_analyzer::{NonGitFolder, RepoInfo, RepoStatus, WorkspaceAnalysis};

pub struct DisplayOptions {
    pub show_paths: bool,
    pub show_urls: bool,
    pub compact: bool,
}

impl Default for DisplayOptions {
    fn default() -> Self {
        Self {
            show_paths: true,
            show_urls: false,
            compact: false,
        }
    }
}

pub fn render_workspace_analysis(analysis: &WorkspaceAnalysis, options: &DisplayOptions) {
    let total_repos = analysis.repositories.len();
    let tracked_count = analysis.get_tracked_repos().len();
    let new_count = analysis.get_new_repos().len();
    let missing_count = analysis.get_missing_repos().len();
    let nongit_count = analysis.non_git_folders.len();

    // Header with summary
    println!("{} Workspace Analysis", style("📊").blue().bold());
    println!("{}", "─".repeat(50));

    println!(
        "Total repositories: {} | Tracked: {} | New: {} | Missing: {}",
        style(total_repos).bold(),
        if tracked_count > 0 {
            style(tracked_count).green()
        } else {
            style(tracked_count).dim()
        },
        if new_count > 0 {
            style(new_count).yellow()
        } else {
            style(new_count).dim()
        },
        if missing_count > 0 {
            style(missing_count).red()
        } else {
            style(missing_count).dim()
        }
    );

    if nongit_count > 0 {
        println!("Non-git folders: {}", style(nongit_count).cyan());
    }

    println!();

    // Render repositories by organization
    render_repositories_by_organization(&analysis.organizations, options);

    // Render non-git folders if any
    if !analysis.non_git_folders.is_empty() {
        render_non_git_folders(&analysis.non_git_folders, options);
    }

    // Show actionable summary
    if analysis.has_actionable_items() {
        render_actionable_summary(analysis);
    }
}

fn render_repositories_by_organization(
    organizations: &HashMap<String, Vec<RepoInfo>>,
    options: &DisplayOptions,
) {
    let mut org_names: Vec<_> = organizations.keys().collect();
    org_names.sort();

    for org_name in org_names {
        let repos = &organizations[org_name];

        // Organization header
        println!(
            "{} {} ({})",
            style("📁").blue(),
            style(org_name).cyan().bold(),
            style(format!("{} repos", repos.len())).dim()
        );

        for repo in repos {
            render_repository_entry(repo, options);
        }

        println!(); // Add space between organizations
    }
}

fn render_repository_entry(repo: &RepoInfo, options: &DisplayOptions) {
    let status_icon = match repo.status {
        RepoStatus::Tracked => style("✅").to_string(),
        RepoStatus::New => style("🆕").to_string(),
        RepoStatus::Missing => style("❌").to_string(),
    };

    let repo_name = match repo.status {
        RepoStatus::Tracked => style(&repo.name).green(),
        RepoStatus::New => style(&repo.name).yellow(),
        RepoStatus::Missing => style(&repo.name).red(),
    };

    print!("  {} {}", status_icon, repo_name);

    if options.show_paths && !options.compact {
        print!(" {}", style(format!("({})", repo.path.display())).dim());
    }

    if options.show_urls && repo.remote_url.is_some() {
        print!(" {}", style(repo.remote_url.as_ref().unwrap()).dim());
    }

    println!();
}

fn render_non_git_folders(folders: &[NonGitFolder], options: &DisplayOptions) {
    println!(
        "{} Non-Git Folders ({})",
        style("📁").cyan().bold(),
        style(folders.len()).dim()
    );

    for folder in folders {
        print!("  {} {}", style("📁").cyan(), style(&folder.name).cyan());

        if options.show_paths && !options.compact {
            print!(" {}", style(format!("({})", folder.path.display())).dim());
        }

        println!();
    }

    println!();
}

fn render_actionable_summary(analysis: &WorkspaceAnalysis) {
    println!("{} Actionable Items", style("💡").yellow().bold());
    println!("{}", "─".repeat(30));

    let new_repos = analysis.get_new_repos();
    let missing_repos = analysis.get_missing_repos();

    if !new_repos.is_empty() {
        println!(
            "• {} new repositories found - use {} to add them to config",
            style(new_repos.len()).yellow().bold(),
            style("--import").green()
        );
    }

    if !missing_repos.is_empty() {
        println!(
            "• {} repositories missing from filesystem - use {} to re-clone or {} to remove from config",
            style(missing_repos.len()).red().bold(),
            style("--restore").green(),
            style("--clean").green()
        );
    }

    if !analysis.non_git_folders.is_empty() {
        println!(
            "• {} non-git folders found - consider moving to appropriate locations",
            style(analysis.non_git_folders.len()).cyan().bold()
        );
    }

    println!();
}

pub fn render_repository_status_table(repos: &[RepoInfo], title: &str) {
    if repos.is_empty() {
        return;
    }

    println!(
        "{} {} ({})",
        style("📊").blue(),
        style(title).bold(),
        style(repos.len()).dim()
    );
    println!("{}", "─".repeat(50));

    // Table header
    println!(
        "{:<30} {:<20} {:<15}",
        style("Repository").bold().underlined(),
        style("Organization").bold().underlined(),
        style("Status").bold().underlined()
    );

    for repo in repos {
        let status_text = match repo.status {
            RepoStatus::Tracked => style("Tracked").green(),
            RepoStatus::New => style("New").yellow(),
            RepoStatus::Missing => style("Missing").red(),
        };

        let org_name = repo.organization.as_deref().unwrap_or("Other");

        println!(
            "{:<30} {:<20} {}",
            style(&repo.name).cyan(),
            style(org_name).dim(),
            status_text
        );
    }

    println!();
}

// Format for status command output - hierarchical with detailed git status
pub async fn render_status_summary(analysis: &WorkspaceAnalysis) {
    let tracked_repos = analysis.get_tracked_repos();

    if tracked_repos.is_empty() {
        println!("{} No repositories found", style("ℹ").yellow());
        return;
    }

    println!("{} Repository Status Summary", style("📊").blue().bold());
    println!("{}", "─".repeat(50));

    // Group by organization for status display
    let mut org_groups: HashMap<String, Vec<&RepoInfo>> = HashMap::new();
    for repo in &tracked_repos {
        let org_name = repo.organization.as_deref().unwrap_or("Other").to_string();
        org_groups
            .entry(org_name)
            .or_insert_with(Vec::new)
            .push(repo);
    }

    let mut org_names: Vec<_> = org_groups.keys().collect();
    org_names.sort();

    let mut total_clean = 0;
    let mut total_dirty = 0;
    let mut total_no_remote = 0;

    for org_name in org_names {
        let repos = &org_groups[org_name];

        println!(
            "{} {} ({})",
            style("📁").blue(),
            style(org_name).cyan().bold(),
            style(format!("{} repos", repos.len())).dim()
        );

        for repo in repos {
            // Get detailed git status for each repository
            match get_git_status(&repo.path).await {
                Ok(status) => {
                    if status.clean {
                        total_clean += 1;
                    } else {
                        total_dirty += 1;
                    }

                    // Count repositories without remotes
                    if status.remote_url.is_none() {
                        total_no_remote += 1;
                    }

                    // Format the detailed status line similar to the original
                    let mut status_parts = Vec::new();

                    // Repository name - color by git status (red=no remote, yellow=changes, green=clean)
                    let name_style = if status.remote_url.is_none() {
                        style(&repo.name).red().bold()
                    } else if !status.clean {
                        style(&repo.name).yellow().bold()
                    } else {
                        style(&repo.name).green().bold()
                    };
                    let name_part = format!("  {}", name_style);

                    // Branch information with ahead/behind indicators
                    if let Some(ref branch) = status.branch {
                        let branch_display = if status.ahead > 0 || status.behind > 0 {
                            format!("{} [↑{} ↓{}]", branch, status.ahead, status.behind)
                        } else {
                            branch.to_string()
                        };
                        status_parts.push(format!("on {}", style(branch_display).white().bold()));
                    }

                    // Status indicators
                    let mut indicators = Vec::new();
                    if status.clean {
                        indicators.push(style("✓").green().to_string());
                    } else {
                        if status.staged > 0 {
                            indicators.push(format!("{}S", style(status.staged).green()));
                        }
                        if status.unstaged > 0 {
                            indicators.push(format!("{}M", style(status.unstaged).red()));
                        }
                        if status.untracked > 0 {
                            indicators.push(format!("{}?", style(status.untracked).yellow()));
                        }
                    }

                    if !indicators.is_empty() {
                        status_parts.push(format!("[{}]", indicators.join(" ")));
                    }

                    // Print the complete status line
                    if status_parts.is_empty() {
                        println!("{}", name_part);
                    } else {
                        println!("{} {}", name_part, status_parts.join(" "));
                    }
                }
                Err(e) => {
                    // Handle repositories that can't be analyzed (e.g., not git repos, permission issues)
                    println!(
                        "  {} {} {}",
                        style("⚠").yellow(),
                        style(&repo.name).cyan().bold(),
                        style(format!("({})", e)).dim()
                    );
                }
            }
        }

        println!();
    }

    // Summary
    println!(
        "{} {} clean, {} with changes, {} no remote",
        style("📊").blue(),
        style(total_clean).green(),
        style(total_dirty).red(),
        style(total_no_remote).yellow()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::repo_analyzer::WorkspaceAnalysis;

    #[test]
    fn test_display_options_default() {
        let options = DisplayOptions::default();
        assert!(options.show_paths);
        assert!(!options.show_urls);
        assert!(!options.compact);
    }

    #[test]
    fn test_empty_analysis_display() {
        let analysis = WorkspaceAnalysis::new();
        let options = DisplayOptions::default();

        // This shouldn't panic
        render_workspace_analysis(&analysis, &options);
    }

    #[tokio::test]
    async fn test_status_summary_with_empty_repos() {
        let analysis = WorkspaceAnalysis::new();
        render_status_summary(&analysis).await;
    }
}
