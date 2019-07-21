#![warn(const_err, clippy::all)]

use parser::section::Section;
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token};
use parser::validator::{ValidationContext, ValidationError, ValidationState, Validator};
use parser::{JsonPathSegment, JsonType};

use log::*;
use std::io::{self, stdin, Read};

use derive_more::From;

#[derive(Debug, From)]
enum Error {
    Io(io::Error),
    Validation(ValidationError),
}

fn entrypoint(input: &str) -> Result<(), Error> {
    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let mut path: Vec<JsonPathSegment> = Vec::new();

    let mut section = Section::new(input);
    while let Ok(Some(token)) = compress_next_token(&mut section, is_whitespace) {
        if token.is_whitespace() {
            continue;
        }

        // Any calculations should be done past this point, since validation is done above.
        last_state = validator.process_token(&token)?;

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

        let current_context = validator.current_context();

        let token_is_start = token.is_value_start();
        let token_is_close = token.is_close();

        let is_start_of_value =
            token_is_start && current_context != Some(ValidationContext::ObjectEntryKey);

        // Results in printing values, arrays, and objects at the start.
        if is_start_of_value {
            /*
             * SKELETON: You can put code here for preorder processing of arrays/objects/values
             * This is mostly useful for arrays/objects
             */
        }

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

        /* Example to calculate array_length.
         *
         * let array_length = container_last_segment
         *     .and_then(|p| p.as_index())
         *     .expect("expected index at path segment");
         */

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

        // This can only be the start & end if it is not a container, but a plain value.
        if is_start_of_value && is_end_of_value {
            /*
             * SKELETON: you can access plain values here.
             */

            // At this point, you have access to the path and the value
            info!(
                "{} = {:?}",
                path.iter()
                    .map(|x: &JsonPathSegment| x.to_string())
                    .collect::<Vec<_>>()
                    .join("."),
                token
            );
        }

        // Results in printing values, arrays, and objects only at the end.
        if is_end_of_value {
            // SKELETON: You can put code here for postorder processing of arrays/objects/values
            // This is mostly useful for arrays/objects
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
                // Push path for modification
                path.push(JsonPathSegment::Key("".into()));

                /*
                 * SKELETON: Entrypoint to start of new object.
                 */
            }
            Some(ValidationContext::ObjectEntryKey) => {
                if let Token::String(new_key) = token {
                    if let Some(ref mut segment) = path.last_mut() {
                        if let Some(key) = segment.as_key_mut() {
                            /*
                             * SKELETON: Transition from old key to new key. Path is still pointing to the old
                             */
                            *key = new_key;
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
                path.push(JsonPathSegment::Index(0));
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
    Ok(())
}

fn main() -> Result<(), Error> {
    env_logger::init();
    let mut stdin = stdin();
    let mut buffer = String::new();
    stdin.read_to_string(&mut buffer)?;
    entrypoint(&buffer)?;
    Ok(())
}

