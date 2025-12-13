use crate::util::Command;
use std::io::{self, Write};

mod util;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let stdin = io::stdin().lock();
        let cmd = Command::new(stdin).unwrap();
        match cmd.type_ {
            util::CommandType::Exit => break,
            util::CommandType::Unknown => {
                println!("{}: command not found", cmd.cmd_str)
            }
        }
    }
}
