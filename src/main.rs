use crate::util::{Command, CommandType};
use std::io::{self, Write};

mod util;

fn main() -> anyhow::Result<()> {
    loop {
        print!("$ ");
        io::stdout().flush()?;

        let stdin = io::stdin().lock();
        let cmd = Command::new(stdin)?;

        if let CommandType::Exit = cmd.type_ {
            return Ok(());
        }

        if let Err(error) = cmd.run() {
            println!("{error}")
        }
    }
}
