mod chip8;
mod display;
mod enums;

use anyhow::{bail, Result};
use chip8::Chip8;
use std::env;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        bail!("Please provide a path.");
    }
    let mut c8 = Chip8::new(&args[1])?;
    c8.run()?;
    Ok(())
}
