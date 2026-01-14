use crate::util::config::set_user_config;
use anyhow::Result;
use inquire::Password;

pub async fn login() -> Result<()> {
    let api_key = Password::new("API Key:").without_confirmation().prompt()?;
    set_user_config(|config| config.api_key = Some(api_key))?;
    println!("Successfully logged in");
    Ok(())
}
