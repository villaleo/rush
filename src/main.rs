use crate::util::{find_command, read_command};
use std::io::{self, Write};

mod util;

fn main() {
    loop {
        print!("$ ");
        io::stdout().flush().unwrap();

        let stdin = io::stdin().lock();
        let cmd = read_command(stdin);
        let _ = find_command(&cmd.name);
    }
}
