use std::io::{self, BufRead};

pub fn process_input<R: BufRead>(mut reader: R) -> io::Result<String> {
    let mut input = String::new();
    reader.read_line(&mut input).unwrap();

    Ok(input.trim().into())
}
