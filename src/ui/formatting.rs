use chrono::{Duration, Utc};
use console::style;

use crate::workspace::operations::GitStatus;

/// Shared formatting utilities for consistent display across UI components
/// This module provides unified color schemes and formatting patterns for repository items

/// Format a timestamp as human-readable time ago
pub fn format_time_ago(timestamp: &chrono::DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(timestamp);

    if duration < Duration::minutes(1) {
        "just now".to_string()
    } else if duration < Duration::hours(1) {
        let mins = duration.num_minutes();
        format!("{} min{} ago", mins, if mins == 1 { "" } else { "s" })
    } else if duration < Duration::days(1) {
        let hours = duration.num_hours();
        format!("{} hour{} ago", hours, if hours == 1 { "" } else { "s" })
    } else if duration < Duration::days(7) {
        let days = duration.num_days();
        format!("{} day{} ago", days, if days == 1 { "" } else { "s" })
    } else if duration < Duration::days(30) {
        let weeks = duration.num_weeks();
        format!("{} week{} ago", weeks, if weeks == 1 { "" } else { "s" })
    } else {
        let months = duration.num_days() / 30;
        format!("{} month{} ago", months, if months == 1 { "" } else { "s" })
    }
}

/// Get repository name color based on git status
pub fn get_repo_name_color(name: &str, git_status: Option<&GitStatus>) -> String {
    if let Some(status) = git_status {
        if status.remote_url.is_none() {
            style(name).red().bold().to_string()
        } else if !status.clean {
            style(name).yellow().bold().to_string()
        } else {
            style(name).green().bold().to_string()
        }
    } else {
        style(name).green().bold().to_string()
    }
}

/// Format app indicator with arrow notation for single apps, list for multiple
pub fn format_app_indicator(apps: &[String], last_app: Option<&str>) -> String {
    match apps.len() {
        0 => "".to_string(),
        1 => format!("→ {}", style(&apps[0]).blue()),
        _ => {
            if let Some(last) = last_app {
                // Show the last used app with arrow, others in parentheses
                let other_apps: Vec<&str> = apps
                    .iter()
                    .map(|s| s.as_str())
                    .filter(|&app| app != last)
                    .collect();
                if other_apps.is_empty() {
                    format!("→ {}", style(last).blue())
                } else {
                    format!(
                        "→ {} ({})",
                        style(last).blue(),
                        style(other_apps.join(", ")).dim()
                    )
                }
            } else {
                format!("(apps: {})", style(apps.join(", ")).blue())
            }
        }
    }
}

/// Format git status indicators
pub fn format_git_status_indicators(git_status: &GitStatus) -> String {
    if git_status.clean {
        return "".to_string();
    }

    let mut indicators = Vec::new();
    if git_status.staged > 0 {
        indicators.push(format!("{}S", git_status.staged));
    }
    if git_status.unstaged > 0 {
        indicators.push(format!("{}M", git_status.unstaged));
    }
    if git_status.untracked > 0 {
        indicators.push(format!("{}?", git_status.untracked));
    }
    if git_status.ahead > 0 {
        indicators.push(format!("↑{}", git_status.ahead));
    }
    if git_status.behind > 0 {
        indicators.push(format!("↓{}", git_status.behind));
    }

    if indicators.is_empty() {
        "".to_string()
    } else {
        format!("[{}]", style(indicators.join(" ")).yellow())
    }
}

/// Format branch information
pub fn format_branch_info(git_status: Option<&GitStatus>) -> String {
    if let Some(status) = git_status {
        if let Some(ref branch) = status.branch {
            format!("on {}", style(branch).white().bold())
        } else {
            "".to_string()
        }
    } else {
        "".to_string()
    }
}

/// Format repository item for main menu quick launch (with time and simple app display)
pub fn format_repository_quick_launch(
    number: usize,
    repo_name: &str,
    last_accessed: &str,
    last_app: Option<&str>,
    git_status: Option<&GitStatus>,
) -> String {
    let mut parts = Vec::new();

    // Number indicator
    parts.push(format!("{}.", style(number).cyan().bold()));

    // Repository name with git status color
    parts.push(get_repo_name_color(repo_name, git_status));

    // Time indicator
    parts.push(style(format!("({})", last_accessed)).dim().to_string());

    // App indicator (arrow notation for single app)
    if let Some(app) = last_app {
        parts.push(format!("→ {}", style(app).blue()));
    }

    parts.join(" ")
}

/// Format repository item for launch menu (with git status, apps, and optional time)
pub fn format_repository_launch_item(
    name: &str,
    apps: &[String],
    git_status: Option<&GitStatus>,
    recent_rank: Option<usize>,
    last_accessed: Option<&str>,
    last_app: Option<&str>,
) -> String {
    let mut parts = Vec::new();

    // Add recent rank indicator if present
    if let Some(rank) = recent_rank {
        parts.push(format!("{}.", style(rank).cyan().bold()));
    }

    // Repository name with git status color
    parts.push(get_repo_name_color(name, git_status));

    // For recent items, show time like main menu
    if recent_rank.is_some() {
        if let Some(time) = last_accessed {
            parts.push(style(format!("({})", time)).dim().to_string());
        }
    }

    // Git status indicators (if not clean and available)
    if let Some(status) = git_status {
        let status_indicators = format_git_status_indicators(status);
        if !status_indicators.is_empty() {
            parts.push(status_indicators);
        }

        // Branch information
        let branch_info = format_branch_info(Some(status));
        if !branch_info.is_empty() {
            parts.push(branch_info);
        }
    }

    // App indicator - use arrow notation for recent items with last app, otherwise list all
    if recent_rank.is_some() && last_app.is_some() {
        // Recent item: show like main menu with arrow
        parts.push(format!("→ {}", style(last_app.unwrap()).blue()));

        // Show other apps if more than one
        if apps.len() > 1 {
            let other_apps: Vec<&str> = apps
                .iter()
                .map(|s| s.as_str())
                .filter(|&app| Some(app) != last_app)
                .collect();
            if !other_apps.is_empty() {
                parts.push(format!("(+{})", style(other_apps.join(", ")).dim()));
            }
        }
    } else {
        // Non-recent item: show all apps in parentheses
        if !apps.is_empty() {
            parts.push(format!("(apps: {})", style(apps.join(", ")).blue()));
        }
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_format_time_ago_minutes() {
        let now = Utc::now();
        let five_minutes_ago = now - Duration::minutes(5);
        assert_eq!(format_time_ago(&five_minutes_ago), "5 mins ago");
    }

    #[test]
    fn test_format_time_ago_hours() {
        let now = Utc::now();
        let two_hours_ago = now - Duration::hours(2);
        assert_eq!(format_time_ago(&two_hours_ago), "2 hours ago");
    }

    #[test]
    fn test_format_app_indicator_single() {
        let apps = vec!["vscode".to_string()];
        let result = format_app_indicator(&apps, None);
        // Note: We can't easily test styled output, but we can test the structure
        assert!(result.contains("→"));
        assert!(result.contains("vscode"));
    }

    #[test]
    fn test_format_app_indicator_multiple() {
        let apps = vec!["vscode".to_string(), "cursor".to_string()];
        let result = format_app_indicator(&apps, Some("vscode"));
        assert!(result.contains("→"));
        assert!(result.contains("vscode"));
        assert!(result.contains("cursor"));
    }
}
