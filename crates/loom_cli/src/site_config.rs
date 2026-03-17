use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct SiteConfig {
    pub db_name: Option<String>,
    pub db_type: Option<String>,
    pub db_host: Option<String>,
    pub db_port: Option<u16>,
    pub db_user: Option<String>,
    pub db_password: Option<String>,
    pub developer_mode: Option<bool>,
}

impl SiteConfig {
    /// Build a PostgreSQL connection URL from site config fields.
    pub fn db_url(&self) -> String {
        let host = self.db_host.as_deref().unwrap_or("localhost");
        let port = self.db_port.unwrap_or(5432);
        let name = self.db_name.as_deref().unwrap_or("loom");
        let user = self.db_user.as_deref().unwrap_or("postgres");
        let password = self.db_password.as_deref().unwrap_or("");

        if password.is_empty() {
            format!("postgres://{}@{}:{}/{}", user, host, port, name)
        } else {
            format!(
                "postgres://{}:{}@{}:{}/{}",
                user, password, host, port, name
            )
        }
    }
}

/// Load site config from the first site found in the sites/ directory,
/// or from a specific site if provided.
pub fn load_site_config(site: Option<&str>) -> Option<SiteConfig> {
    let sites_dir = find_sites_dir()?;

    if let Some(site_name) = site {
        let path = sites_dir.join(site_name).join("site_config.json");
        return load_config_file(&path);
    }

    // Auto-detect: use the first (or only) site
    let entries: Vec<_> = std::fs::read_dir(&sites_dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir() && e.path().join("site_config.json").exists())
        .collect();

    if entries.len() == 1 {
        let path = entries[0].path().join("site_config.json");
        return load_config_file(&path);
    }

    if entries.len() > 1 {
        tracing::warn!(
            "Multiple sites found. Use --site to specify. Found: {}",
            entries
                .iter()
                .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    None
}

/// Resolve the database URL: explicit --db-url > site_config.json > DATABASE_URL env > default.
pub fn resolve_db_url(explicit: Option<String>, site: Option<&str>) -> String {
    // 1. Explicit --db-url flag
    if let Some(url) = explicit {
        return url;
    }

    // 2. Site config
    if let Some(config) = load_site_config(site) {
        return config.db_url();
    }

    // 3. Default fallback
    "postgres://postgres@localhost/loom".to_string()
}

fn load_config_file(path: &std::path::Path) -> Option<SiteConfig> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn find_sites_dir() -> Option<std::path::PathBuf> {
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let sites = dir.join("sites");
        if sites.is_dir() {
            return Some(sites);
        }
        if !dir.pop() {
            return None;
        }
    }
}
