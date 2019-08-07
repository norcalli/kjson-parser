#![warn(const_err, clippy::all)]

use parser::section::ByteSection;
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token};
use parser::validator::{ValidationContext, ValidationError, ValidationState, Validator};
use parser::{JsonPath, JsonPathSegment, JsonType};

use log::*;
use std::borrow::Cow;
use std::collections::HashMap;
use std::io::{self, stdin, Read, Write};

use derive_more::From;

#[derive(Debug, From)]
enum Error {
    Io(io::Error),
    Validation(ValidationError),
}

#[derive(Debug, derive_more::Display)]
#[display(fmt = r#"{{"url":{},"title":{}}}"#, url, title)]
struct Output {
    url: String,
    title: String,
}

impl Default for Output {
    fn default() -> Self {
        Self {
            url: r#""""#.into(),
            title: r#""""#.into(),
        }
    }
}

fn entrypoint(input: &[u8]) -> Result<(), Error> {
    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    let mut path: Vec<JsonPathSegment> = Vec::new();

    let mut outputs: HashMap<JsonPath, Output> = Default::default();

    let mut section = ByteSection::new(input);
    while let Ok(token) = compress_next_token(&mut section, is_whitespace) {
        if token.is_whitespace() {
            continue;
        }

        // Any calculations should be done past this point, since validation is done above.
        last_state = validator.process_token(&token)?;

        if last_state == ValidationState::Complete {
            for output in outputs.values() {
                if !output.url.is_empty() {
                    writeln!(stdout, "{}", output)?;
                }
            }
            outputs.clear();
        }

        /*
         * SKELETON: All tokens are processed at this point, including punctuation, which means
         * that you could process the token in this loop as well. Dealing with non-punctuation
         * should be done in the:
         * - is_start_of_value hook: for the beginning of values
         * - is_start_of_value && is_end_of_value hook for values
         * - is_start_of_value hook: for the end of values
         *
         * Example to print tokens eagerly.
         *
         * token.print(&mut stdout)?;
         */
        // token.print(&mut stdout)?;

        let token_value_type: Option<JsonType> = token.value_type();

        let current_context = validator.current_context();

        let token_is_start = token.is_value_start();
        let token_is_close = token.is_close();

        let is_object_key = current_context == Some(ValidationContext::ObjectEntryKey);

        // let is_start_of_value =
        //     token_is_start && current_context != Some(ValidationContext::ObjectEntryKey);
        let is_start_of_value = token_is_start && !is_object_key;

        let is_end_of_value = {
            // True if we are in a context where we just finished a value.
            let is_context_in_value = match current_context {
                // None covers the case where our entire value is not an array or object
                None => true,
                // This covers the case where we just finished processing a value.
                Some(ref context) => context.in_value(),
            };

            (token_is_close || token_is_start) && is_context_in_value
        };

        /*
         * DO calculations after this point
         */

        /* Ordering is important here: Change the path before the post-order visit entrypoint so
         * that it is correct for the consideration of the parent container.
         * Return the last_segment so that you can access the last array index (to get the array
         * length) or the last object key (for whatever you might want that for).
         */
        let container_last_segment = if let Token::ArrayClose = token {
            // Must be popped before doing the post-visit below
            path.pop()
        } else if token == Token::ObjectClose {
            // Must be popped before doing the post-visit below
            path.pop()
        } else {
            None
        };

        let display_path = JsonPath::new(path.clone().into());

        // PREORDER: Results in printing values, arrays, and objects at the start.
        if is_start_of_value {
            /*
             * SKELETON: You can put code here for preorder processing of arrays/objects/values
             * This is mostly useful for arrays/objects
             */

            match token_value_type {
                Some(JsonType::Object) => {
                    info!("{} = object.start", display_path);
                }
                Some(JsonType::Array) => {
                    info!("{} = array.start", display_path);
                }
                _ => (),
            }
        };

        // INORDER: This can only be the start & end if it is not a container, but a plain value.
        if is_start_of_value && is_end_of_value {
            /*
             * SKELETON: you can access plain values here.
             */

            // At this point, you have access to the path and the value
            info!("{} = {:?}", display_path, token);

            if let Some(key) = display_path.last().and_then(|p| p.as_key()) {
                if key == r#""url""# {
                    if let Some(url) = token.as_string() {
                        let output = outputs.entry(display_path.parent()).or_default();
                        output.url = url.to_owned();
                    }
                } else if key == r#""title""# {
                    if let Some(title) = token.as_string() {
                        let output = outputs.entry(display_path.parent()).or_default();
                        output.title = title.to_owned();
                    }
                }
            }
        }

        // POSTORDER: Results in printing values, arrays, and objects only at the end.
        if is_end_of_value {
            // SKELETON: You can put code here for postorder processing of arrays/objects/values
            // This is mostly useful for arrays/objects

            match token_value_type {
                Some(JsonType::Object) => {
                    let last_key = container_last_segment.and_then(|p| p.as_key());
                    info!("{} = object.end, lastkey={:?}", display_path, last_key,);
                    // TODO figure out how to make this work.
                    // if let Some(output) = outputs.get(&display_path) {
                    //     println!("{:?}", output);
                    // }
                }
                Some(JsonType::Array) => {
                    let array_length = container_last_segment
                        .and_then(|p| p.as_index())
                        .expect("expected index at path segment");
                    info!("{} = array.end, length={}", display_path, array_length,);
                }
                _ => (),
            }
        }

        /* Update the path.
         * Path changes should occur:
         * - At the start of an array, push 0
         * - After a comma for an array element, increment 1
         * - After array end, pop
         * - At object start, push null
         * - At an object key, change key
         * - After an object close, pop
         */
        match validator.current_context() {
            Some(ValidationContext::ObjectStart) => {
                /*
                 * SKELETON: Entrypoint to start of new object.
                 */

                // Push path for modification
                path.push(parser::EMPTY_KEY);
            }
            Some(ValidationContext::ObjectEntryKey) => {
                if let Token::String(new_key) = token {
                    if let Some(ref mut segment) = path.last_mut() {
                        if let Some(key) = segment.as_key_mut() {
                            /*
                             * SKELETON: Transition from old key to new key. Path is still pointing to the old
                             */
                            // *key = new_key;
                            *key = match new_key {
                                Cow::Borrowed(bytes) => {
                                    Cow::Borrowed(unsafe { std::str::from_utf8_unchecked(bytes) })
                                }
                                Cow::Owned(bytes) => {
                                    Cow::Owned(unsafe { String::from_utf8_unchecked(bytes) })
                                }
                            }
                        }
                        /*
                         * SKELETON: Entrypoint to new key. Path is updated
                         */
                    }
                }
            }
            // Some(ValidationContext::ObjectEnd) | Some(ValidationContext::ArrayEnd) => { }
            Some(ValidationContext::ArrayStart) => {
                // Set up index path for new values.
                // If the array is empty, then this shouldn't be used.
                path.push(0.into());
            }
            Some(ValidationContext::ArrayValue) => {
                // I wish I could use this instead.
                // try {
                //     *path.last_mut()?.as_index()? += 1;
                // }
                // if let Some(n) = path.last_mut().and_then(|p| p.as_index_mut()) {
                if let Some(JsonPathSegment::Index(ref mut n)) = path.last_mut() {
                    *n += 1;
                }
            }
            _ => (),
        }
    }
    validator.finish()?;

    for output in outputs.values() {
        if !output.url.is_empty() {
            writeln!(stdout, "{}", output)?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut stdin = stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer)?;
    entrypoint(&buffer)?;
    Ok(())
}
