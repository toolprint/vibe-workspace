use anyhow::Result;
use console::style;
use dialoguer::{theme::ColorfulTheme, Select};
use inquire::Text;

use super::provider::{ProviderFactory, SearchProvider};
use super::{CloneCommand, GitConfig, Repository, SearchQuery};
use crate::workspace::manager::WorkspaceManager;

pub struct SearchEngine {
    providers: Vec<Box<dyn SearchProvider>>,
}

impl SearchEngine {
    pub fn new(config: &GitConfig) -> Result<Self> {
        let mut providers = Vec::new();

        for provider_name in &config.search_providers {
            match ProviderFactory::create_provider(provider_name) {
                Ok(provider) => providers.push(provider),
                Err(e) => eprintln!(
                    "Warning: Failed to create provider '{}': {}",
                    provider_name, e
                ),
            }
        }

        if providers.is_empty() {
            anyhow::bail!("No search providers available");
        }

        Ok(Self { providers })
    }

    pub fn add_provider(&mut self, provider: Box<dyn SearchProvider>) {
        self.providers.push(provider);
    }

    pub async fn search(&self, query: &SearchQuery) -> Result<Vec<Repository>> {
        let mut all_results = Vec::new();

        for provider in &self.providers {
            match provider.search(query).await {
                Ok(results) => all_results.extend(results),
                Err(e) => eprintln!(
                    "Warning: Search failed for provider '{}': {}",
                    provider.name(),
                    e
                ),
            }
        }

        // Remove duplicates based on full_name
        all_results.sort_by(|a, b| a.full_name.cmp(&b.full_name));
        all_results.dedup_by(|a, b| a.full_name == b.full_name);

        Ok(all_results)
    }
}

pub struct SearchCommand;

impl SearchCommand {
    pub async fn execute_interactive(
        workspace_manager: &mut WorkspaceManager,
        config: &GitConfig,
    ) -> Result<()> {
        println!(
            "\n{} {} {}",
            style("🔍").blue(),
            style("GitHub Repository Search").cyan().bold(),
            style("- Find and clone repositories").dim()
        );

        // Get search query from user
        let query = Text::new("\nSearch GitHub repositories:")
            .with_placeholder("e.g., rust web framework")
            .prompt()?;

        if query.trim().is_empty() {
            println!("{} Search cancelled", style("❌").red());
            return Ok(());
        }

        println!("\n{} Searching repositories...", style("🔍").blue());

        let search_query = SearchQuery {
            keywords: query.split_whitespace().map(|s| s.to_string()).collect(),
            tags: vec![],
            language: None,
            organization: None,
            limit: Some(20),          // Show more results
            sort: Default::default(), // Uses BestMatch by default
        };

        let engine = SearchEngine::new(config)?;
        let results = engine.search(&search_query).await?;

        if results.is_empty() {
            println!(
                "{} No repositories found for '{}'",
                style("❌").red(),
                query
            );
            return Ok(());
        }

        println!(
            "\n{} {} {} {}",
            style("📦").green(),
            style("Found").green().bold(),
            style(format!("{} repositories", results.len())).dim(),
            style(format!("(sorted by: {})", search_query.sort.display_name())).dim()
        );

        // Display and select repository
        let selected_repo = Self::display_interactive_results(&results)?;

        if let Some(repo) = selected_repo {
            // Clone the selected repository
            CloneCommand::clone_from_search_result(repo, workspace_manager, config).await?;
        } else {
            println!("{} No repository selected", style("ℹ️").blue());
        }

        Ok(())
    }

    fn display_interactive_results(results: &[Repository]) -> Result<Option<Repository>> {
        let items: Vec<String> = results
            .iter()
            .map(|repo| {
                let stars = if repo.stars > 0 {
                    format!("⭐ {} ", Self::format_stars(repo.stars))
                } else {
                    "".to_string()
                };

                let lang = if let Some(language) = &repo.language {
                    format!(" [{}]", language)
                } else {
                    "".to_string()
                };

                let license = if let Some(license_key) = &repo.license {
                    format!(" [{}]", license_key.to_uppercase())
                } else {
                    "".to_string()
                };

                let desc = repo.description.as_deref().unwrap_or("No description");
                let truncated_desc = if desc.chars().count() > 40 {
                    let truncated: String = desc.chars().take(40).collect();
                    format!("{}...", truncated)
                } else {
                    desc.to_string()
                };

                format!(
                    "{}{}{}{} - {}",
                    stars, repo.full_name, lang, license, truncated_desc
                )
            })
            .collect();

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select repository to clone (ESC to cancel)")
            .items(&items)
            .default(0)
            .interact_opt()?;

        Ok(selection.map(|i| results[i].clone()))
    }

    fn format_stars(count: u32) -> String {
        if count >= 1000 {
            format!("{:.1}k", count as f32 / 1000.0)
        } else {
            count.to_string()
        }
    }
}
