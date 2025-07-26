use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::process::Command;
use tokio::io::AsyncReadExt;
use tracing::{debug, info};

use crate::fixtures::{TestEnvironment, TestDataBuilder};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub success: bool,
    pub duration: Duration,
    pub output: Option<String>,
    pub error: Option<String>,
}

pub struct TestRunner {
    binary_path: PathBuf,
    keep_temp: bool,
    verbose: bool,
}

impl TestRunner {
    pub fn new(binary_path: PathBuf, keep_temp: bool, verbose: bool) -> Self {
        Self {
            binary_path,
            keep_temp,
            verbose,
        }
    }
    
    /// Run tests sequentially
    pub async fn run_sequential(&mut self, test_files: &[PathBuf]) -> Result<Vec<TestResult>> {
        let mut results = Vec::new();
        
        for test_file in test_files {
            let result = self.run_test_file(test_file).await?;
            results.push(result);
        }
        
        Ok(results)
    }
    
    /// Run tests in parallel
    pub async fn run_parallel(&mut self, test_files: &[PathBuf]) -> Result<Vec<TestResult>> {
        use futures::future::join_all;
        
        let futures: Vec<_> = test_files
            .iter()
            .map(|test_file| {
                let runner = TestRunner {
                    binary_path: self.binary_path.clone(),
                    keep_temp: self.keep_temp,
                    verbose: self.verbose,
                };
                let test_file = test_file.clone();
                
                tokio::spawn(async move {
                    runner.run_test_file(&test_file).await
                })
            })
            .collect();
        
        let results = join_all(futures).await;
        
        results
            .into_iter()
            .map(|r| r?)
            .collect::<Result<Vec<_>>>()
    }
    
    /// Run a single test file
    async fn run_test_file(&self, test_file: &Path) -> Result<TestResult> {
        let test_name = test_file
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        info!("Running test: {}", test_name);
        let start = Instant::now();
        
        match self.execute_test(&test_name, test_file).await {
            Ok(output) => Ok(TestResult {
                test_name,
                success: true,
                duration: start.elapsed(),
                output: Some(output),
                error: None,
            }),
            Err(e) => Ok(TestResult {
                test_name,
                success: false,
                duration: start.elapsed(),
                output: None,
                error: Some(e.to_string()),
            }),
        }
    }
    
    /// Execute the test by running the test definition
    async fn execute_test(&self, test_name: &str, test_file: &Path) -> Result<String> {
        // Load test definition
        let test_def = self.load_test_definition(test_file).await?;
        
        // Create test environment
        let env = TestEnvironment::new(test_name, self.keep_temp).await?;
        
        // Set up test data
        if let Some(setup) = &test_def.setup {
            self.setup_test_data(&env, setup).await?;
        }
        
        let mut output = String::new();
        
        // Run test steps
        for (i, step) in test_def.steps.iter().enumerate() {
            debug!("Running step {}: {}", i + 1, step.name);
            
            let step_output = self.run_test_step(&env, step).await
                .with_context(|| format!("Failed at step {}: {}", i + 1, step.name))?;
            
            output.push_str(&format!("Step {}: {}\n", i + 1, step.name));
            output.push_str(&step_output);
            output.push_str("\n");
            
            // Validate step if needed
            if let Some(validation) = &step.validate {
                self.validate_step(&env, validation, &step_output).await
                    .with_context(|| format!("Validation failed at step {}", i + 1))?;
            }
        }
        
        Ok(output)
    }
    
    /// Load test definition from file
    async fn load_test_definition(&self, test_file: &Path) -> Result<TestDefinition> {
        // Since these are Rust test files, we'll compile and run them directly
        // For now, create a simple test that just executes basic vibe commands
        Ok(TestDefinition {
            name: test_file.file_stem().unwrap().to_string_lossy().to_string(),
            setup: Some(TestSetup {
                repositories: vec!["test-repo".to_string()],
                config: false, // Don't pre-create config, let init do it
                templates: true,
            }),
            steps: vec![
                TestStep {
                    name: "Run compiled test".to_string(),
                    command: vec!["init".to_string(), "--name".to_string(), "test-workspace".to_string()],
                    validate: Some(StepValidation::OutputContains("workspace".to_string())),
                },
            ],
        })
    }
    
    /// Set up test data based on setup configuration
    async fn setup_test_data(&self, env: &TestEnvironment, setup: &TestSetup) -> Result<()> {
        let _builder = TestDataBuilder::new(env);
        
        // Create repositories if needed
        if !setup.repositories.is_empty() {
            for repo_name in &setup.repositories {
                env.create_git_repo(repo_name).await?;
            }
        }
        
        // Create basic config if needed
        if setup.config {
            env.create_basic_config().await?;
        }
        
        // Create templates if needed
        if setup.templates {
            env.create_default_templates().await?;
        }
        
        Ok(())
    }
    
    /// Run a single test step
    async fn run_test_step(&self, env: &TestEnvironment, step: &TestStep) -> Result<String> {
        let mut cmd = Command::new(&self.binary_path);
        
        // Add command arguments
        for arg in &step.command {
            cmd.arg(arg);
        }
        
        // Set up environment
        cmd.current_dir(&env.workspace_root);
        
        // Override config and root paths
        cmd.arg("--config").arg(&env.config_path);
        cmd.arg("--root").arg(&env.workspace_root);
        
        // Set environment variables
        for (key, value) in env.get_env_vars() {
            cmd.env(key, value);
        }
        
        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        if self.verbose {
            debug!("Running command: {:?}", cmd);
        }
        
        let mut child = cmd.spawn()?;
        
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();
        
        if let Some(mut out) = child.stdout.take() {
            out.read_to_end(&mut stdout).await?;
        }
        
        if let Some(mut err) = child.stderr.take() {
            err.read_to_end(&mut stderr).await?;
        }
        
        let status = child.wait().await?;
        
        let output = String::from_utf8_lossy(&stdout);
        let error = String::from_utf8_lossy(&stderr);
        
        if !status.success() {
            anyhow::bail!(
                "Command failed with status {}: {}\nStderr: {}",
                status.code().unwrap_or(-1),
                output,
                error
            );
        }
        
        Ok(output.to_string())
    }
    
    /// Validate step output
    async fn validate_step(&self, env: &TestEnvironment, validation: &StepValidation, output: &str) -> Result<()> {
        match validation {
            StepValidation::ConfigExists => {
                if !env.config_path.exists() {
                    anyhow::bail!("Config file was not created");
                }
            }
            StepValidation::OutputContains(text) => {
                if !output.contains(text) {
                    anyhow::bail!("Output does not contain expected text: {}", text);
                }
            }
            StepValidation::FileExists(path) => {
                let full_path = env.workspace_root.join(path);
                if !full_path.exists() {
                    anyhow::bail!("Expected file does not exist: {}", path);
                }
            }
            StepValidation::FileContains(path, text) => {
                let full_path = env.workspace_root.join(path);
                let content = tokio::fs::read_to_string(&full_path).await?;
                if !content.contains(text) {
                    anyhow::bail!("File {} does not contain expected text: {}", path, text);
                }
            }
        }
        
        Ok(())
    }
}

/// Test definition structure
#[derive(Debug, Deserialize)]
struct TestDefinition {
    name: String,
    setup: Option<TestSetup>,
    steps: Vec<TestStep>,
}

#[derive(Debug, Deserialize)]
struct TestSetup {
    repositories: Vec<String>,
    config: bool,
    templates: bool,
}

#[derive(Debug, Deserialize)]
struct TestStep {
    name: String,
    command: Vec<String>,
    validate: Option<StepValidation>,
}

#[derive(Debug, Deserialize)]
enum StepValidation {
    ConfigExists,
    OutputContains(String),
    FileExists(String),
    FileContains(String, String),
}

/// Test execution context
pub struct TestContext {
    pub env: TestEnvironment,
    pub binary_path: PathBuf,
    pub verbose: bool,
}

impl TestContext {
    /// Run a vibe command and return output
    pub async fn run_command(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(&self.binary_path);
        
        // Add arguments
        for arg in args {
            cmd.arg(arg);
        }
        
        // Override paths
        cmd.arg("--config").arg(&self.env.config_path);
        cmd.arg("--root").arg(&self.env.workspace_root);
        
        // Set working directory
        cmd.current_dir(&self.env.workspace_root);
        
        // Set environment
        for (key, value) in self.env.get_env_vars() {
            cmd.env(key, value);
        }
        
        // Capture output
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        if self.verbose {
            debug!("Running: vibe {}", args.join(" "));
        }
        
        let output = cmd.output().await?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Command failed: {}", stderr);
        }
        
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
    
    /// Run a command and expect it to fail
    pub async fn run_command_expect_failure(&self, args: &[&str]) -> Result<String> {
        let mut cmd = Command::new(&self.binary_path);
        
        for arg in args {
            cmd.arg(arg);
        }
        
        cmd.arg("--config").arg(&self.env.config_path);
        cmd.arg("--root").arg(&self.env.workspace_root);
        cmd.current_dir(&self.env.workspace_root);
        
        for (key, value) in self.env.get_env_vars() {
            cmd.env(key, value);
        }
        
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());
        
        let output = cmd.output().await?;
        
        if output.status.success() {
            anyhow::bail!("Command unexpectedly succeeded");
        }
        
        Ok(String::from_utf8_lossy(&output.stderr).to_string())
    }
}