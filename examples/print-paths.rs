// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err, clippy::all)]

use parser::section::Section;
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token};
use parser::validator::{ValidationContext, ValidationError, ValidationState, Validator};
use parser::JsonPathSegment;

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

// trait Lens {}

// struct Traverser {
//     path: Vec<JsonPath>,
// }

// impl Traverser {}

fn eager_reformat_entrypoint(input: &str) -> Result<(), Error> {
    let mut stdout = stdout();
    // let mut stdout = stdout.lock();
    // let mut stdout = io::BufWriter::new(stdout);
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let mut path = Vec::new();

    let mut section = Section::new(input);
    while let Ok(Some(token)) = compress_next_token(&mut section, is_whitespace) {
        if token.is_whitespace() {
            continue;
        }
        writeln!(
            stderr,
            "pre|token={:?}, state={:?}, context={:?}",
            token,
            last_state,
            validator.current_context()
        )?;

        last_state = validator.process_token(&token)?;
        let current_context = validator.current_context();

        let token_is_start = token.is_value_start();
        let token_is_close = token.is_close();

        // let is_start_of_value =
        //     token.is_value_start() && current_context != Some(ValidationContext::ObjectEntryKey);
        let is_start_of_value =
            token_is_start && current_context != Some(ValidationContext::ObjectEntryKey);

        // Results in printing values, arrays, and objects at the start.
        if is_start_of_value {
            writeln!(
                stdout,
                ">\t{:?}\t{}",
                token.value_type(),
                path.iter()
                    .map(|x: &JsonPathSegment| x.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            )?;
        }
        // token.print(&mut stdout)?;
        // stdout.write_all(b"\n")?;

        // Must be popped before doing the post-visit below
        // if token.is_close() {
        if token_is_close {
            path.pop();
        }

        let is_end_of_value = {
            // True if we are in a context where we just finished a value.
            let is_context_in_value = match current_context {
                // None covers the case where our entire value is not an array or object
                None => true,
                // This covers the case where we just finished processing a value.
                Some(ref context) => context.in_value(),
            };

            // These are equivalent
            // (token.is_close() || token.is_value_start()) && is_context_in_value
            // token.value_type().is_some() && is_context_in_value
            (token_is_close || token_is_start) && is_context_in_value
        };

        // Results in printing values, arrays, and objects only at the end.
        if is_end_of_value {
            writeln!(
                stdout,
                "<\t{:?}\t{}",
                token.value_type(),
                path.iter()
                    .map(|x: &JsonPathSegment| x.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            )?;
        }
        writeln!(
            stderr,
            "post|token={:?}, state={:?}, context={:?}",
            token, last_state, current_context
        )?;

        // Path changes should occur:
        // - At the start of an array, push 0
        // - After a comma for an array element, increment 1
        // - After array end, pop
        // - At object start, push null
        // - At an object key, change key
        // - After an object close, pop
        match validator.current_context() {
            Some(ValidationContext::ObjectStart) => path.push(JsonPathSegment::Key("".into())),
            Some(ValidationContext::ObjectEntryKey) => {
                if let Token::String(key) = token {
                    if let Some(JsonPathSegment::Key(ref mut path)) = path.last_mut() {
                        *path = key;
                    }
                }
            }
            // Some(ValidationContext::ObjectEnd) | Some(ValidationContext::ArrayEnd) => { }
            Some(ValidationContext::ArrayStart) => {
                path.push(JsonPathSegment::Index(0));
            }
            Some(ValidationContext::ArrayValue) => {
                if let Some(JsonPathSegment::Index(ref mut n)) = path.last_mut() {
                    *n += 1;
                }
            }
            _ => (),
        }
    }
    validator.finish()?;
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut stdin = stdin();
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;
    eager_reformat_entrypoint(&buffer)
}
