mod chip8;
mod display;

use chip8::Chip8;
use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Please provide a path.");
        return;
    }
    let c8 = Chip8::new(&args[1]);
    match c8 {
        Ok(mut c) => c.run(),
        Err(e) => println!("{}", e),
    }
}
