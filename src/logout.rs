use crate::util::config::{Config, get_config, set_config};
use anyhow::Result;
use inquire::Confirm;

pub async fn logout() -> Result<()> {
    let config = get_config()?;

    if config.api_key.is_none() {
        println!("You are not logged in");
        return Ok(());
    }

    let confirmed = Confirm::new("Are you sure?").with_default(false).prompt()?;

    if !confirmed {
        println!("Log out cancelled");
        return Ok(());
    }

    let new_config = Config {
        api_key: None,
        ..config
    };

    set_config(new_config)?;
    println!("Successfully logged out");
    Ok(())
}
