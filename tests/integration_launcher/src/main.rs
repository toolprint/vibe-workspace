use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use colored::*;
use console::style;
use std::env;
use std::path::{Path, PathBuf};
use tracing::{info, warn, Level};
use tracing_subscriber;

mod assertions;
mod fixtures;
mod runner;

use runner::{TestRunner, TestResult};

#[derive(Parser)]
#[command(name = "vibe-integration-launcher")]
#[command(about = "Integration test launcher for vibe-workspace CLI")]
struct Cli {
    /// Test filter pattern (e.g., "basic_operations", "app_*")
    #[arg(long)]
    test_filter: Option<String>,

    /// Use release binary instead of debug
    #[arg(long)]
    release: bool,

    /// Show verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Keep temporary directories after tests (for debugging)
    #[arg(long)]
    keep_temp: bool,

    /// Override path to vibe binary
    #[arg(long)]
    binary_path: Option<PathBuf>,

    /// Run tests in parallel (default: sequential)
    #[arg(long)]
    parallel: bool,

    /// Output format
    #[arg(long, value_enum, default_value = "pretty")]
    output: OutputFormat,
}

#[derive(Debug, Clone, ValueEnum)]
enum OutputFormat {
    Pretty,
    Json,
    Tap,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    let log_level = if cli.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };
    
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .with_target(false)
        .without_time()
        .init();
    
    // Check environment variables
    let keep_temp = cli.keep_temp || env::var("VIBE_TEST_KEEP_TEMP").is_ok();
    let verbose = cli.verbose || env::var("VIBE_TEST_VERBOSE").is_ok();
    
    // Find vibe binary
    let binary_path = find_vibe_binary(&cli)?;
    info!("Using vibe binary: {}", binary_path.display());
    
    // Create test runner
    let mut runner = TestRunner::new(binary_path, keep_temp, verbose);
    
    // Discover tests
    let test_dir = find_test_directory()?;
    let tests = discover_tests(&test_dir, cli.test_filter.as_deref())?;
    
    if tests.is_empty() {
        warn!("No tests found matching filter");
        return Ok(());
    }
    
    info!("Found {} tests to run", tests.len());
    
    // Run tests
    let results = if cli.parallel {
        runner.run_parallel(&tests).await?
    } else {
        runner.run_sequential(&tests).await?
    };
    
    // Display results
    display_results(&results, &cli.output)?;
    
    // Exit with appropriate code
    let failed_count = results.iter().filter(|r| !r.success).count();
    if failed_count > 0 {
        std::process::exit(1);
    }
    
    Ok(())
}

fn find_vibe_binary(cli: &Cli) -> Result<PathBuf> {
    // Check for explicit override
    if let Some(path) = &cli.binary_path {
        if path.exists() {
            return Ok(path.clone());
        }
        anyhow::bail!("Specified binary path does not exist: {}", path.display());
    }
    
    // Check environment variable
    if let Ok(path) = env::var("VIBE_TEST_BINARY") {
        let path = PathBuf::from(path);
        if path.exists() {
            return Ok(path);
        }
        warn!("VIBE_TEST_BINARY path does not exist: {}", path.display());
    }
    
    // Find in target directory
    let workspace_root = find_workspace_root()?;
    let binary_name = if cfg!(windows) { "vibe.exe" } else { "vibe" };
    
    let target_dir = workspace_root.join("target");
    let possible_paths = if cli.release {
        vec![target_dir.join("release").join(binary_name)]
    } else {
        vec![
            target_dir.join("debug").join(binary_name),
            target_dir.join("release").join(binary_name),
        ]
    };
    
    for path in possible_paths {
        if path.exists() {
            return Ok(path);
        }
    }
    
    anyhow::bail!(
        "Could not find vibe binary. Please run 'cargo build{}' first.",
        if cli.release { " --release" } else { "" }
    )
}

fn find_workspace_root() -> Result<PathBuf> {
    let mut current = env::current_dir()?;
    
    loop {
        if current.join("Cargo.toml").exists() {
            // Check if this is the workspace root
            let content = std::fs::read_to_string(current.join("Cargo.toml"))?;
            if content.contains("vibe-workspace") && !content.contains("vibe-integration-launcher") {
                return Ok(current);
            }
        }
        
        if !current.pop() {
            break;
        }
    }
    
    anyhow::bail!("Could not find vibe-workspace root directory")
}

fn find_test_directory() -> Result<PathBuf> {
    let workspace_root = find_workspace_root()?;
    let test_dir = workspace_root.join("tests").join("integration");
    
    if !test_dir.exists() {
        anyhow::bail!("Test directory does not exist: {}", test_dir.display());
    }
    
    Ok(test_dir)
}

fn discover_tests(test_dir: &Path, filter: Option<&str>) -> Result<Vec<PathBuf>> {
    let pattern = format!("{}/**/*.rs", test_dir.display());
    let mut tests = Vec::new();
    
    for entry in glob::glob(&pattern)? {
        let path = entry?;
        let file_name = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        // Skip module files
        if file_name == "mod" {
            continue;
        }
        
        // Apply filter if provided
        if let Some(filter) = filter {
            if !file_name.contains(filter) && !glob::Pattern::new(filter)?.matches(file_name) {
                continue;
            }
        }
        
        tests.push(path);
    }
    
    tests.sort();
    Ok(tests)
}

fn display_results(results: &[TestResult], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Pretty => display_pretty_results(results),
        OutputFormat::Json => display_json_results(results),
        OutputFormat::Tap => display_tap_results(results),
    }
}

fn display_pretty_results(results: &[TestResult]) -> Result<()> {
    println!("\n{}", style("Test Results").bold().underlined());
    println!();
    
    let mut passed = 0;
    let mut failed = 0;
    
    for result in results {
        let status = if result.success {
            passed += 1;
            format!("{}", "PASS".green().bold())
        } else {
            failed += 1;
            format!("{}", "FAIL".red().bold())
        };
        
        let duration = format!("({}ms)", result.duration.as_millis());
        println!("{} {} {}", status, result.test_name, duration.dimmed());
        
        if !result.success {
            if let Some(output) = &result.output {
                println!("  {}", "Output:".yellow());
                for line in output.lines() {
                    println!("    {}", line);
                }
            }
            if let Some(error) = &result.error {
                println!("  {}: {}", "Error".red(), error);
            }
        }
    }
    
    println!();
    println!("{}", style("Summary").bold());
    println!("  {} passed", format!("{}", passed).green());
    println!("  {} failed", format!("{}", failed).red());
    println!("  {} total", passed + failed);
    
    Ok(())
}

fn display_json_results(results: &[TestResult]) -> Result<()> {
    let json = serde_json::to_string_pretty(results)?;
    println!("{}", json);
    Ok(())
}

fn display_tap_results(results: &[TestResult]) -> Result<()> {
    println!("TAP version 13");
    println!("1..{}", results.len());
    
    for (i, result) in results.iter().enumerate() {
        if result.success {
            println!("ok {} - {}", i + 1, result.test_name);
        } else {
            println!("not ok {} - {}", i + 1, result.test_name);
            if let Some(error) = &result.error {
                for line in error.lines() {
                    println!("  # {}", line);
                }
            }
        }
    }
    
    Ok(())
}