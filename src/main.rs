use crate::util::process_input;
use std::io::{self, Write};

mod util;

fn main() {
    print!("$ ");
    io::stdout().flush().unwrap();

    let _ = find_command(&read_command());
}

fn read_command() -> String {
    let stdin = io::stdin().lock();

    match process_input(stdin) {
        Ok(input) => input,
        Err(_) => unreachable!(),
    }
}

fn find_command(name: &str) -> Option<()> {
    // All commands are unknown for now
    println!("{name}: command not found");
    None
}
