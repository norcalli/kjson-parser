// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err)]

use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;

#[derive(Debug, From)]
enum Error {
    Io(std::io::Error),
    ChanRecv(chan::RecvError),
    Validation(ValidationError),
}

fn validator_entrypoint(input: &str) -> Result<(), Error> {
    let mut validator = Validator::new();

    let mut section = Section::new(input);
    let mut last_state = ValidationState::Incomplete;
    while let Ok(Some(token)) = next_token(&mut section) {
        if token.is_whitespace() {
            continue;
        }
        last_state = validator.process_token(&token).unwrap();
    }
    assert_eq!(last_state, ValidationState::Complete);
    Ok(())
}

fn eager_reformat_entrypoint(input: &str) -> Result<(), Error> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut stdout = io::BufWriter::new(stdout);
    let mut validator = Validator::new();

    let mut section = Section::new(input);
    while let Ok(Some(token)) = compress_next_token(&mut section, is_whitespace) {
        if token.is_whitespace() {
            continue;
        }
        token.print(&mut stdout)?;
        if ValidationState::Complete == validator.process_token(&token)? {
            write!(stdout, "{}", '\n')?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut stdin = stdin();
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;
    eager_reformat_entrypoint(&buffer)

    // tty_entrypoint()
}
