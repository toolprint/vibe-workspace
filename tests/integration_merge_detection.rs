//! Integration tests for merge detection functionality

use anyhow::Result;
use tempfile::TempDir;
use tokio::process::Command;
use vibe_workspace::worktree::{
    config::WorktreeMergeDetectionConfig,
    merge_detection::{detect_worktree_merge_status, MergeDetector},
    status::check_worktree_status_with_config,
};

/// Setup a basic git repository for testing
async fn setup_git_repo() -> Result<(TempDir, std::path::PathBuf)> {
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(&["init"])
        .current_dir(&path)
        .output()
        .await?;

    // Configure git user
    Command::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(&path)
        .output()
        .await?;

    Command::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(&path)
        .output()
        .await?;

    // Create initial commit
    std::fs::write(path.join("README.md"), "# Test Repository")?;
    Command::new("git")
        .args(&["add", "README.md"])
        .current_dir(&path)
        .output()
        .await?;

    Command::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(&path)
        .output()
        .await?;

    Ok((temp_dir, path))
}

#[tokio::test]
async fn test_merge_detector_basic_functionality() -> Result<()> {
    let config = WorktreeMergeDetectionConfig::default();
    let detector = MergeDetector::new(config);

    // Test with a temporary directory (should fail gracefully)
    let temp_dir = TempDir::new()?;
    let path = temp_dir.path();

    // Should return error or indicate no merge for non-git directory
    let result = detector.detect_merge(path, "main").await;
    match result {
        Ok(merge_result) => {
            // If it doesn't error, it should at least indicate not merged with low confidence
            assert!(!merge_result.is_merged || merge_result.confidence < 0.5);
        }
        Err(_) => {
            // Failing is also acceptable for non-git directories
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_merge_detection_with_real_git_repo() -> Result<()> {
    let (_temp_dir, path) = setup_git_repo().await?;
    let config = WorktreeMergeDetectionConfig::default();

    // Test merge detection on main branch (should not be "merged" into itself)
    let result = detect_worktree_merge_status(&path, "main", &config).await;

    match result {
        Ok(merge_info) => {
            // Main branch shouldn't be detected as merged into itself
            // The exact behavior may vary based on git setup, but function should not crash
            println!("Merge info: {:?}", merge_info);
        }
        Err(e) => {
            // It's OK if some methods fail in a minimal git setup
            println!("Merge detection failed (expected in minimal setup): {}", e);
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_status_integration_with_merge_detection() -> Result<()> {
    let (_temp_dir, path) = setup_git_repo().await?;
    let config = WorktreeMergeDetectionConfig::default();

    // Test status check with merge detection enabled
    let status = check_worktree_status_with_config(&path, Some(&config)).await?;

    // Should have basic status information
    assert!(status.uncommitted_changes.is_empty());
    assert!(status.untracked_files.is_empty());

    // Merge info may or may not be present depending on git configuration
    // but the function should complete successfully
    println!("Status with merge detection: {:?}", status.merge_info);

    Ok(())
}

#[tokio::test]
async fn test_status_without_merge_detection() -> Result<()> {
    let (_temp_dir, path) = setup_git_repo().await?;

    // Test status check without merge detection
    let status = check_worktree_status_with_config(&path, None).await?;

    // Should have basic status information
    assert!(status.uncommitted_changes.is_empty());
    assert!(status.untracked_files.is_empty());

    // Should not have merge info when config is not provided
    assert!(status.merge_info.is_none());

    Ok(())
}

#[tokio::test]
async fn test_merge_detection_config_variations() -> Result<()> {
    let (_temp_dir, path) = setup_git_repo().await?;

    // Test with different configurations
    let configs = vec![
        WorktreeMergeDetectionConfig {
            use_github_cli: false,
            methods: vec!["standard".to_string()],
            main_branches: vec!["main".to_string()],
        },
        WorktreeMergeDetectionConfig {
            use_github_cli: false,
            methods: vec!["standard".to_string(), "squash".to_string()],
            main_branches: vec!["main".to_string(), "master".to_string()],
        },
    ];

    for (i, config) in configs.iter().enumerate() {
        println!("Testing config variant {}", i + 1);

        let result = detect_worktree_merge_status(&path, "main", config).await;

        // Each configuration should either succeed or fail gracefully
        match result {
            Ok(merge_info) => {
                println!("  Success: {:?}", merge_info);
                // Should have a valid detection method
                assert!(!merge_info.detection_method.is_empty());
                // Confidence should be between 0.0 and 1.0
                assert!(merge_info.confidence >= 0.0 && merge_info.confidence <= 1.0);
            }
            Err(e) => {
                println!("  Expected failure for minimal git setup: {}", e);
            }
        }
    }

    Ok(())
}

#[test]
fn test_merge_detection_method_string_conversion() {
    use vibe_workspace::worktree::merge_detection::MergeDetectionMethod;

    // Test all method conversions
    let methods = vec![
        (MergeDetectionMethod::Standard, "standard"),
        (MergeDetectionMethod::Squash, "squash"),
        (MergeDetectionMethod::GitHubPR, "github_pr"),
        (MergeDetectionMethod::FileContent, "file_content"),
    ];

    for (method, expected_str) in methods {
        // Test to string conversion
        assert_eq!(method.as_str(), expected_str);

        // Test from string conversion
        assert_eq!(MergeDetectionMethod::from_str(expected_str), Some(method));
    }

    // Test invalid string
    assert_eq!(MergeDetectionMethod::from_str("invalid"), None);
}

#[test]
fn test_merge_detection_result_structures() {
    use vibe_workspace::worktree::merge_detection::{MergeDetectionResult, MethodResult};
    use vibe_workspace::worktree::status::MergeInfo;

    // Test MethodResult creation
    let method_result = MethodResult {
        method: "test_method".to_string(),
        is_merged: true,
        confidence: 0.85,
        details: Some("Test details".to_string()),
        error: None,
    };

    assert_eq!(method_result.method, "test_method");
    assert!(method_result.is_merged);
    assert_eq!(method_result.confidence, 0.85);
    assert!(method_result.error.is_none());

    // Test MergeDetectionResult creation and conversion
    let detection_result = MergeDetectionResult {
        is_merged: true,
        detection_method: "standard".to_string(),
        confidence: 0.9,
        details: Some("merged into main".to_string()),
        method_results: vec![method_result],
    };

    // Test conversion to MergeInfo
    let merge_info: MergeInfo = detection_result.into();
    assert!(merge_info.is_merged);
    assert_eq!(merge_info.detection_method, "standard");
    assert_eq!(merge_info.confidence, 0.9);
    assert_eq!(merge_info.details, Some("merged into main".to_string()));
}
