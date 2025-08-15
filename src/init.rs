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
        println!("Created .rngo directory");
    }

    // Create spec.yml file
    let spec_path = rngo_dir.join("spec.yml");
    if spec_path.exists() {
        println!("spec.yml already exists, skipping creation");
    } else {
        let spec_content = format!(
            r#"key: {}
seed: 1
entities:
  # Define your entities here
  # Example:
  # users:
  #   stream:
  #     type: object
  #     properties:
  #       id:
  #         type: integer
  #       name:
  #         type: string
"#,
            dir_name
        );

        fs::write(&spec_path, spec_content).context("Failed to write spec.yml")?;
        println!("Created .rngo/spec.yml with key: {}", dir_name);
    }

    // Ensure .gitignore has .rngo/simulations
    ensure_gitignore_entry(&current_dir)?;

    println!("Project initialized successfully!");
    Ok(())
}

fn ensure_gitignore_entry(project_dir: &Path) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let entry = ".rngo/simulations";

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path).context("Failed to read .gitignore")?;

        if content.lines().any(|line| line.trim() == entry) {
            println!(".gitignore already contains .rngo/simulations");
            return Ok(());
        }

        // Append the entry
        let new_content = if content.ends_with('\n') {
            format!("{}{}\n", content, entry)
        } else {
            format!("{}\n{}\n", content, entry)
        };

        fs::write(&gitignore_path, new_content).context("Failed to update .gitignore")?;
        println!("Added .rngo/simulations to .gitignore");
    } else {
        // Create new .gitignore
        fs::write(&gitignore_path, format!("{}\n", entry))
            .context("Failed to create .gitignore")?;
        println!("Created .gitignore with .rngo/simulations");
    }

    Ok(())
}
