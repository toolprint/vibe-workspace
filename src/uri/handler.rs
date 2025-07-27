use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;

use crate::git::{CloneCommand, GitConfig, SearchCommand};
use crate::uri::{parse_vibe_uri, VibeUri};
use crate::workspace::manager::WorkspaceManager;

#[async_trait]
pub trait UriHandler: Send + Sync {
    fn can_handle(&self, uri: &VibeUri) -> bool;
    async fn handle(&self, uri: &VibeUri) -> Result<()>;
}

pub struct UriRouter {
    handlers: Vec<Box<dyn UriHandler>>,
}

impl UriRouter {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Box<dyn UriHandler>) {
        self.handlers.push(handler);
    }

    pub async fn handle_uri(&self, uri_str: &str) -> Result<()> {
        let uri = parse_vibe_uri(uri_str)?;

        for handler in &self.handlers {
            if handler.can_handle(&uri) {
                return handler.handle(&uri).await;
            }
        }

        anyhow::bail!("No handler found for URI: {}", uri_str)
    }
}

// GitHub URI Handler
pub struct GitHubUriHandler {
    workspace_manager: Arc<tokio::sync::Mutex<WorkspaceManager>>,
    git_config: GitConfig,
}

impl GitHubUriHandler {
    pub fn new(
        workspace_manager: Arc<tokio::sync::Mutex<WorkspaceManager>>,
        git_config: GitConfig,
    ) -> Self {
        Self {
            workspace_manager,
            git_config,
        }
    }
}

#[async_trait]
impl UriHandler for GitHubUriHandler {
    fn can_handle(&self, uri: &VibeUri) -> bool {
        uri.action == "github"
    }

    async fn handle(&self, uri: &VibeUri) -> Result<()> {
        match uri.command.as_str() {
            "install" => {
                if let Some(path) = uri.params.get("path") {
                    let url = format!("https://github.com/{}", path);
                    let mut manager = self.workspace_manager.lock().await;
                    CloneCommand::execute(url, None, false, false, &mut manager, &self.git_config)
                        .await?;
                } else {
                    anyhow::bail!("Missing repository path in URI");
                }
            }
            "search" => {
                // For URI-based search, we'll just open the interactive search
                let mut manager = self.workspace_manager.lock().await;
                SearchCommand::execute_interactive(&mut manager, &self.git_config).await?;
            }
            _ => anyhow::bail!("Unknown GitHub command: {}", uri.command),
        }

        Ok(())
    }
}

// Platform-specific URI registration
pub fn register_uri_scheme(scheme: &str) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        macos::register_uri_scheme(scheme)
    }

    #[cfg(target_os = "linux")]
    {
        linux::register_uri_scheme(scheme)
    }

    #[cfg(target_os = "windows")]
    {
        windows::register_uri_scheme(scheme)
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        anyhow::bail!("URI scheme registration not supported on this platform")
    }
}

// macOS implementation
#[cfg(target_os = "macos")]
mod macos {
    use anyhow::Result;

    pub fn register_uri_scheme(scheme: &str) -> Result<()> {
        // For macOS, we need to update the Info.plist of the application
        // This is typically done during the build/install process
        // For now, we'll provide instructions

        eprintln!("To register the '{}' URI scheme on macOS:", scheme);
        eprintln!("1. Add the following to your Info.plist:");
        eprintln!("   <key>CFBundleURLTypes</key>");
        eprintln!("   <array>");
        eprintln!("     <dict>");
        eprintln!("       <key>CFBundleURLSchemes</key>");
        eprintln!("       <array>");
        eprintln!("         <string>{}</string>", scheme);
        eprintln!("       </array>");
        eprintln!("     </dict>");
        eprintln!("   </array>");
        eprintln!("2. Rebuild and reinstall the application");

        Ok(())
    }
}

// Linux implementation (stub)
#[cfg(target_os = "linux")]
mod linux {
    use anyhow::Result;

    pub fn register_uri_scheme(scheme: &str) -> Result<()> {
        // Linux implementation would create a .desktop file
        eprintln!(
            "Linux URI scheme registration not yet implemented for '{}'",
            scheme
        );
        Ok(())
    }
}

// Windows implementation (stub)
#[cfg(target_os = "windows")]
mod windows {
    use anyhow::Result;

    pub fn register_uri_scheme(scheme: &str) -> Result<()> {
        // Windows implementation would modify the registry
        eprintln!(
            "Windows URI scheme registration not yet implemented for '{}'",
            scheme
        );
        Ok(())
    }
}
