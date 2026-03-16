use clap::Args;

#[derive(Debug, Args)]
pub struct NewAppArgs {
    /// App name (e.g., loom_hr)
    pub app_name: String,
}

pub async fn run(args: NewAppArgs) -> anyhow::Result<()> {
    let app_dir = format!("apps/{}", args.app_name);

    if std::path::Path::new(&app_dir).exists() {
        anyhow::bail!("App '{}' already exists at {}/", args.app_name, app_dir);
    }

    let module = args
        .app_name
        .strip_prefix("loom_")
        .unwrap_or(&args.app_name);
    let module_title = to_title_case(module);
    let slug = args.app_name.replace('-', "_");

    tracing::info!("Scaffolding new app '{}'", args.app_name);

    // Create directory structure
    std::fs::create_dir_all(format!("{}/doctypes", app_dir))?;
    std::fs::create_dir_all(format!("{}/api", app_dir))?;
    std::fs::create_dir_all(format!("{}/scripts", app_dir))?;
    std::fs::create_dir_all(format!("{}/fixtures", app_dir))?;
    std::fs::create_dir_all(format!("{}/plugins", app_dir))?;
    std::fs::create_dir_all(format!("{}/frontend/src/pages", app_dir))?;

    // loom_app.toml
    std::fs::write(
        format!("{}/loom_app.toml", app_dir),
        format!(
            r#"[app]
name = "{name}"
version = "0.1.0"
title = "{title}"
description = "A Loom application"
modules = ["{module}"]
"#,
            name = slug,
            title = module_title,
            module = module_title,
        ),
    )?;

    // hooks.toml — with examples for all features
    std::fs::write(
        format!("{}/hooks.toml", app_dir),
        format!(
            r#"# {title} — App Hooks
#
# Documentation: https://github.com/anthropics/loom/docs

# Scheduled tasks (cron format: minute hour day month weekday)
# [[scheduler]]
# cron = "0 */6 * * *"
# method = "{slug}.sync_data"

# Document event hooks
# [[doc_events]]
# doctype = "Todo"
# event = "on_update"
# method = "scripts/on_todo_update.rhai"

# Named queues for background jobs
# [queues]
# names = ["default", "long"]

# Fixtures — exported with `loom export-fixtures`
[[fixtures]]
doctype = "Role"
filters = {{ module = "{module}" }}

# Plugin pages (custom frontend routes)
# [[pages]]
# route = "/app/dashboard"
# label = "Dashboard"
# component = "Dashboard"
"#,
            title = module_title,
            slug = slug,
            module = module_title,
        ),
    )?;

    // .gitkeep files for empty directories
    std::fs::write(format!("{}/doctypes/.gitkeep", app_dir), "")?;
    std::fs::write(format!("{}/api/.gitkeep", app_dir), "")?;
    std::fs::write(format!("{}/scripts/.gitkeep", app_dir), "")?;
    std::fs::write(format!("{}/fixtures/.gitkeep", app_dir), "")?;
    std::fs::write(format!("{}/plugins/.gitkeep", app_dir), "")?;

    // .gitignore
    std::fs::write(
        format!("{}/.gitignore", app_dir),
        "node_modules/\ndist/\n*.wasm\n",
    )?;

    // README
    std::fs::write(
        format!("{}/README.md", app_dir),
        format!(
            r#"# {title}

A Loom application.

## Install

```bash
loom --site mysite.localhost install-app {name}
```

## Development

Set developer mode in `sites/mysite.localhost/site_config.json`:

```json
{{
  "developer_mode": true
}}
```

Create DocTypes in the Desk UI — they auto-export to `doctypes/` in developer mode.

## Export Fixtures

```bash
loom export-fixtures
```

## Structure

```
{name}/
├── loom_app.toml     # App metadata
├── hooks.toml        # Scheduled tasks, fixtures, queues
├── doctypes/         # DocType definitions (JSON + Rhai scripts)
├── api/              # Whitelisted API methods (Rhai)
├── fixtures/         # Seed data (loaded on install)
├── scripts/          # Shared Rhai scripts
├── plugins/          # WASM plugins (optional)
└── frontend/         # Frontend extensions (optional)
```
"#,
            title = module_title,
            name = slug,
        ),
    )?;

    println!("Created app '{}' at {}/", args.app_name, app_dir);
    println!();
    println!("  Structure:");
    println!("    {}/loom_app.toml    — App metadata", app_dir);
    println!("    {}/hooks.toml      — Hooks, fixtures, queues", app_dir);
    println!("    {}/doctypes/       — DocType definitions (JSON + Rhai)", app_dir);
    println!("    {}/api/            — Whitelisted API methods", app_dir);
    println!("    {}/fixtures/       — Seed data", app_dir);
    println!("    {}/scripts/        — Shared Rhai scripts", app_dir);
    println!();
    println!("  Next steps:");
    println!("    1. Create DocTypes in the Desk UI at /app/DocType/new");
    println!("    2. Enable developer mode in site_config.json");
    println!("    3. DocType JSON files auto-export to {}/doctypes/", app_dir);
    println!("    4. loom --site mysite.localhost install-app {}", args.app_name);

    Ok(())
}

fn to_title_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut c = word.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().to_string() + c.as_str(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}
