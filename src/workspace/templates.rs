use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

use crate::workspace::{Repository, WorkspaceConfig};

/// Template manager for app configurations
pub struct TemplateManager {
    template_root: PathBuf,
}

impl TemplateManager {
    pub fn new(template_root: PathBuf) -> Self {
        Self { template_root }
    }

    /// Get the template directory for a specific app
    pub fn get_app_template_dir(&self, app: &str) -> PathBuf {
        self.template_root.join(app)
    }

    /// List available templates for an app
    pub async fn list_templates(&self, app: &str) -> Result<Vec<String>> {
        let template_dir = self.get_app_template_dir(app);

        if !template_dir.exists() {
            return Ok(vec![]);
        }

        let mut templates = Vec::new();
        let mut entries = fs::read_dir(&template_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.is_file() {
                if let Some(name) = path.file_stem() {
                    templates.push(name.to_string_lossy().to_string());
                }
            }
        }

        templates.sort();
        Ok(templates)
    }

    /// Load a template file
    pub async fn load_template(&self, app: &str, template_name: &str) -> Result<String> {
        let template_path = self
            .get_app_template_dir(app)
            .join(format!("{template_name}.yaml"));

        if !template_path.exists() {
            // Try with .json extension for iTerm2
            let json_path = self
                .get_app_template_dir(app)
                .join(format!("{template_name}.json"));

            if json_path.exists() {
                return fs::read_to_string(&json_path)
                    .await
                    .context("Failed to read template file");
            }

            // Try with .lua extension for WezTerm
            let lua_path = self
                .get_app_template_dir(app)
                .join(format!("{template_name}.lua"));

            if lua_path.exists() {
                return fs::read_to_string(&lua_path)
                    .await
                    .context("Failed to read template file");
            }

            anyhow::bail!("Template '{}' not found for app '{}'", template_name, app);
        }

        fs::read_to_string(&template_path)
            .await
            .context("Failed to read template file")
    }

    /// Save a template file
    pub async fn save_template(&self, app: &str, template_name: &str, content: &str) -> Result<()> {
        let template_dir = self.get_app_template_dir(app);

        // Create template directory if it doesn't exist
        if !template_dir.exists() {
            fs::create_dir_all(&template_dir).await?;
        }

        let extension = match app {
            "iterm2" => "json",
            "wezterm" => "lua",
            _ => "yaml",
        };

        let template_path = template_dir.join(format!("{template_name}.{extension}"));

        fs::write(&template_path, content)
            .await
            .context("Failed to write template file")
    }

    /// Delete a template file
    pub async fn delete_template(&self, app: &str, template_name: &str) -> Result<()> {
        let template_path = self
            .get_app_template_dir(app)
            .join(format!("{template_name}.yaml"));

        if !template_path.exists() {
            let json_path = self
                .get_app_template_dir(app)
                .join(format!("{template_name}.json"));

            if json_path.exists() {
                fs::remove_file(&json_path).await?;
                return Ok(());
            }

            let lua_path = self
                .get_app_template_dir(app)
                .join(format!("{template_name}.lua"));

            if lua_path.exists() {
                fs::remove_file(&lua_path).await?;
                return Ok(());
            }

            anyhow::bail!("Template '{}' not found for app '{}'", template_name, app);
        }

        fs::remove_file(&template_path).await?;
        Ok(())
    }

    /// Initialize default templates
    pub async fn init_default_templates(&self) -> Result<()> {
        // Create default Warp template
        self.save_template("warp", "default", DEFAULT_WARP_TEMPLATE)
            .await?;

        // Create default iTerm2 template
        self.save_template("iterm2", "default", DEFAULT_ITERM2_TEMPLATE)
            .await?;

        // Create default WezTerm template
        self.save_template("wezterm", "default", DEFAULT_WEZTERM_TEMPLATE)
            .await?;

        // Create default VS Code template
        self.save_template("vscode", "default", DEFAULT_VSCODE_TEMPLATE)
            .await?;

        // Create default Cursor template
        self.save_template("cursor", "default", DEFAULT_CURSOR_TEMPLATE)
            .await?;

        // Create default Windsurf template
        self.save_template("windsurf", "default", DEFAULT_WINDSURF_TEMPLATE)
            .await?;

        Ok(())
    }

    /// Apply variable substitution to a template
    pub fn substitute_variables(
        &self,
        template: &str,
        variables: &HashMap<String, String>,
    ) -> String {
        let mut result = template.to_string();

        for (key, value) in variables {
            let placeholder = format!("{{{{{key}}}}}");
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Create variables for template substitution
    pub fn create_variables(
        config: &WorkspaceConfig,
        repo: &Repository,
    ) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        // Common variables
        vars.insert("workspace_name".to_string(), config.workspace.name.clone());
        vars.insert("repo_name".to_string(), repo.name.clone());
        vars.insert(
            "repo_path".to_string(),
            config.workspace.root.join(&repo.path).display().to_string(),
        );
        vars.insert(
            "repo_branch".to_string(),
            repo.branch.clone().unwrap_or_else(|| "main".to_string()),
        );

        // Add repo URL if available
        if let Some(url) = &repo.url {
            vars.insert("repo_url".to_string(), url.clone());
        }

        // Add configurable command variables with defaults
        vars.insert("agent_launcher".to_string(), "claude".to_string());
        vars.insert("git_manager".to_string(), "gitui".to_string());
        vars.insert("project_commands".to_string(), "just".to_string());

        vars
    }
}

// Default template for Warp
pub const DEFAULT_WARP_TEMPLATE: &str = r#"---
# Warp Launch Configuration
# This configuration opens a tab with 3 panes:
# - Left pane: {{agent_launcher}} (takes full height)
# - Right side split horizontally: {{git_manager}} (top) and {{project_commands}} (bottom)

name: {{workspace_name}} - {{repo_name}}
windows:
  - tabs:
      - title: {{repo_name}} Development
        layout:
          split_direction: horizontal
          panes:
            - cwd: {{repo_path}}
              commands:
                - exec: {{agent_launcher}}
            - split_direction: vertical
              panes:
                - cwd: {{repo_path}}
                  commands:
                    - exec: {{git_manager}}
                - cwd: {{repo_path}}
                  commands:
                    - exec: {{project_commands}}
        color: blue
"#;

// Default template for iTerm2
pub const DEFAULT_ITERM2_TEMPLATE: &str = r#"{
  "Profiles": [{
    "Name": "{{workspace_name}} - {{repo_name}}",
    "Guid": "vibe-{{workspace_name}}-{{repo_name}}",
    "Working Directory": "{{repo_path}}",
    "Custom Directory": "Yes",
    "Badge Text": "📁 {{repo_name}} Development",
    "Use Custom Tab Title": true,
    "Custom Tab Title": "{{repo_name}} Development",
    "Tab Color": {
      "Red Component": 0.2,
      "Green Component": 0.4,
      "Blue Component": 0.8,
      "Alpha Component": 1.0
    },
    "Background Color": {
      "Red Component": 0.05,
      "Green Component": 0.05,
      "Blue Component": 0.1,
      "Alpha Component": 1.0
    },
    "Foreground Color": {
      "Red Component": 0.9,
      "Green Component": 0.9,
      "Blue Component": 0.9,
      "Alpha Component": 1.0
    },
    "Initial Text": "echo '🚀 {{repo_name}} Development Environment'\\necho '📍 Path: {{repo_path}}'\\necho '🌿 Branch: {{repo_branch}}'\\necho ''\\necho '📝 3-Pane Setup Instructions:'\\necho ''\\necho '   1. This pane should run: {{agent_launcher}}'\\necho '   2. Press Cmd+D to split vertically'\\necho '   3. In the right pane, run: {{git_manager}}'\\necho '   4. Press Cmd+Shift+D to split horizontally'\\necho '   5. In the bottom-right pane, run: {{project_commands}}'\\necho ''\\necho '⚡ Commands to run:'\\necho '   • Left pane: {{agent_launcher}}'\\necho '   • Top-right pane: {{git_manager}}'\\necho '   • Bottom-right pane: {{project_commands}}'\\necho ''\\necho '💡 Tip: Copy these commands before creating panes!'",
    "Send Text at Start": true,
    "Tags": ["vibe", "{{workspace_name}}", "{{repo_name}}"]
  }]
}"#;

// Default iTermocil template for iTerm2 (YAML format)
pub const DEFAULT_ITERMOCIL_TEMPLATE: &str = r#"# iTermocil Layout Configuration
# This creates a 3-pane layout similar to Warp:
# - Left pane: {{agent_launcher}}
# - Right side split vertically: {{git_manager}} (top) and {{project_commands}} (bottom)

windows:
  - name: {{workspace_name}} - {{repo_name}}
    root: {{repo_path}}
    layout: main-vertical
    panes:
      - name: "Agent Launcher"
        commands:
          - echo '🚀 Starting {{agent_launcher}}...'
          - {{agent_launcher}}
      - name: "Git Manager"
        commands:
          - echo '📊 Starting {{git_manager}}...'
          - {{git_manager}}
      - name: "Project Commands"
        commands:
          - echo '⚡ Starting {{project_commands}}...'
          - {{project_commands}}
"#;

// Default weztermocil template for WezTerm (YAML format)
pub const DEFAULT_WEZTERMOCIL_TEMPLATE: &str = r#"# weztermocil Layout Configuration
# This creates a 3-pane layout:
# - Left pane: {{agent_launcher}} (takes full height)
# - Right side split horizontally: {{git_manager}} (top) and {{project_commands}} (bottom)

windows:
  - name: {{workspace_name}} - {{repo_name}}
    root: {{repo_path}}
    layout: main-vertical
    panes:
      - name: "Agent Launcher"
        commands:
          - echo '🚀 Starting {{agent_launcher}}...'
          - {{agent_launcher}}
      - name: "Git Manager"
        commands:
          - echo '📊 Starting {{git_manager}}...'
          - {{git_manager}}
      - name: "Project Commands"
        commands:
          - echo '⚡ Starting {{project_commands}}...'
          - {{project_commands}}
"#;

// Default Lua template for WezTerm
pub const DEFAULT_WEZTERM_TEMPLATE: &str = r#"-- WezTerm Configuration
-- This creates a 3-pane layout:
-- - Left pane: {{agent_launcher}} (takes full height)
-- - Right side split horizontally: {{git_manager}} (top) and {{project_commands}} (bottom)

local wezterm = require 'wezterm'
local mux = wezterm.mux
local config = wezterm.config_builder()

-- Basic configuration
config.initial_cols = 180
config.initial_rows = 48
config.font_size = 12

wezterm.on('gui-startup', function()
  local tab, pane, window = mux.spawn_window {
    cwd = '{{repo_path}}',
  }
  
  -- Set window title
  window:gui_window():set_title('{{workspace_name}} - {{repo_name}}')
  
  -- Create right pane (50% width)
  local right_pane = pane:split {
    direction = 'Right',
    size = 0.5,
    cwd = '{{repo_path}}',
  }
  
  -- Split right pane horizontally
  local bottom_right = right_pane:split {
    direction = 'Bottom',
    size = 0.5,
    cwd = '{{repo_path}}',
  }
  
  -- Send welcome messages and instructions to each pane
  pane:send_text 'echo "🚀 {{repo_name}} Development Environment"\n'
  pane:send_text 'echo "📍 Path: {{repo_path}}"\n'
  pane:send_text 'echo "🌿 Branch: {{repo_branch}}"\n'
  pane:send_text 'echo ""\n'
  pane:send_text 'echo "💡 This pane is for: {{agent_launcher}}"\n'
  pane:send_text 'echo "Run: {{agent_launcher}}"\n'
  
  right_pane:send_text 'echo "💡 This pane is for: {{git_manager}}"\n'
  right_pane:send_text 'echo "Run: {{git_manager}}"\n'
  
  bottom_right:send_text 'echo "💡 This pane is for: {{project_commands}}"\n'
  bottom_right:send_text 'echo "Run: {{project_commands}}"\n'
end)

return config
"#;

// Default template for VS Code
pub const DEFAULT_VSCODE_TEMPLATE: &str = r#"{
  "folders": [
    {
      "name": "{{repo_name}}",
      "path": "{{repo_path}}"
    }
  ],
  "settings": {
    "window.title": "{{repo_name}} - {{workspace_name}}",
    "git.autoRepositoryDetection": true,
    "git.autorefresh": true,
    "explorer.fileNesting.enabled": true,
    "explorer.fileNesting.patterns": {
      "*.ts": "${capture}.js, ${capture}.d.ts.map, ${capture}.d.ts, ${capture}.js.map",
      "*.tsx": "${capture}.js, ${capture}.d.ts.map, ${capture}.d.ts, ${capture}.js.map",
      "package.json": "package-lock.json, yarn.lock, pnpm-lock.yaml, bun.lockb",
      "Cargo.toml": "Cargo.lock",
      "pyproject.toml": "poetry.lock, uv.lock",
      "README.md": "README.*, CHANGELOG.*, CONTRIBUTING.*, LICENSE.*"
    }
  },
  "extensions": {
    "recommendations": [
      "eamodio.gitlens",
      "mhutchie.git-graph",
      "ms-vscode.vscode-json",
      "redhat.vscode-yaml",
      "tamasfe.even-better-toml"
    ]
  }
}"#;

// Default template for Cursor
pub const DEFAULT_CURSOR_TEMPLATE: &str = r#"{
  "folders": [
    {
      "name": "{{repo_name}}",
      "path": "{{repo_path}}"
    }
  ],
  "settings": {
    "window.title": "{{repo_name}} - {{workspace_name}} (Cursor)",
    "git.autoRepositoryDetection": true,
    "git.autorefresh": true,
    "explorer.fileNesting.enabled": true,
    "explorer.fileNesting.patterns": {
      "*.ts": "${capture}.js, ${capture}.d.ts.map, ${capture}.d.ts, ${capture}.js.map",
      "*.tsx": "${capture}.js, ${capture}.d.ts.map, ${capture}.d.ts, ${capture}.js.map",
      "package.json": "package-lock.json, yarn.lock, pnpm-lock.yaml, bun.lockb",
      "Cargo.toml": "Cargo.lock",
      "pyproject.toml": "poetry.lock, uv.lock",
      "README.md": "README.*, CHANGELOG.*, CONTRIBUTING.*, LICENSE.*"
    },
    "cursor.aiCodeActionsEnabled": true,
    "cursor.aiReviewEnabled": true,
    "cursor.copilotMode": true
  },
  "extensions": {
    "recommendations": [
      "eamodio.gitlens",
      "mhutchie.git-graph",
      "ms-vscode.vscode-json",
      "redhat.vscode-yaml",
      "tamasfe.even-better-toml"
    ]
  }
}"#;

// Default template for Windsurf
pub const DEFAULT_WINDSURF_TEMPLATE: &str = r#"{
  "folders": [
    {
      "name": "{{repo_name}}",
      "path": "{{repo_path}}"
    }
  ],
  "settings": {
    "window.title": "{{repo_name}} - {{workspace_name}} (Windsurf)",
    "git.autoRepositoryDetection": true,
    "git.autorefresh": true,
    "explorer.fileNesting.enabled": true,
    "explorer.fileNesting.patterns": {
      "*.ts": "${capture}.js, ${capture}.d.ts.map, ${capture}.d.ts, ${capture}.js.map",
      "*.tsx": "${capture}.js, ${capture}.d.ts.map, ${capture}.d.ts, ${capture}.js.map",
      "package.json": "package-lock.json, yarn.lock, pnpm-lock.yaml, bun.lockb",
      "Cargo.toml": "Cargo.lock",
      "pyproject.toml": "poetry.lock, uv.lock",
      "README.md": "README.*, CHANGELOG.*, CONTRIBUTING.*, LICENSE.*"
    },
    "windsurf.aiFlowEnabled": true,
    "windsurf.agenticMode": true
  },
  "extensions": {
    "recommendations": [
      "eamodio.gitlens",
      "mhutchie.git-graph",
      "ms-vscode.vscode-json",
      "redhat.vscode-yaml",
      "tamasfe.even-better-toml"
    ]
  }
}"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_template_manager() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TemplateManager::new(temp_dir.path().to_path_buf());

        // Test init default templates
        manager.init_default_templates().await.unwrap();

        // Test list templates
        let warp_templates = manager.list_templates("warp").await.unwrap();
        assert!(warp_templates.contains(&"default".to_string()));

        // Test load template
        let template = manager.load_template("warp", "default").await.unwrap();
        assert!(template.contains("{{workspace_name}}"));

        // Test save custom template
        manager
            .save_template("warp", "custom", "test content")
            .await
            .unwrap();
        let custom = manager.load_template("warp", "custom").await.unwrap();
        assert_eq!(custom, "test content");

        // Test delete template
        manager.delete_template("warp", "custom").await.unwrap();
        let templates = manager.list_templates("warp").await.unwrap();
        assert!(!templates.contains(&"custom".to_string()));
    }

    #[test]
    fn test_variable_substitution() {
        let temp_dir = TempDir::new().unwrap();
        let manager = TemplateManager::new(temp_dir.path().to_path_buf());

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "test".to_string());
        vars.insert("path".to_string(), "/home/user".to_string());

        let template = "Hello {{name}}, your path is {{path}}";
        let result = manager.substitute_variables(template, &vars);

        assert_eq!(result, "Hello test, your path is /home/user");
    }
}
