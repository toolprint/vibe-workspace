use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct VibeUri {
    pub scheme: String,  // "vibe"
    pub action: String,  // "github", "gitlab", etc.
    pub command: String, // "install", "search", etc.
    pub params: HashMap<String, String>,
}

impl VibeUri {
    pub fn new(action: String, command: String) -> Self {
        Self {
            scheme: "vibe".to_string(),
            action,
            command,
            params: HashMap::new(),
        }
    }

    pub fn with_params(mut self, params: HashMap<String, String>) -> Self {
        self.params = params;
        self
    }

    pub fn add_param(mut self, key: String, value: String) -> Self {
        self.params.insert(key, value);
        self
    }

    pub fn to_string(&self) -> String {
        let mut uri = format!("{}://{}/{}", self.scheme, self.action, self.command);

        if !self.params.is_empty() {
            let query_string: Vec<String> = self
                .params
                .iter()
                .map(|(k, v)| format!("{}={}", k, urlencoding::encode(v)))
                .collect();
            uri.push('?');
            uri.push_str(&query_string.join("&"));
        }

        uri
    }
}

// Supported URI schemes and their descriptions
pub const SUPPORTED_SCHEMES: &[(&str, &str)] = &[
    (
        "vibe://github/install/<org>/<repo>",
        "Install a GitHub repository",
    ),
    (
        "vibe://github/search?q=<query>",
        "Search GitHub repositories",
    ),
    (
        "vibe://workspace/open/<repo-name>",
        "Open a workspace repository",
    ),
    ("vibe://workspace/list", "List all workspace repositories"),
];
