// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(clippy::all)]
#![warn(const_err)]

// #[global_allocator]
// static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

use parser::section::{ByteSection, PeekSeek};
use parser::tokenizer::{
    compress_next_token, utils, utils::is_whitespace, Token, TokenContext, TokenizeError,
};
use parser::validator::{ValidationError, ValidationState, Validator};

use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;
use log::*;

#[derive(Debug, From)]
pub enum Error {
    Io(std::io::Error),
    Validation(ValidationError),
    Tokenizer(TokenizeError),
    ExtraInput,
    InvalidStringUtf8(std::str::Utf8Error),
    InvalidString,
    EmptyInput,
    InvalidNumber(std::num::ParseFloatError),
    InvalidInt(std::num::ParseIntError),
}

pub type Result<T> = std::result::Result<T, Error>;

// const BUFFER_SIZE: usize = 4 * 1024 * 1024;
// const BUFFER_SIZE: usize = 1024;
const BUFFER_SIZE: usize = 1000;

// TODO make checking integers/extra input/empty input configurable errors.
// You can make a validator more permissive or less permissive depending on your
// preference for edge cases.
fn eager_reformat_entrypoint(input: &[u8]) -> Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    let mut stdout = io::BufWriter::new(stdout);

    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let mut section = ByteSection::new(input);

    let mut had_tokens = false;
    while !section.is_empty() {
        info!("section: {}", section);
        let token = compress_next_token(&mut section, is_whitespace)?;
        debug!("{:?}", token);
        if token.is_whitespace() {
            continue;
        }
        last_state = validator.process_token(&token)?;
        had_tokens = true;
        match token {
            Token::String(ref s) => {
                let s = std::str::from_utf8(s)?;
                json::parse(s).map_err(|_| Error::InvalidString)?;
            }
            Token::Number(ref s) => {
                let s = std::str::from_utf8(s)?;
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

fn finish_string_token<R: Read, W: Write>(
    stdin: &mut R,
    stdout: &mut W,
    mut input: &mut [u8],
) -> Result<(usize, usize)> {
    let mut input_length = input.len();
    loop {
        let mut section = ByteSection::new(&input[..input_length]);
        match utils::section_inside_string(&mut section) {
            Ok(()) => {
                stdout.write_all(&section.src[..section.n])?;
                return Ok((section.n, input_length));
            }
            Err(err) => {
                // Recoverable
                if let Some(recovery) = err.recovery_point() {
                    stdout.write_all(&input[..recovery])?;
                    let recovery_length = input_length - recovery;
                    debug!("Recovering: length {}", recovery_length);
                    input.copy_within(recovery.., 0);
                    input_length = stdin.read(&mut input[recovery_length..])?;
                    if input_length == 0 {
                        return Err(TokenizeError::UnexpectedEndOfInput.into());
                    }
                    input = &mut input[..input_length + recovery_length];
                } else {
                    return Err(err.into());
                }
            }
        }
        // println!("token start = {}, recovery = {}", token_start, recovery);
        // println!("input = {:?}", &input[token_start..recovery]);

        // stdout.write_all(&input[token_start..recovery])?;
        // input.copy_within(recovery.., 0);
        // let recovery_length = input_length - recovery;
        // input_length =
        //     stdin.read(&mut input[recovery_length..])? + recovery_length;
        // input.truncate(input_length);
    }
}

// TODO make checking integers/extra input/empty input configurable errors.
// You can make a validator more permissive or less permissive depending on your
// preference for edge cases.
fn chunked_entrypoint() -> Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    // TODO re-enable
    // let mut stdout = io::BufWriter::new(stdout);

    let mut stdin = stdin();

    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    // let mut input = Vec::with_capacity(BUFFER_SIZE);
    let mut input = vec![0; BUFFER_SIZE];

    let mut input_length = stdin.read(&mut input)?;
    let mut section_start = 0;
    let mut token_start = 0;

    let mut had_tokens = false;
    while input_length > 0 {
        let mut section = ByteSection::new(&input[section_start..input_length]);

        let result = (|| -> Result<()> {
            while !section.is_empty() {
                // println!("section = {}", section);
                token_start = section.n;
                let token = compress_next_token(&mut section, is_whitespace)?;
                // println!("token = {:?}", token);
                if token.is_whitespace() {
                    continue;
                }
                last_state = validator.process_token(&token)?;
                had_tokens = true;
                match token {
                    Token::String(ref s) => {
                        let s = std::str::from_utf8(s)?;
                        json::parse(s).map_err(|_| Error::InvalidString)?;
                    }
                    Token::Number(ref s) => {
                        let s = std::str::from_utf8(s)?;
                        // TODO check overflow/underflow/etc.
                        let x: f64 = s.parse()?;
                        // Extra testing for integers.
                        if s.find(|c| c == 'e' || c == 'E' || c == '.').is_none() && x.floor() == x
                        {
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
            Ok(())
        })();

        // println!("result = {:?}", result);

        match result {
            Err(Error::Tokenizer(ref err)) => {
                match err.recovery_point() {
                    // unrecoverable
                    None => {
                        return result;
                    }
                    // recoverable.
                    Some(mut recovery) => {
                        if err.context() == Some(TokenContext::String) {
                            stdout.write_all(&input[token_start..recovery])?;
                            let recovery_length = input_length - recovery;
                            debug!("Recovering: length {}", recovery_length);
                            input.copy_within(recovery.., 0);
                            input_length = stdin.read(&mut input[recovery_length..])?;
                            if input_length == 0 {
                                break;
                            }
                            let (section_start, input_length) = finish_string_token(
                                &mut stdin,
                                &mut stdout,
                                &mut input[..input_length + recovery_length],
                            )?;
                            validator.process_token(&Token::String(vec![].into()))?;
                        }
                    }
                }
            }
            err @ Err(_) => {
                return err;
            }
            Ok(()) => {
                section_start = 0;
                input_length = stdin.read(&mut input)?;
                if input_length == 0 {
                    break;
                }
                // validator.finish()?;
                // while section.check_next_pattern(is_whitespace) {}
                // if section.peek().is_some() {
                //     return Err(Error::ExtraInput);
                // }
                // if !had_tokens {
                //     return Err(Error::EmptyInput);
                // }
            }
        }
    }
    validator.finish()?;
    if !had_tokens {
        return Err(Error::EmptyInput);
    }
    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    // let mut stdin = stdin();
    // let mut buffer: Vec<u8> = Vec::new();
    // stdin.read_to_end(&mut buffer)?;
    // let result = eager_reformat_entrypoint(&buffer);
    // info!("{:?}", result);
    // result

    chunked_entrypoint()

    // tty_entrypoint()
}
