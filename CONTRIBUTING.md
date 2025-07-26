# Contributing to Vibe Workspace

Thank you for your interest in contributing to Vibe Workspace! This document provides guidelines and instructions for contributing to the project.

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/your-username/vibe-workspace.git`
3. Create a feature branch: `git checkout -b feature/your-feature-name`
4. Make your changes
5. Commit your changes: `git commit -m "feat: add amazing feature"`
6. Push to your fork: `git push origin feature/your-feature-name`
7. Open a Pull Request

## Development Setup

### Prerequisites

- Rust (stable toolchain)
- Git
- Development tools for your platform

### Installing Dependencies

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone the repository
git clone https://github.com/toolprint/vibe-workspace.git
cd vibe-workspace

# Install project dependencies (handled by Cargo)
cargo build
```

### Building the Project

```bash
cargo build          # Debug build
cargo build --release # Release build
cargo build --profile release-small # Ultra-optimized build
```

## Making Changes

### Branch Naming

Use descriptive branch names:

- `feature/add-app-integration`
- `fix/issue-123-description`
- `docs/update-readme`
- `refactor/improve-workspace-manager`

### Development Workflow

1. **Create a new branch**:

   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** following the coding standards

3. **Run tests and checks**:

   ```bash
   cargo fmt     # Format code
   cargo clippy  # Run linter
   cargo test    # Run tests
   ```

4. **Test your changes**:

   ```bash
   # Test the binary locally
   cargo run -- --help
   cargo run -- init
   cargo run -- interactive
   ```

5. **Commit your changes**:

   ```bash
   git add .
   git commit -m "feat: add support for XYZ"
   ```

## Commit Messages

We follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` New features
- `fix:` Bug fixes
- `docs:` Documentation changes
- `style:` Code style changes (formatting, missing semicolons, etc)
- `refactor:` Code refactoring
- `test:` Adding or updating tests
- `chore:` Maintenance tasks

## Coding Standards

### Rust Guidelines

- Follow Rust idioms and best practices
- Use meaningful variable and function names
- Add comments for complex logic
- Prefer clarity over cleverness

### Code Organization

- Keep modules focused and cohesive
- Use appropriate visibility modifiers
- Group related functionality
- Follow existing project structure:
  - `apps/` - App integrations and management
  - `workspace/` - Core workspace functionality
  - `ui/` - User interface components
  - `utils/` - Shared utilities

### Error Handling

- Use `anyhow` for application errors
- Use `thiserror` for library errors
- Provide context with `.context()`
- Handle all error cases appropriately

## Testing

### Running Tests

```bash
cargo test                    # Run all tests
cargo test --verbose         # Run with detailed output
cargo test workspace         # Test specific module
cargo test -- --nocapture    # Show print statements
```

### Writing Tests

- Add unit tests in module files using `#[cfg(test)]`
- Add integration tests in `tests/` directory 
- Use the integration test launcher in `tests/integration_launcher/`
- Test edge cases and error conditions
- Use descriptive test names
- Test app integrations with mock data when possible

### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_initialization() {
        // Arrange
        let temp_dir = tempfile::tempdir().unwrap();
        let config_path = temp_dir.path().join("config.yaml");
        
        // Act
        let result = WorkspaceManager::init_workspace("test", temp_dir.path());
        
        // Assert
        assert!(result.is_ok());
        assert!(config_path.exists());
    }
}
```

## Documentation

### Code Documentation

- Document public APIs with doc comments
- Include examples in doc comments
- Update README.md for user-facing changes
- Update docs/APPS.md for app integrations
- Update docs/QUICK_START.md for workflow changes

### Documentation Standards

```rust
/// Brief description of the function.
///
/// More detailed explanation if needed.
///
/// # Arguments
///
/// * `param` - Description of parameter
///
/// # Returns
///
/// Description of return value
///
/// # Examples
///
/// ```
/// use vibe_workspace::WorkspaceManager;
/// let manager = WorkspaceManager::new(config_path).await?;
/// let result = manager.open_repo_with_app("my-repo", "warp").await?;
/// ```
pub async fn open_repo_with_app(&self, repo: &str, app: &str) -> Result<()> {
    // Implementation
}
```

## Pull Request Process

1. Update the README.md if needed
2. Ensure all tests pass
3. Update documentation for API changes
4. Get at least one code review approval
5. Squash commits if requested

### PR Requirements

- All tests must pass (`cargo test`)
- Code must be formatted (`cargo fmt`)
- No clippy warnings (`cargo clippy`)
- Documentation updated if needed
- Follows project coding standards
- Integration tests pass for affected features

## Contributing App Integrations

Vibe Workspace supports two types of apps:

### Repository Opening Apps
These apps support full integration with `vibe open <repo> --app <app>`:

1. **Add to the registry** in `src/apps/registry.rs`:
   ```rust
   AppPackage::new("myapp", "My App", "Description")
       .with_binary_name("myapp")
       .with_version_command(vec!["--version"])
       .with_brew_cask("myapp")
   ```

2. **Create integration module** in `src/apps/myapp.rs`:
   - Implement `open_with_myapp()` function
   - Add template support
   - Handle platform-specific behavior

3. **Add to module exports** in `src/apps/mod.rs`

4. **Create default template** in appropriate format (YAML/JSON)

5. **Update documentation**:
   - Add to docs/APPS.md with usage examples
   - Update README.md supported apps list
   - Add to docs/QUICK_START.md examples

### Installable Developer Tools
For tools that don't need repository opening integration:

1. **Add to registry** in `src/apps/registry.rs` with package manager info
2. **Update documentation** in docs/APPS.md under "Additional Developer Tools"

### App Integration Testing

- Test with mock configurations in isolation
- Use `tests/integration_launcher/` for full integration tests
- Test template generation and variable substitution
- Verify cross-platform compatibility when possible

## Code of Conduct

Please read our [Code of Conduct](CODE_OF_CONDUCT.md) before contributing.

## Questions?

Feel free to open an issue for any questions about contributing.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.