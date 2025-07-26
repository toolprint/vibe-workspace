use anyhow::Result;
use console::style;
use inquire::{MultiSelect, Text};

use super::app_manager::AppManager;

/// Run the interactive app installer
pub async fn run_interactive_installer() -> Result<()> {
    println!(
        "\n{} {} {}",
        style("🚀").blue(),
        style("Vibe App Installer").cyan().bold(),
        style("- Install developer tools and applications").dim()
    );

    // Initialize app manager
    let app_manager = AppManager::new().await?;

    // Check available package managers
    let available_managers = app_manager.get_available_managers();
    if available_managers.is_empty() {
        anyhow::bail!("No package managers available. Please install Homebrew or Cargo.");
    }

    println!("\n{} Checking installed apps...", style("🔍").blue());

    // Get status of all apps
    let mut statuses = app_manager.get_all_status().await?;
    statuses.sort_by(|a, b| a.app_name.cmp(&b.app_name));

    // Separate installed and available apps
    let (installed, available): (Vec<_>, Vec<_>) = statuses.iter().partition(|s| s.installed);

    // Display installed apps
    if !installed.is_empty() {
        println!(
            "\n{} {} {}",
            style("✅").green(),
            style("Already Installed").green().bold(),
            style(format!("({})", installed.len())).dim()
        );

        for status in &installed {
            let app = app_manager
                .list_available()
                .iter()
                .find(|a| a.name == status.app_name)
                .unwrap();

            let version_str = status
                .version
                .as_ref()
                .map(|v| format!(" ({v})"))
                .unwrap_or_default();

            // Show warning if installed but not managed by a package manager
            let indicator = if !status.is_managed {
                style("⚠️").yellow()
            } else {
                style("•").dim()
            };

            let mut line = format!(
                "  {} {}{}",
                indicator,
                style(&app.display_name).green(),
                style(&version_str).dim()
            );

            // Add path info for unmanaged apps
            if !status.is_managed {
                if let Some(path) = &status.installed_path {
                    line.push_str(&format!(" - {}", style(path).dim()));
                }
            }

            println!("{line}");
        }
    }

    // Check if any apps are available to install
    if available.is_empty() {
        println!(
            "\n{} All available apps are already installed!",
            style("🎉").green()
        );
        return Ok(());
    }

    // Prepare options for multi-select
    let mut options = Vec::new();
    let mut app_map = std::collections::HashMap::new();

    for status in &available {
        if !status.available_managers.is_empty() {
            let app = app_manager
                .list_available()
                .iter()
                .find(|a| a.name == status.app_name)
                .unwrap();

            let option = format!("{} - {}", app.display_name, app.description);
            options.push(option.clone());
            app_map.insert(option, &app.name);
        }
    }

    if options.is_empty() {
        println!(
            "\n{} No apps available to install with current package managers.",
            style("⚠️").yellow()
        );
        return Ok(());
    }

    // Show multi-select prompt
    println!(
        "\n{} {} {}",
        style("📦").blue(),
        style("Available to Install").blue().bold(),
        style(format!("({})", options.len())).dim()
    );

    let selected = MultiSelect::new(
        "Select apps to install (use Space to select, Enter to confirm):",
        options.clone(),
    )
    .with_page_size(15)
    .prompt()?;

    if selected.is_empty() {
        println!(
            "\n{} No apps selected for installation.",
            style("ℹ️").blue()
        );
        return Ok(());
    }

    // Confirm installation
    println!(
        "\n{} Selected {} app(s) for installation:",
        style("📋").blue(),
        style(selected.len()).cyan()
    );

    let mut app_names = Vec::new();
    for selection in &selected {
        if let Some(app_name) = app_map.get(selection) {
            app_names.push((*app_name).to_string());
            let app = app_manager
                .list_available()
                .iter()
                .find(|a| a.name == **app_name)
                .unwrap();
            println!("  {} {}", style("•").dim(), style(&app.display_name).cyan());
        }
    }

    let confirm = Text::new("\nProceed with installation? (Y/n)")
        .with_default("y")
        .prompt()?;

    if confirm.to_lowercase() == "n" {
        println!("{} Installation cancelled.", style("❌").red());
        return Ok(());
    }

    // Install selected apps
    println!("\n{} Starting installation...", style("🚀").blue());
    app_manager.install_multiple(&app_names).await?;

    println!(
        "\n{} Done! Your selected apps have been installed.",
        style("🎆").green()
    );

    Ok(())
}
