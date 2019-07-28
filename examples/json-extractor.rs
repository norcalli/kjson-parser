// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err, clippy::all)]

// use parser::section::Section;
use parser::section::ByteSection;
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token, TokenizeError};
use parser::validator::{ValidationError, ValidationState, Validator};

use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;
use log::*;

#[derive(Debug, From)]
pub enum Error {
    Io(std::io::Error),
    TokenizerError(TokenizeError),
    // Validation(ValidationError),
}

pub type Result<T> = std::result::Result<T, Error>;

const GREEDY: bool = true;

fn entrypoint<'a>(input: &'a [u8]) -> Result<()> {
    let stdout = stdout();
    let stdout = stdout.lock();
    let mut stdout = io::BufWriter::new(stdout);

    let mut validator = Validator::new();
    let mut tokens: Vec<Token<'a>> = Vec::new();

    let mut section = ByteSection::new(input);

    let mut last_start = section.n;

    loop {
        match compress_next_token(&mut section, is_whitespace) {
            Ok(token) => {
                match validator.process_token(&token) {
                    // On completion, add the final token, and write the stored tokens to output
                    Ok(ValidationState::Complete) => {
                        tokens.push(token.into_owned());
                        for token in tokens.drain(..) {
                            token.print(&mut stdout)?;
                        }
                        stdout.write_all(b"\n")?;
                    }

                    Ok(ValidationState::Incomplete) => {
                        tokens.push(token.into_owned());
                    }

                    Ok(ValidationState::Ignored) => {}

                    Err(err) => {
                        // TODO make this an option that can be enabled/disabled
                        // Go back through our token stack to print any values which could
                        // be valid tokens on standalone, aka Number/String/Null/True/False

                        validator.reset();
                        if GREEDY {
                            if token.is_complete_value() {
                                token.print(&mut stdout)?;
                                stdout.write_all(b"\n")?;
                            }
                            for token in tokens.drain(..) {
                                if token.is_complete_value() {
                                    token.print(&mut stdout)?;
                                    stdout.write_all(b"\n")?;
                                }
                            }
                        } else {
                            tokens.clear();
                        }
                    }
                }
            }

            // On an unexpected byte, we should retry parsing at this point
            Err(TokenizeError::UnexpectedByteWithContext { .. })
            | Err(TokenizeError::UnexpectedByte(_)) => {
                validator.reset();
                tokens.clear();

                // The byte was consumed, so we have to rewind one.
                section.n -= 1;

                // We've already tried restarting, so don't try it again.
                if last_start == section.n {
                    section.n += 1;
                }

                last_start = section.n;
            }

            Err(ref err) if err.is_eof() => {
                match validator.finish() {
                    Ok(ValidationState::Complete) => {
                        for token in tokens.drain(..) {
                            token.print(&mut stdout)?;
                        }
                        stdout.write_all(b"\n")?;
                    }
                    Ok(_) => {}
                    Err(_) => {}
                }
                break;
            }

            // The only errors remaining are InvalidString* stuff, so we can skip these and
            // try the next token.
            Err(err) => {}
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    // env_logger::init();
    let mut stdin = stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer)?;
    entrypoint(&buffer)
}
