// Dagger CI/CD module for vibe-workspace
package main

import (
	"context"
	"fmt"

	"dagger/vibe-workspace/internal/dagger"
	"golang.org/x/sync/errgroup"
)

// VibeWorkspace is the main Dagger module for the vibe-workspace project
type VibeWorkspace struct{}

// Format checks if the Rust code is properly formatted
func (m *VibeWorkspace) Format(ctx context.Context, source *dagger.Directory) *dagger.Container {
	return m.rustBase(source).
		WithExec([]string{"cargo", "fmt", "--check"})
}

// Lint runs clippy to check for code quality issues
func (m *VibeWorkspace) Lint(ctx context.Context, source *dagger.Directory) *dagger.Container {
	return m.buildEnv(source).
		WithExec([]string{"cargo", "clippy", "--all-targets", "--all-features"})
}

// Test runs the test suite for a specific platform
func (m *VibeWorkspace) Test(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
) *dagger.Container {
	return m.buildEnvWithPlatform(source, platform).
		WithExec([]string{"cargo", "test"})
}

// Coverage generates a code coverage report
func (m *VibeWorkspace) Coverage(ctx context.Context, source *dagger.Directory) *dagger.File {
	return m.buildEnv(source).
		WithExec([]string{"cargo", "install", "cargo-tarpaulin"}).
		WithExec([]string{"cargo", "tarpaulin", "--out", "Html", "--output-dir", "."}).
		File("tarpaulin-report.html")
}

// Build creates a debug build for a specific platform
func (m *VibeWorkspace) Build(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
) *dagger.File {
	base := m.buildEnvWithPlatform(source, platform)
	name := "vibe"
	
	// Build the binary
	container := base.
		WithExec([]string{"cargo", "build", "--bin", name})
	
	// The binary path in the container
	path := fmt.Sprintf("/src/target/debug/%s", name)
	
	return container.File(path)
}

// BuildDebug creates a debug build and shows the build output
func (m *VibeWorkspace) BuildDebug(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
) (string, error) {
	base := m.buildEnvWithPlatform(source, platform)
	name := "vibe"
	
	// Build the binary and capture output
	container := base.
		WithExec([]string{"cargo", "build", "--bin", name})
	
	// Get the build output
	output, err := container.Stdout(ctx)
	if err != nil {
		return "", fmt.Errorf("build failed: %w", err)
	}
	
	// Also check if the binary exists
	checkContainer := container.
		WithExec([]string{"ls", "-la", "/src/target/debug/"})
	
	lsOutput, err := checkContainer.Stdout(ctx)
	if err != nil {
		return output, fmt.Errorf("failed to list directory: %w", err)
	}
	
	return fmt.Sprintf("Build output:\n%s\n\nDirectory listing:\n%s", output, lsOutput), nil
}

// BuildRelease creates an optimized release build for a specific platform
func (m *VibeWorkspace) BuildRelease(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="linux/amd64"
	platform string,
) *dagger.File {
	base := m.buildEnvWithPlatform(source, platform)
	name := "vibe"
	path := fmt.Sprintf("/src/target/release/%s", name)

	return base.
		WithExec([]string{"cargo", "build", "--release", "--bin", name}).
		File(path)
}

// CI runs the complete CI pipeline (format, lint, test)
func (m *VibeWorkspace) CI(ctx context.Context, source *dagger.Directory) *dagger.Container {
	return m.rustBase(source).
		WithExec([]string{"cargo", "fmt", "--check"}).
		WithExec([]string{"cargo", "clippy", "--all-targets", "--all-features"}).
		WithExec([]string{"cargo", "test"})
}

// Package creates a release archive for a specific platform
func (m *VibeWorkspace) Package(
	ctx context.Context,
	source *dagger.Directory,
	binary *dagger.File,
	platform string,
	version string,
) *dagger.File {
	archiveName := fmt.Sprintf("vibe-workspace-%s-%s.tar.gz", version, platform)
	
	return dag.Container().
		From("alpine:latest").
		WithExec([]string{"apk", "add", "--no-cache", "tar", "gzip"}).
		WithFile("/tmp/vibe", binary).
		WithFile("/tmp/README.md", source.File("README.md")).
		WithWorkdir("/tmp").
		WithExec([]string{"tar", "czf", archiveName, "vibe", "README.md"}).
		File(archiveName)
}

// Release builds release binaries for Linux platforms (both x86_64 and ARM64)
func (m *VibeWorkspace) Release(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="v0.1.0"
	version string,
) *dagger.Directory {
	var archives []*dagger.File

	// Build for Linux x86_64
	linuxAmd64Binary := m.BuildRelease(ctx, source, "linux/amd64")
	linuxAmd64Archive := m.Package(ctx, source, linuxAmd64Binary, "x86_64-unknown-linux-gnu", version)
	archives = append(archives, linuxAmd64Archive)

	// Build for Linux ARM64
	linuxArm64Binary := m.BuildRelease(ctx, source, "linux/arm64")
	linuxArm64Archive := m.Package(ctx, source, linuxArm64Binary, "aarch64-unknown-linux-gnu", version)
	archives = append(archives, linuxArm64Archive)

	// Create output directory with all archives
	output := dag.Directory()
	
	// Add Linux x86_64 archive
	output = output.WithFile(fmt.Sprintf("vibe-workspace-%s-x86_64-unknown-linux-gnu.tar.gz", version), archives[0])
	
	// Add Linux ARM64 archive
	output = output.WithFile(fmt.Sprintf("vibe-workspace-%s-aarch64-unknown-linux-gnu.tar.gz", version), archives[1])

	return output
}

// ZigbuildSingle builds a release binary for a single platform using cargo-zigbuild
func (m *VibeWorkspace) ZigbuildSingle(
	ctx context.Context,
	source *dagger.Directory,
	target string,
) *dagger.File {
	base := m.zigbuildBase(source)
	
	// Add the target
	base = base.WithExec([]string{"rustup", "target", "add", target})
	
	// Determine features based on target
	var buildCmd []string
	if contains(target, "apple-darwin") {
		// macOS targets might have specific features in the future
		buildCmd = []string{"cargo", "zigbuild", "--release", "--target", target}
	} else if contains(target, "windows") {
		// Windows targets
		buildCmd = []string{"cargo", "zigbuild", "--release", "--target", target}
	} else {
		// Linux and other targets
		buildCmd = []string{"cargo", "zigbuild", "--release", "--target", target}
	}
	
	// Build the binary
	container := base.WithExec(buildCmd)
	
	// Determine binary path
	binaryName := "vibe"
	if contains(target, "windows") {
		binaryName = "vibe.exe"
	}
	binaryPath := fmt.Sprintf("target/%s/release/%s", target, binaryName)
	
	return container.File(binaryPath)
}

// ReleaseZigbuild builds release binaries for multiple platforms using cargo-zigbuild
func (m *VibeWorkspace) ReleaseZigbuild(
	ctx context.Context,
	source *dagger.Directory,
	// +optional
	// +default="v0.1.0"
	version string,
) (*dagger.Directory, error) {
	// Define all target platforms
	targets := []struct {
		rust     string
		platform string
		archive  string
	}{
		{"x86_64-unknown-linux-gnu", "linux/amd64", "tar.gz"},
		{"x86_64-apple-darwin", "darwin/amd64", "tar.gz"},
		{"aarch64-apple-darwin", "darwin/arm64", "tar.gz"},
		{"universal2-apple-darwin", "darwin/universal", "tar.gz"},
	}

	// Build all targets in parallel
	g, gctx := errgroup.WithContext(ctx)
	archives := make([]*dagger.File, len(targets))
	archiveNames := make([]string, len(targets))

	for i, target := range targets {
		i, target := i, target // capture loop variables
		g.Go(func() error {
			// Build the binary
			binary := m.ZigbuildSingle(gctx, source, target.rust)
			
			// Package the binary and determine archive name
			if target.archive == "zip" {
				archives[i] = m.packageZip(gctx, source, binary, target.rust, version)
				archiveNames[i] = fmt.Sprintf("vibe-workspace-%s-%s.zip", version, target.rust)
			} else {
				archives[i] = m.packageTarGz(gctx, source, binary, target.rust, version)
				archiveNames[i] = fmt.Sprintf("vibe-workspace-%s-%s.tar.gz", version, target.rust)
			}
			
			return nil
		})
	}

	// Wait for all builds to complete
	if err := g.Wait(); err != nil {
		return nil, err
	}

	// Create output directory with all archives
	output := dag.Directory()
	for i, archive := range archives {
		if archive != nil {
			output = output.WithFile(archiveNames[i], archive)
		}
	}

	return output, nil
}

// Helper functions

// rustBase returns a container with Rust toolchain installed
func (m *VibeWorkspace) rustBase(source *dagger.Directory) *dagger.Container {
	return dag.Container().
		From("rust:1.88.0").
		WithDirectory("/src", source).
		WithWorkdir("/src").
		WithExec([]string{"rustup", "component", "add", "rustfmt", "clippy"})
}

// rustBaseWithPlatform returns a container with Rust toolchain for a specific platform
func (m *VibeWorkspace) rustBaseWithPlatform(source *dagger.Directory, platform string) *dagger.Container {
	return dag.Container(dagger.ContainerOpts{Platform: dagger.Platform(platform)}).
		From("rust:1.88.0").
		WithDirectory("/src", source).
		WithWorkdir("/src").
		WithExec([]string{"rustup", "component", "add", "rustfmt", "clippy"})
}

// buildEnv returns a container with build dependencies and source code
func (m *VibeWorkspace) buildEnv(source *dagger.Directory) *dagger.Container {
	return m.rustBase(source).
		WithEnvVariable("CARGO_HOME", "/cargo").
		WithMountedCache("/cargo", dag.CacheVolume("cargo-cache")).
		WithMountedCache("/src/target", dag.CacheVolume("target-cache"))
}

// buildEnvWithPlatform returns a container with build dependencies for a specific platform
func (m *VibeWorkspace) buildEnvWithPlatform(source *dagger.Directory, platform string) *dagger.Container {
	return m.rustBaseWithPlatform(source, platform).
		WithEnvVariable("CARGO_HOME", "/cargo").
		WithMountedCache("/cargo", dag.CacheVolume("cargo-cache")).
		WithMountedCache("/src/target", dag.CacheVolume("target-cache"))
}

// zigbuildBase returns a container with cargo-zigbuild installed
func (m *VibeWorkspace) zigbuildBase(source *dagger.Directory) *dagger.Container {
	return dag.Container().
		From("ghcr.io/rust-cross/cargo-zigbuild:latest").
		WithDirectory("/src", source).
		WithWorkdir("/src").
		WithEnvVariable("CARGO_HOME", "/cargo").
		WithMountedCache("/cargo", dag.CacheVolume("cargo-cache")).
		WithMountedCache("/src/target", dag.CacheVolume("target-cache"))
}

// packageTarGz creates a tar.gz archive
func (m *VibeWorkspace) packageTarGz(
	ctx context.Context,
	source *dagger.Directory,
	binary *dagger.File,
	platform string,
	version string,
) *dagger.File {
	archiveName := fmt.Sprintf("vibe-workspace-%s-%s.tar.gz", version, platform)
	
	return dag.Container().
		From("alpine:latest").
		WithExec([]string{"apk", "add", "--no-cache", "tar", "gzip"}).
		WithFile("/tmp/vibe", binary).
		WithFile("/tmp/README.md", source.File("README.md")).
		WithWorkdir("/tmp").
		WithExec([]string{"tar", "czf", archiveName, "vibe", "README.md"}).
		File(archiveName)
}

// packageZip creates a zip archive (for Windows)
func (m *VibeWorkspace) packageZip(
	ctx context.Context,
	source *dagger.Directory,
	binary *dagger.File,
	platform string,
	version string,
) *dagger.File {
	archiveName := fmt.Sprintf("vibe-workspace-%s-%s.zip", version, platform)
	
	return dag.Container().
		From("alpine:latest").
		WithExec([]string{"apk", "add", "--no-cache", "zip"}).
		WithFile("/tmp/vibe.exe", binary).
		WithFile("/tmp/README.md", source.File("README.md")).
		WithWorkdir("/tmp").
		WithExec([]string{"zip", "-j", archiveName, "vibe.exe", "README.md"}).
		File(archiveName)
}

// contains checks if a string contains a substring
func contains(s, substr string) bool {
	return len(s) > 0 && len(substr) > 0 && (s == substr || len(s) > len(substr) && (s[:len(substr)] == substr || s[len(s)-len(substr):] == substr || substr != s && contains(s[1:], substr)))
}