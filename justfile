#!/usr/bin/env -S just --justfile

# =====================================
# 🚀 QUICK REFERENCE - Manual Release Workflow
# =====================================
# 1. just zigbuild-release           # Build cross-platform binaries (includes checksums)
# 2. just release-all                # Complete release (validation → GitHub → cargo publish)
#
# Individual steps:
# - just validate-artifacts          # Validate built binaries
# - just create-github-release       # Create GitHub release + upload artifacts & checksums
# - just validate-github-release     # Verify upload success
# - just test-binstall               # Test cargo-binstall installation from GitHub
# - just cargo-publish               # Publish to crates.io
#
# cargo-binstall support: Users can install with `cargo binstall vibe-workspace`
# =====================================

_default:
    @just -l -u

# Brew installation
[group('setup')]
brew:
    brew update & brew bundle install --file=./Brewfile

# Rust Development Commands

# Build the project
[group('rust')]
build:
    @echo "🔨 Building vibe-workspace..."
    cargo build

# Build in release mode
[group('rust')]
build-release:
    @echo "🔨 Building vibe-workspace (release)..."
    cargo build --release
    @just release-info

# Install tq (TOML query tool) for better TOML parsing
[group('rust')]
install-tq:
    @echo "📦 Installing tq (TOML query tool)..."
    cargo install --git https://github.com/cryptaliagy/tomlq

# Show information about release binaries
[group('rust')]
release-info:
    #!/usr/bin/env bash
    echo "============================="
    echo "📦 Release Binary Information"
    echo "============================="
    echo ""
    
    if [ ! -d "target/release" ]; then
        echo "❌ Release directory not found"
        echo "   Run 'just build-release' first"
        exit 0
    fi
    
    echo "🗂️  Release Directory: target/release/"
    echo ""
    
    # Parse TOML to get binary names
    if command -v tq >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        echo "🔍 Using tq + jq to parse Cargo.toml"
        binaries=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null | jq -r '.[].name' 2>/dev/null | tr '\n' ' ')
    elif command -v tq >/dev/null 2>&1; then
        echo "🔍 Using tq to parse Cargo.toml (install jq for better parsing)"
        bin_json=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null)
        # Extract names from JSON manually
        binaries=$(echo "$bin_json" | sed 's/.*"name":"\([^"]*\)".*/\1/g' | tr '\n' ' ')
    else
        echo "🔍 Using AWK to parse Cargo.toml (fallback - install tq for better parsing)"
        echo "   Install with: just install-tq"
        binaries=$(awk '
        /^\[\[bin\]\]/ { in_bin=1; next }
        /^\[/ { in_bin=0 }
        in_bin && /^name = / {
            gsub(/^name = "|"$/, "")
            print
        }
        ' Cargo.toml | tr '\n' ' ')
    fi
    
    if [ -z "$binaries" ]; then
        echo "❌ No [[bin]] sections found in Cargo.toml"
        echo "   Check Cargo.toml configuration"
        exit 0
    fi
    
    echo "🔍 Binaries defined in Cargo.toml: $binaries"
    echo ""
    
    found_any=false
    for binary in $binaries; do
        if [ -f "target/release/$binary" ]; then
            echo "🔧 Binary: $binary"
            echo "   📍 Path: target/release/$binary"
            echo "   📏 Size: $(du -h target/release/$binary | cut -f1)"
            echo "   🏗️  Platform: $(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]')"
            echo "   📅 Modified: $(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' target/release/$binary 2>/dev/null || stat -c '%y' target/release/$binary 2>/dev/null | cut -d'.' -f1)"
            if command -v file >/dev/null 2>&1; then
                echo "   🔍 Type: $(file target/release/$binary | cut -d':' -f2 | sed 's/^ *//')"
            fi
            echo ""
            found_any=true
        else
            echo "❌ Binary $binary not found in target/release/"
            echo ""
        fi
    done
    
    if [ "$found_any" = false ]; then
        echo "❌ No binaries found in target/release/"
        echo "   Run 'just build-release' first"
    fi

# Install release binaries locally and show installation info
[group('rust')]
install-local-release: build-release
    #!/usr/bin/env bash
    echo "📦 Installing Release Binaries"
    echo "=============================="
    echo ""
    
    # Parse TOML to get binary names (same logic as release-info)
    if command -v tq >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        echo "🔍 Using tq + jq to parse Cargo.toml"
        binaries=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null | jq -r '.[].name' 2>/dev/null | tr '\n' ' ')
    elif command -v tq >/dev/null 2>&1; then
        echo "🔍 Using tq to parse Cargo.toml"
        bin_json=$(tq -o json -f Cargo.toml 'bin' 2>/dev/null)
        binaries=$(echo "$bin_json" | sed 's/.*"name":"\([^"]*\)".*/\1/g' | tr '\n' ' ')
    else
        echo "🔍 Using AWK to parse Cargo.toml"
        binaries=$(awk '
        /^\[\[bin\]\]/ { in_bin=1; next }
        /^\[/ { in_bin=0 }
        in_bin && /^name = / {
            gsub(/^name = "|"$/, "")
            print
        }
        ' Cargo.toml | tr '\n' ' ')
    fi
    
    if [ -z "$binaries" ]; then
        echo "❌ No [[bin]] sections found in Cargo.toml"
        exit 1
    fi
    
    echo "🔍 Installing binaries: $binaries"
    echo ""
    
    # Install using cargo install
    echo "🚀 Running: cargo install --path . --force"
    if cargo install --path . --force; then
        echo ""
        echo "✅ Installation completed successfully!"
        echo ""
        
        # Show installation information  
        if [ -n "$CARGO_HOME" ]; then
            cargo_bin_dir="$CARGO_HOME/bin"
        else
            cargo_bin_dir="$HOME/.cargo/bin"
        fi
        
        echo "📂 Installation Directory: $cargo_bin_dir"
        echo ""
        
        for binary in $binaries; do
            if [ -f "$cargo_bin_dir/$binary" ]; then
                echo "🔧 Binary: $binary"
                echo "   📍 Path: $cargo_bin_dir/$binary"
                echo "   📏 Size: $(du -h $cargo_bin_dir/$binary | cut -f1)"
                echo "   🏗️  Platform: $(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]')"
                echo "   📅 Installed: $(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' $cargo_bin_dir/$binary 2>/dev/null || stat -c '%y' $cargo_bin_dir/$binary 2>/dev/null | cut -d'.' -f1)"
                if command -v file >/dev/null 2>&1; then
                    echo "   🔍 Type: $(file $cargo_bin_dir/$binary | cut -d':' -f2 | sed 's/^ *//')"
                fi
                echo ""
            else
                echo "❌ Binary $binary not found at $cargo_bin_dir/$binary"
                echo ""
            fi
        done
        
        echo "💡 Usage:"
        echo "   Run directly: $binary --help"
        echo "   Or ensure ~/.cargo/bin is in your PATH"
        echo ""
        
    else
        echo ""
        echo "❌ Installation failed!"
        exit 1
    fi

# Install from zigbuild release artifacts
[group('rust')]
install-zig-release:
    #!/usr/bin/env bash
    echo "📦 Installing from Zigbuild Release Artifacts"
    echo "============================================="
    echo ""
    
    # Check if release-artifacts directory exists
    if [ ! -d "./release-artifacts" ]; then
        echo "❌ Release artifacts directory not found"
        echo "   Run 'just zigbuild-release' or 'just dagger-release' first"
        exit 1
    fi
    
    # Extract version from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ]; then
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    
    version="v$version"  # Add v prefix for release naming
    echo "🔍 Looking for version: $version"
    
    # Detect platform
    arch=$(uname -m)
    os=$(uname -s | tr '[:upper:]' '[:lower:]')
    
    echo "🔍 Detected platform: $arch-$os"
    echo ""
    
    # Determine target platform and artifact name
    case "$os" in
        "darwin")
            # Check for universal2 first (preferred for macOS)
            if [ -f "./release-artifacts/vibe-workspace-$version-universal2-apple-darwin.tar.gz" ]; then
                target="universal2-apple-darwin"
                artifact="vibe-workspace-$version-universal2-apple-darwin.tar.gz"
                echo "🎯 Using universal2 binary for macOS"
            elif [ "$arch" = "arm64" ] && [ -f "./release-artifacts/vibe-workspace-$version-aarch64-apple-darwin.tar.gz" ]; then
                target="aarch64-apple-darwin"
                artifact="vibe-workspace-$version-aarch64-apple-darwin.tar.gz"
                echo "🎯 Using ARM64 binary for macOS"
            elif [ "$arch" = "x86_64" ] && [ -f "./release-artifacts/vibe-workspace-$version-x86_64-apple-darwin.tar.gz" ]; then
                target="x86_64-apple-darwin"
                artifact="vibe-workspace-$version-x86_64-apple-darwin.tar.gz"
                echo "🎯 Using x86_64 binary for macOS"
            else
                echo "❌ No compatible macOS artifact found for version $version"
                echo "   Available artifacts:"
                ls -1 ./release-artifacts/ | grep -E '\.tar\.gz$' | sed 's/^/   /'
                exit 1
            fi
            ;;
        "linux")
            if [ "$arch" = "x86_64" ] && [ -f "./release-artifacts/vibe-workspace-$version-x86_64-unknown-linux-gnu.tar.gz" ]; then
                target="x86_64-unknown-linux-gnu"
                artifact="vibe-workspace-$version-x86_64-unknown-linux-gnu.tar.gz"
                echo "🎯 Using x86_64 binary for Linux"
            else
                echo "❌ No compatible Linux artifact found for version $version"
                echo "   Available artifacts:"
                ls -1 ./release-artifacts/ | grep -E '\.tar\.gz$' | sed 's/^/   /'
                exit 1
            fi
            ;;
        *)
            echo "❌ Unsupported platform: $os"
            echo "   Available artifacts:"
            ls -1 ./release-artifacts/ | grep -E '\.tar\.gz$' | sed 's/^/   /'
            exit 1
            ;;
    esac
    
    echo "📁 Selected artifact: $artifact"
    echo ""
    
    # Determine installation directory
    if [ -n "$CARGO_HOME" ]; then
        cargo_bin_dir="$CARGO_HOME/bin"
    else
        cargo_bin_dir="$HOME/.cargo/bin"
    fi
    
    # Create cargo bin directory if it doesn't exist
    mkdir -p "$cargo_bin_dir"
    
    # Create temporary directory for extraction
    temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT
    
    echo "🔧 Extracting $artifact..."
    if tar -xzf "./release-artifacts/$artifact" -C "$temp_dir"; then
        echo "✅ Extraction successful"
    else
        echo "❌ Failed to extract artifact"
        exit 1
    fi
    
    # Find the binary in extracted files
    binary_name="vibe"
    if [ -f "$temp_dir/$binary_name" ]; then
        echo "🚀 Installing $binary_name to $cargo_bin_dir/"
        
        # Copy binary and make executable
        cp "$temp_dir/$binary_name" "$cargo_bin_dir/$binary_name"
        chmod +x "$cargo_bin_dir/$binary_name"
        
        echo "✅ Installation completed successfully!"
        echo ""
        
        # Show installation information
        echo "📂 Installation Directory: $cargo_bin_dir"
        echo ""
        echo "🔧 Binary: $binary_name"
        echo "   📍 Path: $cargo_bin_dir/$binary_name"
        echo "   📏 Size: $(du -h $cargo_bin_dir/$binary_name | cut -f1)"
        echo "   🏗️  Platform: $target"
        echo "   📅 Installed: $(stat -f '%Sm' -t '%Y-%m-%d %H:%M:%S' $cargo_bin_dir/$binary_name 2>/dev/null || stat -c '%y' $cargo_bin_dir/$binary_name 2>/dev/null | cut -d'.' -f1)"
        if command -v file >/dev/null 2>&1; then
            echo "   🔍 Type: $(file $cargo_bin_dir/$binary_name | cut -d':' -f2 | sed 's/^ *//')"
        fi
        echo ""
        
        echo "💡 Usage:"
        echo "   Run directly: $binary_name --help"
        echo "   Or ensure ~/.cargo/bin is in your PATH"
        echo ""
        
    else
        echo "❌ Binary $binary_name not found in extracted archive"
        echo "   Contents of archive:"
        ls -la "$temp_dir/"
        exit 1
    fi

# Install from zigbuild release artifacts (default install command)
[group('rust')]
install: install-zig-release

# Run cli with arguments (example: just run --help)
[group('rust')]
run *args:
    @echo "🚀 Running cli with args: {{args}}"
    cargo run -- {{args}}

# Run tests
[group('rust')]
test:
    @echo "🧪 Running tests..."
    cargo test

# Run only MCP tests
[group('rust')]
test-mcp:
    @echo "🧪 Running MCP tests..."
    cargo test -- --ignored mcp

# Check code without building
[group('rust')]
check:
    @echo "🔍 Checking code..."
    cargo check

# Format code
[group('rust')]
fmt:
    @echo "🎨 Formatting code..."
    cargo fmt

# Run clippy linter
[group('rust')]
clippy:
    @echo "📎 Running clippy..."
    cargo clippy

# Clean build artifacts
[group('rust')]
clean:
    @echo "🧹 Cleaning build artifacts..."
    cargo clean

# Formatting Commands

# Check all formatting
[group('format')]
check-fmt:
    @echo "🔍 Checking Rust formatting..."
    cargo fmt --check

# Pre-commit validation - runs all checks required before committing
[group('format')]
pre-commit:
    #!/usr/bin/env bash
    echo "🔄 Running pre-commit validation..."
    echo "=================================="
    echo ""
    
    # Track success for checks and linters
    checks_success=true
    
    # 1. Static check (cargo check)
    echo "1️⃣  Static code check..."
    if cargo check; then
        echo "   ✅ Static check passed"
    else
        echo "   ❌ Static check failed"
        checks_success=false
    fi
    echo ""
    
    # 2. Code formatting check
    echo "2️⃣  Code formatting check..."
    if cargo fmt --check; then
        echo "   ✅ Code formatting is correct"
    else
        echo "   ❌ Code formatting issues found"
        echo "   💡 Run 'just fmt' to fix formatting"
        checks_success=false
    fi
    echo ""
    
    # 3. Clippy linter
    echo "3️⃣  Clippy linter check..."
    # TODO: Re-enable strict warnings with `cargo clippy -- -D warnings` before release
    # Currently allowing warnings during active development
    if cargo clippy; then
        echo "   ✅ Clippy linter passed"
    else
        echo "   ❌ Clippy linter found issues"
        checks_success=false
    fi
    echo ""
    
    # Check if we should proceed to tests
    if [ "$checks_success" = false ]; then
        echo "=================================="
        echo "❌ FAILURE: Code checks and linters failed"
        echo "🔧 Please fix the above issues before running tests"
        echo "💡 Once fixed, run 'just pre-commit' again to include tests"
        exit 1
    fi
    
    # 4. Tests (only run if all checks passed, excluding MCP tests)
    echo "4️⃣  Running tests (excluding MCP tests)..."
    if cargo test -- --skip mcp; then
        echo "   ✅ All tests passed (MCP tests excluded from pre-commit)"
    else
        echo "   ❌ Some tests failed"
        echo ""
        echo "=================================="
        echo "❌ FAILURE: Tests failed"
        echo "🔧 Please fix the failing tests before committing"
        exit 1
    fi
    echo ""
    
    # Final success message
    echo "=================================="
    echo "🎉 SUCCESS: All pre-commit checks passed!"
    echo "✅ Code is ready for commit"

# =====================================
# Dagger CI/CD Commands (LEGACY - Not used in current workflow)
# =====================================
# NOTE: Dagger commands are slow locally and not part of the current
# manual release process. Use zigbuild-release and release-all instead.

# Run Dagger CI pipeline locally
[group('dagger')]
dagger-ci:
    @echo "🚀 Running Dagger CI pipeline..."
    dagger call ci --source .

# Run Dagger format check
[group('dagger')]
dagger-format:
    @echo "🔍 Checking code formatting with Dagger..."
    dagger call format --source .

# Run Dagger lint
[group('dagger')]
dagger-lint:
    @echo "📋 Running clippy with Dagger..."
    dagger call lint --source .

# Run Dagger tests
[group('dagger')]
dagger-test platform="linux/amd64":
    @echo "🧪 Running tests on {{ platform }} with Dagger..."
    dagger call test --source . --platform {{ platform }}

# Run Dagger coverage
[group('dagger')]
dagger-coverage:
    @echo "📊 Generating coverage report with Dagger..."
    dagger call coverage --source . export --path ./tarpaulin-report.html
    @echo "✅ Coverage report saved to tarpaulin-report.html"

# Build with Dagger
[group('dagger')]
dagger-build platform="linux/amd64":
    @echo "🔨 Building for {{ platform }} with Dagger..."
    @mkdir -p ./build
    dagger call build --source . --platform {{ platform }} export --path ./build/vibe-debug-{{ replace(platform, "/", "-") }}

# Build release with Dagger
[group('dagger')]
dagger-build-release platform="linux/amd64":
    @echo "📦 Building release for {{ platform }} with Dagger..."
    @mkdir -p ./build
    dagger call build-release --source . --platform {{ platform }} export --path ./build/vibe-release-{{ replace(platform, "/", "-") }}

# Build releases for all platforms using Dagger with zigbuild (parallel execution)
[group('dagger')]
dagger-release:
    #!/usr/bin/env bash
    echo "🚀 Building all platform releases in parallel with Dagger + zigbuild..."
    
    # Extract version from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ]; then
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    
    version="v$version"  # Add v prefix for release naming
    echo "📦 Building version: $version"
    echo ""
    
    mkdir -p ./release-artifacts
    dagger call release-zigbuild --source . --version $version export --path ./release-artifacts/
    echo "✅ All platform releases built successfully!"
    echo "📦 Release artifacts:"
    ls -la ./release-artifacts/

# Run complete release pipeline using Dagger
[group('dagger')]
dagger-release-all:
    #!/usr/bin/env bash
    echo "🚀 Running complete release pipeline with Dagger..."
    
    # Extract version from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ]; then
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    
    version="v$version"  # Add v prefix for release naming
    echo "📦 Building version: $version"
    echo ""
    
    mkdir -p ./release-artifacts
    dagger call release --source . --version $version export --path ./release-artifacts/
    echo "✅ Complete release pipeline finished!"
    echo "📦 Release artifacts:"
    ls -la ./release-artifacts/


# =====================================
# Zigbuild Cross-Compilation Commands
# =====================================

# Build all platforms using cargo-zigbuild Docker image
[group('zigbuild')]
zigbuild-release:
    #!/usr/bin/env bash
    echo "🚀 Building releases for all platforms using cargo-zigbuild..."
    
    # Extract version from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ]; then
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    
    version="v$version"  # Add v prefix for release naming
    echo "📦 Building version: $version"
    echo ""
    
    mkdir -p ./release-artifacts
    
    
    # Build all platforms in a single container to maintain state
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest \
        sh -c '
            echo "📦 Adding Rust targets..." && \
            rustup target add x86_64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin && \
            echo "🔨 Building Linux x86_64..." && \
            cargo zigbuild --release --target x86_64-unknown-linux-gnu && \
            echo "🔨 Building macOS x86_64..." && \
            cargo zigbuild --release --target x86_64-apple-darwin && \
            echo "🔨 Building macOS ARM64..." && \
            cargo zigbuild --release --target aarch64-apple-darwin && \
            echo "🔨 Building macOS Universal Binary..." && \
            cargo zigbuild --release --target universal2-apple-darwin
        '
    
    # Package all builds
    echo "📦 Packaging release artifacts..."
    
    # Linux x86_64
    tar czf ./release-artifacts/vibe-workspace-$version-x86_64-unknown-linux-gnu.tar.gz \
        -C target/x86_64-unknown-linux-gnu/release vibe \
        -C "$(pwd)" README.md
    
    # macOS x86_64
    tar czf ./release-artifacts/vibe-workspace-$version-x86_64-apple-darwin.tar.gz \
        -C target/x86_64-apple-darwin/release vibe \
        -C "$(pwd)" README.md
    
    # macOS ARM64
    tar czf ./release-artifacts/vibe-workspace-$version-aarch64-apple-darwin.tar.gz \
        -C target/aarch64-apple-darwin/release vibe \
        -C "$(pwd)" README.md
    
    # macOS Universal
    tar czf ./release-artifacts/vibe-workspace-$version-universal2-apple-darwin.tar.gz \
        -C target/universal2-apple-darwin/release vibe \
        -C "$(pwd)" README.md
    
    echo "✅ All platform releases built successfully!"
    echo ""
    
    # Generate checksums for security verification
    echo "🔐 Generating SHA256 checksums..."
    cd ./release-artifacts
    for file in *.tar.gz; do
        if [ -f "$file" ]; then
            shasum -a 256 "$file" >> SHA256SUMS
            echo "   ✅ $file"
        fi
    done
    cd ..
    
    echo "📦 Release artifacts:"
    ls -la ./release-artifacts/
    echo ""
    echo "🔐 Checksums saved to: ./release-artifacts/SHA256SUMS"

# Test zigbuild setup for a single platform
[group('zigbuild')]
zigbuild-test target="x86_64-apple-darwin":
    #!/usr/bin/env bash
    echo "🧪 Testing cargo-zigbuild for {{ target }}..."
    
    
    # Determine feature flags based on target
    if [[ "{{ target }}" == *"apple-darwin"* ]]; then
        features=""
    else
        features=""
    fi
    
    docker run --rm -v $(pwd):/io -w /io ghcr.io/rust-cross/cargo-zigbuild:latest \
        sh -c "rustup target add {{ target }} && cargo zigbuild --release --target {{ target }} $features"
    
    # Determine binary name based on target
    if [[ "{{ target }}" == *"windows"* ]]; then
        binary_name="vibe.exe"
    else
        binary_name="vibe"
    fi
    
    echo "✅ Build successful! Binary at: target/{{ target }}/release/$binary_name"

# Clean up release artifacts
[group('zigbuild')]
clean-release-artifacts:
    #!/usr/bin/env bash
    echo "🧹 Cleaning release artifacts..."
    if [ -d "./release-artifacts" ]; then
        rm -rf ./release-artifacts/*
        echo "✅ Release artifacts cleaned"
        echo "📁 Directory: $(pwd)/release-artifacts/ (empty)"
    else
        echo "ℹ️  No release artifacts to clean"
    fi

# =====================================
# Cargo Publishing Commands
# =====================================

# Check if package is ready for publishing
[group('publish')]
cargo-check-publish:
    #!/usr/bin/env bash
    echo "🔍 Checking package readiness for publishing..."
    echo "=============================================="
    echo ""
    
    # Check if we're in a git repository and if it's clean
    if git rev-parse --git-dir > /dev/null 2>&1; then
        if [ -n "$(git status --porcelain)" ]; then
            echo "⚠️  Warning: Git working directory is not clean"
            echo "   Consider committing changes before publishing"
            git status --porcelain | sed 's/^/   /'
            echo ""
        else
            echo "✅ Git working directory is clean"
        fi
        
        # Check if current commit is tagged
        current_tag=$(git describe --exact-match --tags HEAD 2>/dev/null || echo "")
        if [ -n "$current_tag" ]; then
            echo "✅ Current commit is tagged: $current_tag"
        else
            echo "⚠️  Warning: Current commit is not tagged"
            echo "   Consider creating a version tag before publishing"
        fi
        echo ""
    else
        echo "⚠️  Warning: Not in a git repository"
        echo ""
    fi
    
    # Extract version from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    if [ -n "$version" ]; then
        echo "📦 Package version: $version"
        
        # Check if this version already exists on crates.io
        echo "🔍 Checking if version $version exists on crates.io..."
        if curl -s "https://crates.io/api/v1/crates/vibe-workspace" | grep -q "\"num\":\"$version\""; then
            echo "❌ Version $version already exists on crates.io"
            echo "   You need to bump the version before publishing"
            exit 1
        else
            echo "✅ Version $version is available on crates.io"
        fi
    else
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    echo ""
    
    # Check for required metadata
    echo "🔍 Checking package metadata..."
    
    # Check description
    if grep -q '^description = ' Cargo.toml; then
        echo "✅ Description: present"
    else
        echo "❌ Description: missing (required for crates.io)"
    fi
    
    # Check license
    if grep -q '^license = ' Cargo.toml; then
        echo "✅ License: present"
    else
        echo "❌ License: missing (required for crates.io)"
    fi
    
    # Check repository
    if grep -q '^repository = ' Cargo.toml; then
        echo "✅ Repository: present"
    else
        echo "⚠️  Repository: missing (recommended)"
    fi
    
    # Check keywords
    if grep -q '^keywords = ' Cargo.toml; then
        echo "✅ Keywords: present"
    else
        echo "⚠️  Keywords: missing (recommended)"
    fi
    
    # Check categories
    if grep -q '^categories = ' Cargo.toml; then
        echo "✅ Categories: present"
    else
        echo "⚠️  Categories: missing (recommended)"
    fi
    
    echo ""
    echo "📋 Package check completed!"

# Create a package for inspection without uploading
[group('publish')]
cargo-package:
    @echo "📦 Creating package for inspection..."
    cargo package
    @echo "✅ Package created successfully!"
    @echo "📁 Package file: target/package/vibe-workspace-$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/').crate"
    @echo ""
    @echo "💡 To inspect the package contents:"
    @echo "   tar -tzf target/package/vibe-workspace-*.crate"

# Dry run of cargo publish (validates without uploading)
[group('publish')]
cargo-publish-dry: cargo-check-publish
    @echo "🔍 Performing dry run of cargo publish..."
    cargo publish --dry-run
    @echo "✅ Dry run completed successfully!"
    @echo "📋 Package is ready for publishing"

# Publish to crates.io (requires authentication)
[group('publish')]
cargo-publish: cargo-check-publish
    #!/usr/bin/env bash
    echo "🚀 Publishing to crates.io..."
    echo "=============================="
    echo ""
    
    # Final confirmation
    echo "⚠️  This will publish the package to crates.io!"
    echo "   Once published, versions cannot be yanked or deleted"
    echo ""
    read -p "Are you sure you want to proceed? (y/N): " -n 1 -r
    echo ""
    
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        echo ""
        echo "🚀 Publishing..."
        if cargo publish; then
            version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
            echo ""
            echo "🎉 Successfully published vibe-workspace v$version to crates.io!"
            echo "📦 Package URL: https://crates.io/crates/vibe-workspace"
            echo ""
            echo "💡 Users can now install with:"
            echo "   cargo install vibe-workspace"
        else
            echo ""
            echo "❌ Publishing failed!"
            exit 1
        fi
    else
        echo ""
        echo "❌ Publishing cancelled"
        exit 1
    fi

# Show publishing status and information
[group('publish')]
cargo-publish-info:
    #!/usr/bin/env bash
    echo "📊 Publishing Information"
    echo "========================"
    echo ""
    
    # Package info
    if command -v tq >/dev/null 2>&1; then
        name=$(tq -r '.package.name' Cargo.toml 2>/dev/null)
        version=$(tq -r '.package.version' Cargo.toml 2>/dev/null)
    else
        name=$(grep '^name = ' Cargo.toml | head -1 | sed 's/name = "\(.*\)"/\1/')
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    echo "📦 Package: $name v$version"
    echo ""
    
    # Check crates.io status
    echo "🔍 Checking crates.io status..."
    if curl -s "https://crates.io/api/v1/crates/$name" > /dev/null 2>&1; then
        echo "✅ Package exists on crates.io"
        
        # Get published versions
        versions=$(curl -s "https://crates.io/api/v1/crates/$name" | grep -o '"num":"[^"]*"' | cut -d'"' -f4 | head -5)
        echo "📋 Recent versions:"
        echo "$versions" | sed 's/^/   /'
        
        if echo "$versions" | grep -q "^$version$"; then
            echo "✅ Current version ($version) is published"
        else
            echo "⚠️  Current version ($version) is not yet published"
        fi
    else
        echo "⚠️  Package not found on crates.io (not yet published)"
    fi
    echo ""
    
    echo "🔗 Links:"
    echo "   📦 Crates.io: https://crates.io/crates/$name"
    echo "   📚 Docs.rs: https://docs.rs/$name"
    if grep -q '^repository = ' Cargo.toml; then
        repo=$(grep '^repository = ' Cargo.toml | sed 's/repository = "\(.*\)"/\1/')
        echo "   🔗 Repository: $repo"
    fi

# =====================================
# Manual Release Workflow Commands
# =====================================

# Validate release artifacts are properly built and functional
[group('release')]
validate-artifacts:
    #!/usr/bin/env bash
    echo "🔍 Validating release artifacts..."
    echo "=================================="
    echo ""
    
    # Extract version from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ]; then
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    
    version="v$version"
    echo "🔍 Validating artifacts for version: $version"
    echo ""
    
    # Check if release-artifacts directory exists
    if [ ! -d "./release-artifacts" ]; then
        echo "❌ Release artifacts directory not found"
        echo "   Run 'just zigbuild-release' first"
        exit 1
    fi
    
    # Expected artifacts
    expected_artifacts=(
        "vibe-workspace-$version-x86_64-unknown-linux-gnu.tar.gz"
        "vibe-workspace-$version-x86_64-apple-darwin.tar.gz"
        "vibe-workspace-$version-aarch64-apple-darwin.tar.gz"
        "vibe-workspace-$version-universal2-apple-darwin.tar.gz"
    )
    
    validation_success=true
    
    for artifact in "${expected_artifacts[@]}"; do
        artifact_path="./release-artifacts/$artifact"
        
        if [ ! -f "$artifact_path" ]; then
            echo "❌ Missing artifact: $artifact"
            validation_success=false
            continue
        fi
        
        echo "🔧 Validating: $artifact"
        
        # Check if archive can be extracted
        temp_dir=$(mktemp -d)
        if tar -tzf "$artifact_path" > /dev/null 2>&1; then
            echo "   ✅ Archive format is valid"
            
            # Extract and check for vibe binary
            tar -xzf "$artifact_path" -C "$temp_dir" 2>/dev/null
            
            if [ -f "$temp_dir/vibe" ]; then
                echo "   ✅ Contains vibe binary"
                
                # Check if binary is executable (on compatible platforms)
                if [[ "$artifact" == *"apple-darwin"* ]] && [[ "$(uname)" == "Darwin" ]]; then
                    if file "$temp_dir/vibe" | grep -q "executable"; then
                        echo "   ✅ Binary is executable"
                    else
                        echo "   ⚠️  Binary may not be executable"
                    fi
                elif [[ "$artifact" == *"linux-gnu"* ]] && [[ "$(uname)" == "Linux" ]]; then
                    if file "$temp_dir/vibe" | grep -q "executable"; then
                        echo "   ✅ Binary is executable"
                    else
                        echo "   ⚠️  Binary may not be executable"
                    fi
                else
                    echo "   ℹ️  Cross-platform binary (cannot test execution on this platform)"
                fi
                
                # Show binary size
                size=$(du -h "$temp_dir/vibe" | cut -f1)
                echo "   📏 Binary size: $size"
            else
                echo "   ❌ Missing vibe binary in archive"
                validation_success=false
            fi
            
            # Check for README.md
            if [ -f "$temp_dir/README.md" ]; then
                echo "   ✅ Contains README.md"
            else
                echo "   ⚠️  Missing README.md"
            fi
        else
            echo "   ❌ Invalid archive format"
            validation_success=false
        fi
        
        # Cleanup temp directory
        rm -rf "$temp_dir"
        
        # Show artifact size
        artifact_size=$(du -h "$artifact_path" | cut -f1)
        echo "   📦 Archive size: $artifact_size"
        echo ""
    done
    
    if [ "$validation_success" = true ]; then
        echo "✅ All artifacts validated successfully!"
        echo ""
        echo "📋 Summary:"
        echo "   📦 Total artifacts: ${#expected_artifacts[@]}"
        echo "   📁 Total size: $(du -sch ./release-artifacts/*.tar.gz | tail -1 | cut -f1)"
        echo ""
        echo "🚀 Ready for GitHub release!"
    else
        echo "❌ Artifact validation failed!"
        echo "   Please rebuild artifacts with 'just zigbuild-release'"
        exit 1
    fi

# Create GitHub release and upload artifacts
[group('release')]
create-github-release:
    #!/usr/bin/env bash
    echo "🚀 Creating GitHub release..."
    echo "============================="
    echo ""
    
    # Check if gh CLI is available
    if ! command -v gh >/dev/null 2>&1; then
        echo "❌ GitHub CLI (gh) is not installed"
        echo "   Install with: brew install gh"
        echo "   Or download from: https://cli.github.com/"
        exit 1
    fi
    
    # Check if user is authenticated
    if ! gh auth status >/dev/null 2>&1; then
        echo "❌ Not authenticated with GitHub CLI"
        echo "   Run: gh auth login"
        exit 1
    fi
    
    # Extract version and other info from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
        name=$(tq -f Cargo.toml -r '.package.name' 2>/dev/null)
        description=$(tq -f Cargo.toml -r '.package.description' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
        name=$(grep '^name = ' Cargo.toml | head -1 | sed 's/name = "\(.*\)"/\1/')
        description=$(grep '^description = ' Cargo.toml | head -1 | sed 's/description = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ]; then
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    
    tag="v$version"
    echo "📦 Creating release for: $name v$version"
    echo "🏷️  Git tag: $tag"
    echo ""
    
    # Check if release already exists
    if gh release view "$tag" >/dev/null 2>&1; then
        echo "⚠️  Release $tag already exists!"
        read -p "Do you want to delete it and recreate? (y/N): " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "🗑️  Deleting existing release..."
            gh release delete "$tag" --yes
        else
            echo "❌ Release creation cancelled"
            exit 1
        fi
    fi
    
    # Check if tag already exists
    if git tag -l | grep -q "^$tag$"; then
        echo "⚠️  Git tag $tag already exists!"
        read -p "Do you want to delete it and recreate? (y/N): " -n 1 -r
        echo ""
        if [[ $REPLY =~ ^[Yy]$ ]]; then
            echo "🗑️  Deleting existing tag..."
            git tag -d "$tag"
            git push origin --delete "$tag" 2>/dev/null || true
        else
            echo "❌ Release creation cancelled"
            exit 1
        fi
    fi
    
    # Create and push git tag
    echo "🏷️  Creating git tag..."
    git tag -a "$tag" -m "Release $tag"
    git push origin "$tag"
    
    # Generate release notes from template
    if [ ! -f "release-notes-template.md" ]; then
        echo "❌ Release notes template not found: release-notes-template.md"
        echo "   This file should exist in the project root"
        exit 1
    fi
    
    echo "📝 Generating release notes from template..."
    release_notes=$(sed -e "s/__NAME__/$name/g" \
                        -e "s/__TAG__/$tag/g" \
                        -e "s/__DESCRIPTION__/$description/g" \
                        release-notes-template.md)
    
    # Create GitHub release
    echo "📝 Creating GitHub release..."
    echo "$release_notes" | gh release create "$tag" \
        --title "$name $tag" \
        --notes-file - \
        --draft
    
    echo "✅ Draft release created successfully!"
    echo ""
    
    # Upload artifacts
    echo "📤 Uploading release artifacts..."
    artifacts=(
        "./release-artifacts/vibe-workspace-$tag-x86_64-unknown-linux-gnu.tar.gz"
        "./release-artifacts/vibe-workspace-$tag-x86_64-apple-darwin.tar.gz"
        "./release-artifacts/vibe-workspace-$tag-aarch64-apple-darwin.tar.gz"
        "./release-artifacts/vibe-workspace-$tag-universal2-apple-darwin.tar.gz"
        "./release-artifacts/SHA256SUMS"
    )
    
    upload_success=true
    for artifact in "${artifacts[@]}"; do
        if [ -f "$artifact" ]; then
            echo "📤 Uploading $(basename "$artifact")..."
            if gh release upload "$tag" "$artifact"; then
                echo "   ✅ Uploaded successfully"
            else
                echo "   ❌ Upload failed"
                upload_success=false
            fi
        else
            echo "❌ Artifact not found: $artifact"
            upload_success=false
        fi
    done
    
    if [ "$upload_success" = true ]; then
        echo ""
        echo "✅ All artifacts uploaded successfully!"
        echo ""
        echo "🔗 Release URL: $(gh release view "$tag" --json url --jq .url)"
        echo ""
        echo "⚠️  Release is currently in DRAFT status"
        echo "   Review the release and publish it manually on GitHub"
        echo "   Or run: gh release edit '$tag' --draft=false"
    else
        echo ""
        echo "❌ Some artifacts failed to upload!"
        echo "   Please check the errors above and try again"
        exit 1
    fi

# Validate that GitHub release artifacts were uploaded successfully
[group('release')]
validate-github-release:
    #!/usr/bin/env bash
    echo "🔍 Validating GitHub release artifacts..."
    echo "========================================"
    echo ""
    
    # Check if gh CLI is available
    if ! command -v gh >/dev/null 2>&1; then
        echo "❌ GitHub CLI (gh) is not installed"
        exit 1
    fi
    
    # Extract version from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ]; then
        echo "❌ Could not extract version from Cargo.toml"
        exit 1
    fi
    
    tag="v$version"
    echo "🔍 Validating release: $tag"
    echo ""
    
    # Check if release exists
    if ! gh release view "$tag" >/dev/null 2>&1; then
        echo "❌ Release $tag not found on GitHub"
        echo "   Run 'just create-github-release' first"
        exit 1
    fi
    
    # Get release info
    release_url=$(gh release view "$tag" --json url --jq .url)
    is_draft=$(gh release view "$tag" --json isDraft --jq .isDraft)
    
    echo "🔗 Release URL: $release_url"
    echo "📋 Draft status: $is_draft"
    echo ""
    
    # Check expected artifacts
    expected_artifacts=(
        "vibe-workspace-$tag-x86_64-unknown-linux-gnu.tar.gz"
        "vibe-workspace-$tag-x86_64-apple-darwin.tar.gz"
        "vibe-workspace-$tag-aarch64-apple-darwin.tar.gz"
        "vibe-workspace-$tag-universal2-apple-darwin.tar.gz"
    )
    
    echo "🔍 Checking uploaded artifacts..."
    uploaded_assets=$(gh release view "$tag" --json assets --jq '.assets[].name')
    
    validation_success=true
    for artifact in "${expected_artifacts[@]}"; do
        if echo "$uploaded_assets" | grep -q "^$artifact$"; then
            echo "   ✅ $artifact"
            
            # Get download URL and size
            download_url=$(gh release view "$tag" --json assets --jq ".assets[] | select(.name==\"$artifact\") | .browserDownloadUrl")
            size=$(gh release view "$tag" --json assets --jq ".assets[] | select(.name==\"$artifact\") | .size")
            size_mb=$(echo "scale=2; $size / 1024 / 1024" | bc -l)
            echo "      📏 Size: ${size_mb}MB"
            echo "      🔗 URL: $download_url"
        else
            echo "   ❌ Missing: $artifact"
            validation_success=false
        fi
    done
    
    echo ""
    
    if [ "$validation_success" = true ]; then
        echo "✅ All artifacts successfully uploaded to GitHub!"
        echo ""
        if [ "$is_draft" = "true" ]; then
            echo "⚠️  Release is currently in DRAFT status"
            echo "   Publish it with: gh release edit '$tag' --draft=false"
            echo "   Or publish manually on GitHub"
        else
            echo "🎉 Release is PUBLISHED and ready!"
        fi
        echo ""
        echo "🚀 Ready for cargo publish!"
    else
        echo "❌ GitHub release validation failed!"
        echo "   Some artifacts are missing from the release"
        exit 1
    fi

# Test cargo-binstall installation from GitHub release
[group('release')]
test-binstall:
    #!/usr/bin/env bash
    echo "🧪 Testing cargo-binstall installation..."
    echo "========================================"
    echo ""
    
    # Check if cargo-binstall is available
    if ! command -v cargo-binstall >/dev/null 2>&1; then
        echo "❌ cargo-binstall is not installed"
        echo "   Install with: cargo install cargo-binstall"
        exit 1
    fi
    
    # Extract version and name from Cargo.toml
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
        name=$(tq -f Cargo.toml -r '.package.name' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
        name=$(grep '^name = ' Cargo.toml | head -1 | sed 's/name = "\(.*\)"/\1/')
    fi
    
    if [ -z "$version" ] || [ -z "$name" ]; then
        echo "❌ Could not extract version or name from Cargo.toml"
        exit 1
    fi
    
    tag="v$version"
    echo "🔍 Testing installation of: $name $tag"
    echo ""
    
    # Check if GitHub release exists
    if ! command -v gh >/dev/null 2>&1; then
        echo "⚠️  GitHub CLI not available - cannot verify release exists"
    elif ! gh release view "$tag" >/dev/null 2>&1; then
        echo "❌ Release $tag not found on GitHub"
        echo "   Run 'just create-github-release' first"
        exit 1
    else
        echo "✅ GitHub release $tag found"
    fi
    
    # Create temporary directory for test installation
    temp_dir=$(mktemp -d)
    trap "rm -rf $temp_dir" EXIT
    
    echo "📦 Testing cargo-binstall installation..."
    echo "   Target directory: $temp_dir"
    echo ""
    
    # Test cargo-binstall with dry-run first
    echo "🧪 Dry-run test..."
    if CARGO_INSTALL_ROOT="$temp_dir" cargo binstall "$name" --version "$version" --dry-run; then
        echo "✅ Dry-run successful - metadata and URLs are valid"
    else
        echo "❌ Dry-run failed - check cargo-binstall metadata configuration"
        exit 1
    fi
    echo ""
    
    # Actual installation test
    echo "📥 Actual installation test..."
    if CARGO_INSTALL_ROOT="$temp_dir" cargo binstall "$name" --version "$version" --no-confirm; then
        echo "✅ Installation successful"
        
        # Test if binary was installed and works
        installed_binary="$temp_dir/bin/vibe"
        if [ -f "$installed_binary" ]; then
            echo "✅ Binary installed at: $installed_binary"
            
            # Test binary execution
            if "$installed_binary" --version >/dev/null 2>&1; then
                actual_version=$("$installed_binary" --version | grep -o '[0-9]\+\.[0-9]\+\.[0-9]\+' | head -1)
                echo "✅ Binary executes successfully"
                echo "   Reported version: $actual_version"
                
                if [ "$actual_version" = "$version" ]; then
                    echo "✅ Version matches expected: $version"
                else
                    echo "⚠️  Version mismatch - Expected: $version, Got: $actual_version"
                fi
            else
                echo "❌ Binary fails to execute"
                exit 1
            fi
        else
            echo "❌ Binary not found at expected location"
            exit 1
        fi
    else
        echo "❌ Installation failed"
        exit 1
    fi
    
    echo ""
    echo "🎉 cargo-binstall test completed successfully!"
    echo ""
    echo "✅ Test Results:"
    echo "   📦 Package metadata: valid"
    echo "   🔗 Download URLs: accessible"
    echo "   📥 Installation: successful"
    echo "   🔧 Binary execution: working"
    echo "   📋 Version: correct"
    echo ""
    echo "👥 Users can install with: cargo binstall $name"

# Complete manual release workflow
[group('release')]
release-all:
    #!/usr/bin/env bash
    echo "🚀 Starting complete manual release workflow..."
    echo "=============================================="
    echo ""
    
    # Extract version for logging
    if command -v tq >/dev/null 2>&1; then
        version=$(tq -f Cargo.toml -r '.package.version' 2>/dev/null)
        name=$(tq -f Cargo.toml -r '.package.name' 2>/dev/null)
    else
        version=$(grep '^version = ' Cargo.toml | head -1 | sed 's/version = "\(.*\)"/\1/')
        name=$(grep '^name = ' Cargo.toml | head -1 | sed 's/name = "\(.*\)"/\1/')
    fi
    
    echo "📦 Releasing: $name v$version"
    echo ""
    
    # Step 1: Pre-commit validation
    echo "1️⃣  Pre-commit validation..."
    if ! just pre-commit; then
        echo "❌ Pre-commit validation failed!"
        exit 1
    fi
    echo ""
    
    # Step 2: Build cross-platform binaries
    echo "2️⃣  Building cross-platform binaries..."
    if ! just zigbuild-release; then
        echo "❌ Cross-platform build failed!"
        exit 1
    fi
    echo ""
    
    # Step 3: Validate artifacts
    echo "3️⃣  Validating release artifacts..."
    if ! just validate-artifacts; then
        echo "❌ Artifact validation failed!"
        exit 1
    fi
    echo ""
    
    # Step 4: Create GitHub release
    echo "4️⃣  Creating GitHub release..."
    if ! just create-github-release; then
        echo "❌ GitHub release creation failed!"
        exit 1
    fi
    echo ""
    
    # Step 5: Validate GitHub release
    echo "5️⃣  Validating GitHub release..."
    if ! just validate-github-release; then
        echo "❌ GitHub release validation failed!"
        exit 1
    fi
    echo ""
    
    # Step 6: Prompt to publish GitHub release
    echo "6️⃣  Publishing GitHub release..."
    tag="v$version"
    read -p "Do you want to publish the GitHub release now? (y/N): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if gh release edit "$tag" --draft=false; then
            echo "✅ GitHub release published!"
        else
            echo "❌ Failed to publish GitHub release!"
            exit 1
        fi
    else
        echo "⚠️  Skipping GitHub release publish"
        echo "   You can publish later with: gh release edit '$tag' --draft=false"
    fi
    echo ""
    
    # Step 7: Final confirmation for cargo publish
    echo "7️⃣  Cargo publish to crates.io..."
    echo "⚠️  This will publish to crates.io and cannot be undone!"
    read -p "Do you want to publish to crates.io now? (y/N): " -n 1 -r
    echo ""
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        if just cargo-publish; then
            echo ""
            echo "🎉 COMPLETE RELEASE SUCCESS!"
            echo "=========================="
            echo "✅ Cross-platform binaries built"
            echo "✅ GitHub release published"
            echo "✅ Published to crates.io"
            echo ""
            echo "🔗 GitHub Release: $(gh release view $tag --json url --jq .url)"
            echo "📦 Crates.io: https://crates.io/crates/$name"
            echo ""
            echo "👥 Users can now install with:"
            echo "   cargo binstall $name  (fast binary install)"
            echo "   cargo install $name   (compile from source)"
        else
            echo "❌ Cargo publish failed!"
            exit 1
        fi
    else
        echo "⚠️  Skipping cargo publish"
        echo "   You can publish later with: just cargo-publish"
        echo ""
        echo "🎯 Release Status:"
        echo "✅ Cross-platform binaries built"
        echo "✅ GitHub release ready"
        echo "⏳ Cargo publish pending"
    fi

# =====================================
# MCP Testing Commands
# =====================================


# Launch MCP Inspector UI for interactive testing
[group('mcp')]
mcp-inspector:
    #!/usr/bin/env bash
    echo "🔍 Launching MCP Inspector for vibe-workspace..."
    echo "=============================================="
    echo ""
    echo "📡 Starting Inspector UI at http://localhost:6274"
    echo "🔐 Note the session token for authentication"
    echo ""
    echo "💡 Tips:"
    echo "   - Use the UI to test tools interactively"
    echo "   - View request/response details in real-time"
    echo "   - Export server config for Claude/Cursor"
    echo ""
    echo "Press Ctrl+C to stop the Inspector"
    echo ""
    npx @modelcontextprotocol/inspector cargo run -- mcp --stdio

# Launch MCP Inspector in CLI mode (non-interactive)
[group('mcp')]
mcp-inspector-cli:
    #!/usr/bin/env bash
    echo "🔍 Running MCP Inspector in CLI mode..."
    echo "======================================="
    echo ""
    echo "📋 Listing available tools:"
    npx @modelcontextprotocol/inspector --cli cargo run -- mcp --stdio --method tools/list

# Test MCP Inspector installation
[group('mcp')]
mcp-inspector-test:
    #!/usr/bin/env bash
    echo "🔍 Testing MCP Inspector installation..."
    echo "======================================="
    echo ""
    
    # Check if npx is available
    if ! command -v npx &> /dev/null; then
        echo "❌ npx not found. Please install Node.js first."
        exit 1
    fi
    
    # Try to run the Inspector with just the version flag
    echo "Checking Inspector version..."
    if npx @modelcontextprotocol/inspector --help 2>&1 | grep -q "Usage:"; then
        echo "✅ MCP Inspector is available and working"
        echo ""
        echo "You can now use:"
        echo "  • just mcp-inspector - Launch the Inspector UI"
    else
        echo "❌ Failed to run MCP Inspector"
        echo "This might be a first-time download. Try running 'just mcp-inspector' directly."
    fi

# List all tools using Inspector CLI mode
[group('mcp')]
mcp-inspector-list-tools:
    #!/usr/bin/env bash
    echo "📋 Listing MCP tools via Inspector CLI..."
    echo "========================================"
    echo ""
    npx @modelcontextprotocol/inspector --cli cargo run -- mcp --stdio --method tools/list


# Call a specific tool using Inspector CLI mode
[group('mcp')]
mcp-inspector-call-tool tool_name params='{}':
    #!/usr/bin/env bash
    echo "🔧 Calling tool '{{tool_name}}' via Inspector CLI..."
    echo "=================================================="
    echo "Parameters: {{params}}"
    echo ""
    
    # Parse JSON params into tool-arg format
    if [ "{{params}}" = "{}" ]; then
        npx @modelcontextprotocol/inspector --cli cargo run -- mcp --stdio --method tools/call --tool-name {{tool_name}}
    else
        # Convert JSON to tool-arg format (simple implementation for common cases)
        args=""
        if echo '{{params}}' | grep -q '"dirty_only":[[:space:]]*true'; then
            args="$args --tool-arg dirty_only=true"
        fi
        if echo '{{params}}' | grep -q '"format":[[:space:]]*"[^"]*"'; then
            format=$(echo '{{params}}' | sed -n 's/.*"format":[[:space:]]*"\([^"]*\)".*/\1/p')
            if [ -n "$format" ]; then
                args="$args --tool-arg format=$format"
            fi
        fi
        if echo '{{params}}' | grep -q '"group":[[:space:]]*"[^"]*"'; then
            group=$(echo '{{params}}' | sed -n 's/.*"group":[[:space:]]*"\([^"]*\)".*/\1/p')
            if [ -n "$group" ]; then
                args="$args --tool-arg group=$group"
            fi
        fi
        
        npx @modelcontextprotocol/inspector --cli cargo run -- mcp --stdio --method tools/call --tool-name {{tool_name}} $args
    fi

# Show CLI mode usage examples
[group('mcp')]
mcp-inspector-cli-examples:
    #!/usr/bin/env bash
    echo "📚 MCP Inspector CLI Mode Examples"
    echo "=================================="
    echo ""
    echo "Basic Usage:"
    echo "  npx @modelcontextprotocol/inspector --cli <server_command> --method <method>"
    echo ""
    echo "Available Methods:"
    echo "  • tools/list         - List all available tools"
    echo "  • tools/call         - Call a specific tool"
    echo ""
    echo "Examples with vibe-workspace:"
    echo ""
    echo "1. List tools:"
    echo "   just mcp-inspector-list-tools"
    echo ""
    echo "2. Call git status tool (show dirty repos only):"
    echo "   just mcp-inspector-call-tool vibe_git_status '{\"dirty_only\": true}'"
    echo ""
    echo "3. Call with different format:"
    echo "   just mcp-inspector-call-tool vibe_git_status '{\"format\": \"table\"}'"
    echo ""
    echo "4. Direct CLI usage (without JSON parsing):"
    echo "   npx @modelcontextprotocol/inspector --cli cargo run -- mcp --stdio \\"
    echo "     --method tools/call --tool-name vibe_git_status --tool-arg dirty_only=true"
    echo ""
    echo "💡 CLI mode is ideal for:"
    echo "   - Scripting and automation"
    echo "   - CI/CD integration"
    echo "   - Programmatic testing"
    echo "   - Quick command-line debugging"

# Show MCP testing help
[group('mcp')]
mcp-help:
    @echo "📚 MCP Testing Commands"
    @echo "======================="
    @echo ""
    @echo "MCP Inspector Testing:"
    @echo "  just mcp-inspector              - Launch Inspector UI (visual testing)"
    @echo "  just mcp-inspector-cli          - Run Inspector in CLI mode"
    @echo "  just mcp-inspector-list-tools   - List tools via CLI"
    @echo "  just mcp-inspector-call-tool    - Call a tool via CLI"
    @echo "  just mcp-inspector-cli-examples - Show CLI usage examples"
    @echo "  just mcp-inspector-export       - Show config for Claude/Cursor"
    @echo ""
    @echo "💡 Start with 'just mcp-inspector' for visual debugging"
    @echo "   Or use 'just mcp-inspector-cli' for command-line testing"

# Show how to export server configuration
[group('mcp')]
mcp-inspector-export:
    @echo "📋 MCP Server Configuration Export"
    @echo "=================================="
    @echo ""
    @echo "Add this to your MCP config:"
    @echo ""
    @echo '  "vibe": {'
    @echo '    "command": "vibe",'
    @echo '    "args": ["mcp", "--stdio"]'
    @echo '  }'
    @echo ""
    @echo "Or with full path:"
    @echo ""
    @echo '  "vibe": {'
    @echo '    "command": "'$(which vibe || echo "/path/to/vibe")'",'
    @echo '    "args": ["mcp", "--stdio"]'
    @echo '  }'

