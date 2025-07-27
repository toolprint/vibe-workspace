use anyhow::Result;
use std::collections::HashMap;
use url::Url;

pub mod handler;
pub mod schemes;

pub use schemes::VibeUri;

pub fn parse_vibe_uri(uri_str: &str) -> Result<VibeUri> {
    let url = Url::parse(uri_str)?;

    if url.scheme() != "vibe" {
        anyhow::bail!(
            "Invalid URI scheme: expected 'vibe', got '{}'",
            url.scheme()
        );
    }

    let host = url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Missing host in URI"))?;
    let path_segments: Vec<&str> = url
        .path_segments()
        .ok_or_else(|| anyhow::anyhow!("Invalid URI path"))?
        .collect();

    let command = path_segments
        .first()
        .ok_or_else(|| anyhow::anyhow!("Missing command in URI path"))?
        .to_string();

    let mut params = HashMap::new();
    for (key, value) in url.query_pairs() {
        params.insert(key.to_string(), value.to_string());
    }

    // Add path segments as parameters if present
    if path_segments.len() > 1 {
        params.insert("path".to_string(), path_segments[1..].join("/"));
    }

    Ok(VibeUri {
        scheme: url.scheme().to_string(),
        action: host.to_string(),
        command,
        params,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vibe_uri() {
        let uri = parse_vibe_uri("vibe://github/install/rust-lang/rust").unwrap();
        assert_eq!(uri.action, "github");
        assert_eq!(uri.command, "install");
        assert_eq!(uri.params.get("path"), Some(&"rust-lang/rust".to_string()));

        let uri = parse_vibe_uri("vibe://github/search?q=rust+web").unwrap();
        assert_eq!(uri.action, "github");
        assert_eq!(uri.command, "search");
        assert_eq!(uri.params.get("q"), Some(&"rust web".to_string()));
    }
}
