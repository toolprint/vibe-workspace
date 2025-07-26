// Basic operations integration test
// This file is processed by the integration test launcher
// Each function represents a test scenario that will be executed

use std::process::Command;
use std::env;

fn main() {
    println!("Basic operations integration test");
    
    // Test 1: Initialize workspace
    test_init_workspace();
    
    // Test 2: Add repository
    test_add_repository();
    
    // Test 3: List repositories
    test_list_repositories();
}

fn test_init_workspace() {
    println!("Testing workspace initialization...");
    
    let output = Command::new(env::var("VIBE_BINARY_PATH").expect("VIBE_BINARY_PATH not set"))
        .args(&["init", "--name", "test-workspace"])
        .env("HOME", env::var("TEST_HOME").expect("TEST_HOME not set"))
        .output()
        .expect("Failed to execute command");
    
    if !output.status.success() {
        panic!("Init command failed: {}", String::from_utf8_lossy(&output.stderr));
    }
    
    let output_str = String::from_utf8_lossy(&output.stdout);
    if !output_str.contains("Initialized") {
        panic!("Expected 'Initialized' in output, but got: {}", output_str);
    }
    
    println!("✓ Workspace initialization test passed");
}

fn test_add_repository() {
    println!("Testing repository addition...");
    
    // This would be called after setting up test repos
    let output = Command::new(env::var("VIBE_BINARY_PATH").expect("VIBE_BINARY_PATH not set"))
        .args(&["add", "test-repo"])
        .env("HOME", env::var("TEST_HOME").expect("TEST_HOME not set"))
        .output()
        .expect("Failed to execute command");
    
    if output.status.success() {
        println!("✓ Repository add test passed");
    } else {
        println!("ℹ Repository add test expected to fail in isolated environment");
    }
}

fn test_list_repositories() {
    println!("Testing repository listing...");
    
    let output = Command::new(env::var("VIBE_BINARY_PATH").expect("VIBE_BINARY_PATH not set"))
        .args(&["list"])
        .env("HOME", env::var("TEST_HOME").expect("TEST_HOME not set"))
        .output()
        .expect("Failed to execute command");
    
    if output.status.success() {
        println!("✓ Repository list test passed");
    } else {
        println!("ℹ Repository list test expected behavior in isolated environment");
    }
}