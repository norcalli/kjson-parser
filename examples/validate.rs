// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err)]

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use parser::section::{ISection, Section};
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token, TokenizeError};
use parser::validator::{ValidationError, ValidationState, Validator};

use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;

#[derive(Debug, From)]
pub enum Error {
    Io(std::io::Error),
    Validation(ValidationError),
    Tokenizer(TokenizeError),
    ExtraInput,
    // InvalidString(std::str::Utf8Error),
    InvalidString,
    EmptyInput,
    InvalidNumber(std::num::ParseFloatError),
    InvalidInt(std::num::ParseIntError),
}

pub type Result<T> = std::result::Result<T, Error>;

// TODO make checking integers/extra input/empty input configurable errors.
// You can make a validator more permissive or less permissive depending on your
// preference for edge cases.
fn eager_reformat_entrypoint(input: &str) -> Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut stdout = io::BufWriter::new(stdout);
    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let mut section = Section::new(input);
    let mut had_tokens = false;
    loop {
        let token = compress_next_token(&mut section, is_whitespace)?;
        let token = if let Some(token) = token {
            token
        } else {
            break;
        };
        if token.is_whitespace() {
            continue;
        }
        last_state = validator.process_token(&token)?;
        had_tokens = true;
        match token {
            Token::String(ref s) => {
                json::parse(s).map_err(|_| Error::InvalidString)?;
            }
            Token::Number(ref s) => {
                // TODO check overflow/underflow/etc.
                let x: f64 = s.parse()?;
                // Extra testing for integers.
                if s.find(|c| c == 'e' || c == 'E' || c == '.').is_none() && x.floor() == x {
                    let _: i64 = s.parse()?;
                }
            }
            _ => (),
        }
        token.print(&mut stdout)?;
        if ValidationState::Complete == last_state {
            write!(stdout, "{}", '\n')?;
            break;
        }
    }
    validator.finish()?;
    while section.check_next_pattern(is_whitespace) {}
    if section.peek().is_some() {
        return Err(Error::ExtraInput);
    }
    if !had_tokens {
        return Err(Error::EmptyInput);
    }
    Ok(())
}

fn main() -> Result<()> {
    let mut stdin = stdin();
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;
    eager_reformat_entrypoint(&buffer)

    // tty_entrypoint()
}
