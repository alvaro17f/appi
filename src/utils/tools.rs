use anyhow::Result;
use std::env;
use std::process::Command;

pub struct Tools;

impl Tools {
    pub fn clear(&self) -> Result<()> {
        Command::new("clear").status()?;
        Ok(())
    }

    pub fn get_user(&self) -> Result<String> {
        Ok(env::var("USER")?)
    }
}
