use anyhow::Result;

pub fn login(api_key: String) -> Result<()> {
    let config = crate::util::get_config()?;
    println!("Found config {:?}", config);
    println!("Logging in with {}", api_key);
    Ok(())
}
