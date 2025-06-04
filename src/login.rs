use crate::util::{Config, set_config};
use anyhow::Result;

pub fn login(api_key: String) -> Result<()> {
    let config = crate::util::get_config()?;

    let new_config = Config {
        api_key: Some(api_key),
        ..config
    };

    set_config(new_config)?;
    println!("Successfully logged in");
    Ok(())
}
