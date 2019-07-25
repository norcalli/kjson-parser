// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err)]

// use parser::section::Section;
use parser::section::{ISection, Section};
use parser::tokenizer::{
    compress_next_token, utils::is_whitespace, Token, TokenizeError, TokenizeResult,
};
use parser::validator::{ValidationError, ValidationState, Validator};

use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;
use log::*;

#[derive(Debug, From)]
pub enum Error {
    Io(std::io::Error),
    Validation(ValidationError),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Default)]
struct Extractor<W: io::Write> {
    tokens: Vec<Token<'static>>,
    validator: Validator,
    stdout: W,
}

impl<W: io::Write> Extractor<W> {
    pub fn check_token(
        &mut self,
        token: TokenizeResult<Option<Token<'_>>>,
        mut section: &mut Section<'_>,
    ) -> Result<bool> {
        match token {
            Ok(Some(token)) => {
                if token.is_whitespace() {
                    return Ok(true);
                }
                self.check_validation(token)?;
            }
            Ok(None) => {
                return Ok(false);
            }
            Err(TokenizeError::UnexpectedEndOfInput) => {
                return Ok(false);
            }
            Err(TokenizeError::UnexpectedCharacter(c)) => {
                debug!("unexpected char = {}", c);
                // let mut section = section.clone();
                // section.next();
                self.tokens.clear();
                self.validator.reset();

                let micro_input = format!("{}", c);
                let mut micro_section = Section::new(micro_input.as_str());
                let micro_token = compress_next_token(&mut micro_section, is_whitespace);
                // if token.is_ok() {
                if let Ok(Some(ref token)) = micro_token {
                    if token.partial_false_positive() {
                        let token = compress_next_token(&mut section, is_whitespace);
                        return self.check_token(token, &mut section);
                    } else {
                        section.next();
                        return self.check_token(micro_token, &mut micro_section);
                    }
                } else {
                    section.next();
                    return Ok(true);
                    // return self.check_token(token, &mut section);
                }
            }
            // TODO make sure the other tokenizer error cases don't consume.
            Err(_) => {
                section.next();
                self.tokens.clear();
            }
        }
        Ok(true)
    }

    pub fn check_validation(&mut self, token: Token<'_>) -> Result<()> {
        match self.validator.process_token(&token) {
            Ok(ValidationState::Complete) => {
                self.tokens.push(token.into_owned());
                debug!("Flushing tokens: {:?}", self.tokens);
                for token in self.tokens.drain(..) {
                    token.print(&mut self.stdout)?;
                }
                self.stdout.write_all(b"\n")?;
            }
            Ok(ValidationState::Incomplete) => {
                debug!("Adding token: {:?}", token);
                self.tokens.push(token.into_owned());
            }
            Ok(ValidationState::Ignored) => {}
            Err(_) => {
                debug!("Clearing tokens: {:?}", self.tokens);
                self.tokens.clear();
            }
        }
        Ok(())
    }
}

fn eager_reformat_entrypoint<'a>(input: &'a str) -> Result<()> {
    let stdout = stdout();
    let stdout = stdout.lock();
    let mut stdout = io::BufWriter::new(stdout);

    let mut extractor = Extractor {
        stdout,
        tokens: Default::default(),
        validator: Default::default(),
    };

    let mut section = Section::new(input);
    loop {
        debug!("peek = {:?}", section.peek());
        let result = compress_next_token(&mut section, is_whitespace);

        debug!("next_token = {:?}", result);

        if !extractor.check_token(result, &mut section)? {
            break;
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();
    let mut stdin = stdin();
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;
    eager_reformat_entrypoint(&buffer)
}
