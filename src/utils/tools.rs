use anyhow::Result;
use std::env;
use std::process::Command;

pub fn clear() -> Result<()> {
    Command::new("clear");
    Ok(())
}

pub fn get_user() -> Result<String> {
    Ok(env::var("USER")?)
}
