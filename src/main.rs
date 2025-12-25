use crate::{
    command::{Command, CommandType},
    util::RushError,
};
use std::io::{self, Write};

mod command;
mod util;

fn rush() -> Result<(), RushError> {
    print!("$ ");
    io::stdout().flush().map_err(|_| RushError::UnexpectedEOF)?;

    let stdin = io::stdin().lock();
    let cmd = Command::new(stdin)?;

    if let CommandType::Exit = cmd.type_ {
        std::process::exit(0);
    }

    cmd.run()
}

fn main() {
    loop {
        if let Err(error) = rush() {
            match error {
                RushError::Nop => {}
                error => eprintln!("{error}"),
            }
        }
    }
}
