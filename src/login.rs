use crate::util::{Config, set_config};
use anyhow::Result;
use inquire::Password;

pub async fn login() -> Result<()> {
    let config = crate::util::get_config()?;

    let api_key = Password::new("API Key:").without_confirmation().prompt()?;

    let new_config = Config {
        api_key: Some(api_key),
        ..config
    };

    set_config(new_config)?;
    println!("Successfully logged in");
    Ok(())
}
