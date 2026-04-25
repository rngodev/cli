use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub async fn init() -> Result<()> {
    let current_dir = std::env::current_dir().context("Failed to get current directory")?;
    let dir_name = current_dir
        .file_name()
        .and_then(|name| name.to_str())
        .context("Failed to get directory name")?;

    // Create .rngo directory
    let rngo_dir = current_dir.join(".rngo");
    if !rngo_dir.exists() {
        fs::create_dir_all(&rngo_dir).context("Failed to create .rngo directory")?;
    }

    // Create spec.yml file
    let spec_path = rngo_dir.join("spec.yml");
    if spec_path.exists() {
        println!(".rngo/spec.yml already exists");
    } else {
        let spec_content = format!(
            r#"key: {}
seed: 1"#,
            dir_name
        );

        fs::write(&spec_path, spec_content).context("Failed to write spec.yml")?;
    }

    // Ensure .gitignore has .rngo/simulations
    ensure_gitignore_entry(&current_dir)?;

    println!("Successfully initialized for rngo!");
    println!("For next steps, see https://rngo.dev/docs/guides/application-setup");
    Ok(())
}

fn ensure_gitignore_entry(project_dir: &Path) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let entry = ".rngo/simulations";

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path).context("Failed to read .gitignore")?;

        if content.lines().any(|line| line.trim() == entry) {
            return Ok(());
        }

        // Append the entry
        let new_content = if content.ends_with('\n') {
            format!("{}{}\n", content, entry)
        } else {
            format!("{}\n{}\n", content, entry)
        };

        fs::write(&gitignore_path, new_content).context("Failed to update .gitignore")?;
    } else {
        // Create new .gitignore
        fs::write(&gitignore_path, format!("{}\n", entry))
            .context("Failed to create .gitignore")?;
    }

    Ok(())
}
