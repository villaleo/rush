use crate::util::read_command;
use std::io::{self, Write};

mod util;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let stdin = io::stdin().lock();
        let cmd = read_command(stdin).unwrap();
        match cmd.type_ {
            util::CommandType::Exit => break,
            util::CommandType::Unknown => println!("{}: unknown command", cmd.cmd_str),
        }
    }
}
