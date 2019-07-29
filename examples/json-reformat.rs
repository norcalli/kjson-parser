#![warn(clippy::all)]
#![warn(const_err)]

use parser::section::{ByteSection, PeekSeek};
use parser::tokenizer::{
    compress_next_token, utils, utils::is_whitespace, Token, TokenContext, TokenizeError,
};
use parser::validator::{ValidationError, ValidationState, Validator};

use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;
use koption_macros::*;

const DEBUG: bool = false;

macro_rules! dprintln {
    ($($e:expr),*) => (
        if DEBUG {
            eprintln!($($e),*);
        }
    )
}

#[derive(Debug, From)]
pub enum Error {
    Io(std::io::Error),
    Validation(ValidationError),
    Tokenizer(TokenizeError),
    ExtraInput,
    InvalidStringUtf8(std::str::Utf8Error),
    InvalidString,
    EmptyInput,
    UnexpectedEndOfInput,
    InvalidNumber(std::num::ParseFloatError),
    InvalidInt(std::num::ParseIntError),
}

pub type Result<T> = std::result::Result<T, Error>;

// const BUFFER_SIZE: usize = 4 * 1024 * 1024;
const BUFFER_SIZE: usize = 1024;
// const BUFFER_SIZE: usize = 1024 * 1024;
// const BUFFER_SIZE: usize = 1000;

pub struct Buffer {
    pub buffer: Vec<u8>,
    pub section_size: usize,
}

impl Buffer {
    pub fn new(size: usize) -> Self {
        Buffer {
            buffer: vec![0; size],
            section_size: 0,
        }
    }

    pub fn section(&self) -> ByteSection<'_> {
        ByteSection::new(&self.buffer[..self.section_size])
    }

    #[inline]
    pub fn init<R: Read>(&mut self, r: &mut R) -> io::Result<usize> {
        self.section_size = r.read(&mut self.buffer)?;
        // self.section.reset(&self.buffer[..count]);
        // std::mem::replace(&mut self.section.src, &self.buffer[..count]);
        Ok(self.section_size)
    }

    #[inline]
    pub fn renew<R: Read>(&mut self, r: &mut R, recovery_point: usize) -> io::Result<usize> {
        let n = self.section_size;
        // debug_assert!(recovery_point < n);
        debug_assert!(recovery_point <= n);
        // Nothing to recover if we are at the end of the buffer.
        if recovery_point == n {
            self.section_size = r.read(&mut self.buffer)?;
            Ok(self.section_size)
        } else {
            let recovery_length = n - recovery_point;
            self.buffer.copy_within(recovery_point..n, 0);
            let read_count = r.read(&mut self.buffer[recovery_length..])?;
            self.section_size = recovery_length + read_count;
            Ok(read_count)
        }
    }
}

/// Returns the point at which to continue parsing.
fn finish_string_token<R: Read, W: Write>(
    buffer: &mut Buffer,
    stdin: &mut R,
    stdout: &mut W,
) -> Result<usize> {
    loop {
        let mut section = buffer.section();
        match utils::section_inside_string(&mut section) {
            Ok(()) => {
                stdout.write_all(&section.src[..section.n])?;
                return Ok(section.n);
            }
            Err(err) => {
                // Recoverable
                if let Some((token_start, recovery_point)) =
                    and!(err.token_start() => err.recovery_point())
                {
                    stdout.write_all(&section.src[token_start..recovery_point])?;
                    if buffer.renew(stdin, recovery_point)? == 0 {
                        return Err(Error::UnexpectedEndOfInput);
                    }
                } else {
                    return Err(err.into());
                }
            }
        }
    }
}

struct Configuration {}

#[derive(Debug)]
enum CompletionState {
    Complete,
    Incomplete,
    PotentialFalsePositive(Token<'static>),
}

// TODO make checking integers/extra input/empty input configurable errors.
// You can make a validator more permissive or less permissive depending on your
// preference for edge cases.
/// Theory of operation:
/// - Fill the buffer
/// - Start tokenizing for the length of the buffer (0 to buffer.len())
/// - Possible outcomes:
///     - Finish processing the input and complete validation.
///     - We hit an unrecoverable error
///     - We hit a recoverable error (eof)
/// - If we hit a recoverable error, then we should try to read more
///     - Copy the recoverable bit to the beginning of the buffer
///     - Read data in from the end of the recoverable section to the end of the buffer
///     - We need to retry the recoverable section, but if we hit another "recoverable" error for
///     this same section, then we should hard fail.
/// - Check for any tokens after the end of the input.
fn chunked_entrypoint() -> Result<()> {
    let stdout = stdout();
    let mut stdout = stdout.lock();
    // TODO re-enable
    // let mut stdout = io::BufWriter::new(stdout);

    let mut stdin = stdin();
    let stdin = &mut stdin;

    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let mut buffer = Buffer::new(BUFFER_SIZE);

    buffer.init(stdin)?;

    let mut continuation_point = 0;
    let mut had_tokens = false;

    loop {
        let mut section = buffer.section();
        dprintln!(
            "beginning. section.n = {}, section.head = {:?}",
            section.n,
            if section.n < section.src.len() {
                std::str::from_utf8(&section.src[section.n..])
            } else {
                Ok("")
            }
        );
        section.skip(continuation_point);
        let result = (|| -> Result<CompletionState> {
            dprintln!(
                "continuation = {}, section.start = {:?}",
                continuation_point,
                // std::str::from_utf8(&section.src[section.n..section.src.len().min(section.n + 10)])
                std::str::from_utf8(&section.src[section.n..])
            );
            while !section.is_empty() {
                let token = compress_next_token(&mut section, is_whitespace)?;
                if token.is_whitespace() {
                    continue;
                }
                if section.is_empty() && token.potential_false_positive() {
                    continuation_point = section.n;
                    return Ok(CompletionState::PotentialFalsePositive(token.into_owned()));
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
                        if s.find(|c| c == 'e' || c == 'E' || c == '.').is_none()
                            && x.floor() - x <= std::f64::EPSILON
                        {
                            let _: i64 = s.parse()?;
                        }
                    }
                    _ => (),
                }
                token.print(&mut stdout)?;
                if ValidationState::Complete == last_state {
                    stdout.write_all(b"\n")?;
                    continuation_point = section.n;
                    return Ok(CompletionState::Complete);
                }
            }
            dprintln!(
                "end. section.n = {}, section.head = {:?}",
                section.n,
                std::str::from_utf8(&section.src[section.n..])
            );
            continuation_point = section.n;
            Ok(CompletionState::Incomplete)
        })();
        continuation_point = section.n;
        // drop(section);

        dprintln!(
            "result. section.n = {}, section.head = {:?}",
            section.n,
            if section.n < section.src.len() {
                std::str::from_utf8(&section.src[section.n..])
            } else {
                Ok("")
            }
        );
        dprintln!("result = {:?}", result);

        match result {
            Ok(CompletionState::Complete) => {
                validator.finish()?;
            }

            // We finished tokenizing and landed exactly on the edge of the buffer, but
            // validation isn't complete yet.
            Ok(CompletionState::Incomplete) => {
                dprintln!("ok(false), renewing buffer");
                if buffer.renew(stdin, buffer.section_size)? == 0 {
                    dprintln!("no more bytes");
                    break;
                }
                continuation_point = 0;
            }

            // Validation is skipped, so we need to do that, but only if we hit the same conditions
            // as the ones that brought us here, since then we would be in an loop.
            // Those conditions are: section.is_empty() && token.potential_false_positive()
            // This could only be triggered twice in a row if we hit the end of the input as well,
            // which implies buffer.renew(..) = 0.
            Ok(CompletionState::PotentialFalsePositive(token)) => match token {
                Token::Number(ref s) => {
                    let token_start = continuation_point - s.len();
                    let recovery_point = token_start;
                    continuation_point = 0;
                    if buffer.renew(stdin, recovery_point)? == 0 {
                        // Process validation.
                        validator.process_token(&token)?;
                        had_tokens = true;
                        break;
                    }
                    dprintln!(
                        "buffer recovery: {:?}",
                        std::str::from_utf8(&buffer.buffer[..buffer.buffer.len() - recovery_point])
                    );
                }
                _ => unreachable!("Only Token::Number is potential_false_positive"),
            },

            // See if we have recoverable errors.
            Err(Error::Tokenizer(err)) => {
                match and!(err.token_start() => err.recovery_point()) {
                    // unrecoverable
                    None => {
                        dprintln!("unrecoverable tokenizer error: {:?}", err);
                        return Err(Error::Tokenizer(err));
                    }

                    // recoverable.
                    Some((token_start, recovery_point)) => {
                        dprintln!("recoverable: {} -> {}", token_start, recovery_point);
                        stdout.write_all(&buffer.buffer[token_start..recovery_point])?;
                        if buffer.renew(stdin, recovery_point)? == 0 {
                            dprintln!("finishing string: no more bytes");
                            return Err(Error::UnexpectedEndOfInput);
                        }
                        dprintln!(
                            "buffer recovery: {:?}",
                            std::str::from_utf8(
                                &buffer.buffer[..buffer.buffer.len() - recovery_point]
                            )
                        );
                        if err.context() == Some(TokenContext::String) {
                            continuation_point =
                                finish_string_token(&mut buffer, stdin, &mut stdout)?;
                            // TODO combine this into process_token so that we don't accidentally
                            // miss setting this.
                            had_tokens = true;
                            if validator.process_token(&Token::String(vec![].into()))?
                                == ValidationState::Complete
                            {
                                stdout.write_all(b"\n")?;
                                // TODO what to do here?
                                // break;
                            }
                        } else {
                            continuation_point = 0;
                        }
                    }
                }
            }

            err @ Err(_) => {
                return err.map(|_| ());
            }
        }
    }
    validator.finish()?;
    let mut section = buffer.section();
    section.skip(continuation_point);
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
    chunked_entrypoint()
}
