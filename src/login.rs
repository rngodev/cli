use crate::util::config::{Config, get_config, set_config};
use anyhow::Result;
use inquire::Password;

pub async fn login() -> Result<()> {
    let config = get_config()?;

    let api_key = Password::new("API Key:").without_confirmation().prompt()?;

    let new_config = Config {
        api_key: Some(api_key),
        ..config
    };

    set_config(new_config)?;
    println!("Successfully logged in");
    Ok(())
}
