// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err)]

use parser::section::Section;
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token};
use parser::validator::{ValidationError, ValidationState, Validator};

use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;

#[derive(Debug, From)]
enum Error {
    Io(std::io::Error),
    Validation(ValidationError),
}

// Look at other combinators for inspiration on how to do
// stronger typing

/// Two possible designs:
/// - Building subtraversal into lenses
/// - Or using lists to represent the recursion

trait Lens {}

enum JsonPath {
    Object(String),
    Array(u64),
}

struct Traverser {
    path: Vec<JsonPath>,
}

impl Traverser {}

fn eager_reformat_entrypoint<'a>(input: &'a str) -> Result<(), Error> {
    let stdout = stdout();
    let stdout = stdout.lock();
    let mut stdout = io::BufWriter::new(stdout);
    let mut validator = Validator::new();

    let mut tokens: Vec<Token<'a>> = Vec::new();

    let mut section = Section::new(input);
    while let Ok(Some(token)) = compress_next_token(&mut section, is_whitespace) {
        if token.is_whitespace() {
            continue;
        }
        match validator.process_token(&token) {
            Ok(ValidationState::Complete) => {
                tokens.push(token);
                for token in tokens.drain(..) {
                    token.print(&mut stdout)?;
                }
                stdout.write_all(b"\n")?;
            }
            Ok(ValidationState::Incomplete) => {
                tokens.push(token);
            }
            Ok(ValidationState::Ignored) => {}
            Err(_) => {
                tokens.clear();
            }
        }
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut stdin = stdin();
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;
    eager_reformat_entrypoint(&buffer)
}

