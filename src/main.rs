use anyhow::Error;

use crate::{
    command::{Command, CommandType},
    util::RushError,
};
use std::io::{self, Write};

mod command;
mod util;

fn rush() -> Result<(), RushError> {
    print!("$ ");
    io::stdout()
        .flush()
        .map_err(|err| RushError::InternalError(Error::new(err)))?;

    let stdin = io::stdin().lock();
    let cmd = Command::new(stdin)?;

    if let CommandType::Exit = cmd.type_ {
        std::process::exit(0);
    }

    cmd.run()
}

fn main() -> Result<(), RushError> {
    loop {
        match rush() {
            Ok(_) => {}
            Err(error) => eprintln!("{error}"),
        }
    }
}
