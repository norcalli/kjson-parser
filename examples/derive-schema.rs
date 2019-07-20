// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err, clippy::all)]

use parser::section::Section;
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token};
use parser::validator::{ValidationContext, ValidationError, ValidationState, Validator};
use parser::{JsonPathSegment, JsonType};

use std::collections::{BTreeMap, BTreeSet};
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

#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum JsonSchema {
    Object(BTreeMap<String, JsonSchema>),
    Array(Vec<JsonSchema>),
    Number,
    String,
    Null,
    Bool,
    Either(BTreeSet<JsonSchema>),
    Empty,
}

impl Default for JsonSchema {
    fn default() -> Self {
        JsonSchema::Empty
    }
}

impl JsonSchema {
    pub fn descend(&mut self, path: &JsonPathSegment) -> &mut JsonSchema {
        match self {
            JsonSchema::Object(ref mut obj) => {
                // TODO unwrap()?
                obj.entry(path.as_key().unwrap().to_owned()).or_default()
            }
            JsonSchema::Array(ref mut arr) => {
                // TODO unwrap()?
                let index = path.as_index().unwrap();
                if index >= arr.len() {
                    arr.push(Default::default());
                }
                &mut arr[index]
            }
            _ => panic!("Cannot descend into not an array or object"),
        }
    }

    pub fn is_same_type(&self, other: &JsonSchema) -> bool {
        match (self, other) {
            (JsonSchema::Object(_), JsonSchema::Object(_))
            | (JsonSchema::Array(_), JsonSchema::Array(_))
            | (JsonSchema::Number, JsonSchema::Number)
            | (JsonSchema::String, JsonSchema::String)
            | (JsonSchema::Null, JsonSchema::Null)
            | (JsonSchema::Bool, JsonSchema::Bool) => true,
            _ => false,
        }
    }
}

impl From<JsonType> for JsonSchema {
    fn from(type_: JsonType) -> Self {
        match type_ {
            JsonType::Object => JsonSchema::Object(BTreeMap::new()),
            JsonType::Array => JsonSchema::Array(Vec::new()),
            JsonType::Number => JsonSchema::Number,
            JsonType::String => JsonSchema::String,
            JsonType::Null => JsonSchema::Null,
            JsonType::Bool => JsonSchema::Bool,
        }
    }
}

impl JsonSchema {
    // pub fn either(self, other: Self) -> Self {
    //     let mut set = match self {
    //         JsonSchema::Empty => return other,
    //         JsonSchema::Either(set) => set,
    //         value if self == other => {
    //             return self;
    //         }
    //         value => {
    //             let mut set = BTreeSet::new();
    //             set.insert(value);
    //             set
    //         }
    //     };
    //     set.insert(other);
    //     JsonSchema::Either(set)
    // }

    pub fn either(&mut self, other: Self) -> &mut Self {
        match self {
            JsonSchema::Empty => {
                *self = other;
                self
            }
            JsonSchema::Either(ref mut set) => {
                set.insert(other);
                self
            }
            // TODO need to handle Object({}).either(Object({...}))
            _ if self == &other => self,
            _ => {
                let mut set = BTreeSet::new();
                set.insert(other);
                let old = std::mem::replace(self, JsonSchema::Either(set));
                self.either(old)
            }
        }
    }
}

/// Inspired by this command:
/// ```sh
/// zephyr-ls json \
///   | cargo run --release --example print-paths 2>/dev/null \
///   | grep '^>' \
///   | sed -r 's/\.[0-9]+/[]/g' \
///   | unq \
///   | sort -k3,3
/// ```
/// If you make the array paths generic ([0-9]+ -> []) and find the unique values by the key
/// using their (path, type), then you can find all the variations of the types at that
/// path. Using this idea, you can generate a representation of all the possibilities of
/// the types.
///
/// This function aims to do that explicitly.
fn eager_reformat_entrypoint(input: &str) -> Result<(), Error> {
    let mut stdout = stdout();
    // let mut stdout = stdout.lock();
    // let mut stdout = io::BufWriter::new(stdout);
    let stderr = io::stderr();
    let mut stderr = stderr.lock();
    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let mut schema = JsonSchema::Empty;

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
            let mut schema = &mut schema;
            for part in &path {
                writeln!(stderr, "descending into {:?} at {:?}", schema, part,)?;
                schema = schema.descend(part);
            }
            schema.either(token.value_type().unwrap().into());
            writeln!(stderr, "schema is now {:?}", schema,)?;

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
            Some(ValidationContext::ObjectStart) => path.push(JsonPathSegment::Object("".into())),
            Some(ValidationContext::ObjectEntryKey) => {
                if let Token::String(key) = token {
                    if let Some(JsonPathSegment::Object(ref mut path)) = path.last_mut() {
                        *path = key;
                    }
                }
            }
            // Some(ValidationContext::ObjectEnd) | Some(ValidationContext::ArrayEnd) => { }
            Some(ValidationContext::ArrayStart) => {
                path.push(JsonPathSegment::Array(0));
            }
            Some(ValidationContext::ArrayValue) => {
                if let Some(JsonPathSegment::Array(ref mut n)) = path.last_mut() {
                    *n += 1;
                }
            }
            _ => (),
        }
    }
    validator.finish()?;
    writeln!(stdout, "Schema: {:#?}", schema)?;
    Ok(())
}

fn main() -> Result<(), Error> {
    let mut stdin = stdin();
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;
    eager_reformat_entrypoint(&buffer)
}
