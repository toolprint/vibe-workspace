# Vibe Workspace Integration Test Launcher

A standalone integration test framework for the `vibe-workspace` CLI that runs tests in isolated environments without affecting the user's actual home directory.

## Overview

This test launcher provides:
- **Complete isolation**: Each test runs in its own temporary directory
- **Path overrides**: All configurable paths (config, workspace root, templates) are overridden
- **Binary testing**: Tests the actual built binary from `cargo build`
- **Parallel execution**: Optional parallel test execution for faster runs
- **Comprehensive assertions**: Built-in helpers for validating CLI behavior

## Architecture

```
tests/
├── integration_launcher/       # Test framework
│   ├── src/
│   │   ├── main.rs           # CLI and test discovery
│   │   ├── fixtures.rs       # Test environment setup
│   │   ├── runner.rs         # Test execution engine
│   │   └── assertions.rs     # Validation helpers
│   └── Cargo.toml
└── integration/               # Test definitions
    ├── basic_operations.rs
    ├── app_integration.rs
    └── template_management.rs
```

## Usage

### Building and Running Tests

```bash
# First, build the main vibe CLI
cd ../..  # Go to workspace root
cargo build

# Run all integration tests
cd tests/integration_launcher
cargo run

# Run specific tests
cargo run -- --test-filter basic_operations

# Run with release binary
cargo build --release
cargo run -- --release

# Run tests in parallel
cargo run -- --parallel

# Show verbose output
cargo run -- --verbose
```

### Environment Variables

- `VIBE_TEST_BINARY`: Override the path to the vibe binary
- `VIBE_TEST_KEEP_TEMP`: Keep temporary directories after tests (for debugging)
- `VIBE_TEST_VERBOSE`: Enable verbose output

### Command Line Options

```
vibe-integration-launcher [OPTIONS]

OPTIONS:
    --test-filter <PATTERN>    Filter tests by name or pattern (e.g., "basic_*")
    --release                  Use release binary instead of debug
    -v, --verbose             Show verbose output
    --keep-temp               Keep temporary directories after tests
    --binary-path <PATH>      Override path to vibe binary
    --parallel                Run tests in parallel
    --output <FORMAT>         Output format: pretty (default), json, tap
```

## Writing Tests

Tests are written as Rust modules in the `tests/integration/` directory. Each test:

1. Creates an isolated test environment
2. Sets up necessary fixtures (repos, configs, etc.)
3. Runs vibe commands with path overrides
4. Validates the results

### Example Test Structure

```rust
use anyhow::Result;
use vibe_integration_launcher::{
    assertions::{Assert, ConfigAssert},
    fixtures::{TestEnvironment, TestDataBuilder},
    runner::TestContext,
};

#[test_case]
async fn test_workspace_init() -> Result<()> {
    // Create isolated environment
    let env = TestEnvironment::new("test_workspace_init", false).await?;
    
    // Create test context
    let ctx = TestContext {
        env: env.clone(),
        binary_path: get_vibe_binary(),
        verbose: is_verbose(),
    };
    
    // Run command
    let output = ctx.run_command(&["init", "--name", "test"]).await?;
    
    // Validate results
    Assert::output_contains(&output, "Initialized workspace")?;
    Assert::file_exists(&env.config_path).await?;
    ConfigAssert::workspace_name(&env.config_path, "test").await?;
    
    Ok(())
}
```

## Test Environment

Each test runs in a completely isolated environment:

```
temp_dir/
├── home/                      # Isolated HOME directory
│   └── .vibe-workspace/      # Vibe configuration
│       ├── config.yaml       # Workspace config
│       └── templates/        # App templates
└── workspace/                # Workspace root for repos
    ├── repo-1/
    └── repo-2/
```

### Path Overrides

The test runner automatically overrides:
- `--config`: Points to `temp_dir/home/.vibe-workspace/config.yaml`
- `--root`: Points to `temp_dir/workspace/`
- `HOME` environment variable: Points to `temp_dir/home/`

## Available Assertions

### Basic Assertions
- `Assert::file_exists(path)` - Verify file exists
- `Assert::dir_exists(path)` - Verify directory exists
- `Assert::file_contains(path, text)` - Check file content
- `Assert::output_contains(output, text)` - Check command output
- `Assert::output_matches(output, pattern)` - Regex matching

### Config Assertions
- `ConfigAssert::workspace_name(path, name)` - Verify workspace name
- `ConfigAssert::has_repository(path, name)` - Check repo exists
- `ConfigAssert::repository_count(path, count)` - Verify repo count
- `ConfigAssert::repo_has_app(path, repo, app)` - Check app config

### Git Assertions
- `GitAssert::is_git_repo(path)` - Verify git repository
- `GitAssert::current_branch(path, branch)` - Check branch name
- `GitAssert::has_remote(path, name, url)` - Verify remote

### App Assertions
- `AppAssert::warp_config_valid(path)` - Validate Warp config
- `AppAssert::iterm2_config_valid(path)` - Validate iTerm2 config
- `AppAssert::vscode_workspace_valid(path)` - Validate VSCode workspace

## Test Data Builders

Use builders to create complex test scenarios:

```rust
let env = TestDataBuilder::new(env)
    .with_repositories(5)              // Create 5 repos
    .with_nested_repos()              // Add nested repos
    .with_branches(&[                 // Set up branches
        ("repo-1", "develop"),
        ("repo-2", "feature/test"),
    ])
    .build();
```

## Debugging Failed Tests

### Keep Temporary Directories

```bash
# Keep temp dirs for inspection
cargo run -- --keep-temp

# Or use environment variable
VIBE_TEST_KEEP_TEMP=1 cargo run
```

The test output will show the temporary directory path:
```
Test environment for 'test_name': /var/folders/.../T/tmp.XXXXX
```

### Verbose Output

```bash
# Show all command executions
cargo run -- --verbose

# Or use environment variable
VIBE_TEST_VERBOSE=1 cargo run
```

### JSON Output

For CI/CD integration:
```bash
cargo run -- --output json > results.json
```

### TAP Output

For test harness integration:
```bash
cargo run -- --output tap
```

## Adding New Tests

1. Create a new `.rs` file in `tests/integration/`
2. Import the test framework components
3. Write test functions with `#[test_case]` attribute
4. Use assertions to validate behavior

Example categories:
- `basic_operations.rs` - Core CLI commands
- `app_integration.rs` - App configuration features
- `template_management.rs` - Template operations
- `workspace_discovery.rs` - Repository discovery
- `error_handling.rs` - Error scenarios

## Continuous Integration

Example GitHub Actions workflow:

```yaml
- name: Build vibe CLI
  run: cargo build

- name: Run integration tests
  run: |
    cd tests/integration_launcher
    cargo run -- --output json > test-results.json
    
- name: Upload test results
  uses: actions/upload-artifact@v3
  with:
    name: integration-test-results
    path: tests/integration_launcher/test-results.json
```

## Notes

- Tests are discovered automatically from `tests/integration/*.rs`
- Each test runs in complete isolation
- Binary must be built before running tests
- Tests can run in parallel for faster execution
- All file system operations use the isolated environment
- Network operations are not mocked (be careful with external APIs)