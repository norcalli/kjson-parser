// use termion::input::TermRead;
// use termion::event::{Key, Event};
// use termion::raw::IntoRawMode;
// use termion::{clear, cursor, color, style};

#![warn(const_err, clippy::all)]

use parser::section::ByteSection;
use parser::tokenizer::{compress_next_token, utils::is_whitespace, Token};
use parser::validator::{ValidationContext, ValidationError, ValidationState, Validator};
use parser::{JsonPathSegment, JsonType};

use log::*;
use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::io::{self, stdin, stdout, Read, Write};

use derive_more::From;

#[derive(Debug, From)]
enum Error {
    Io(std::io::Error),
    Validation(ValidationError),
    Options(kargs::Error),
}

// Look at other combinators for inspiration on how to do
// stronger typing

/// Two possible designs:
/// - Building subtraversal into lenses
/// - Or using lists to represent the recursion

trait Lens {}

// TODO why did I add Ord?
// #[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
#[derive(Debug, Eq, PartialEq)]
pub enum JsonSchema {
    Empty,
    Null,
    Bool,
    Number,
    String,
    // Enum depends on maximum cardinality. You could assume it's an enum depending on a user
    // inputted maximum number which determines if it qualifies to be an enum. However, if your
    // data samples are too few, then it will be too restrictive. Therefore, such a number must be
    // chosen in consideration of the total number of input samples.
    //
    // Enum {
    //     inner: BTreeSet<String>,
    // },
    Object {
        inner: BTreeMap<String, JsonSchema>,
        // required: BTreeSet<String>,
        key_count: HashMap<String, usize>,
        total_count: usize,
    },
    Array {
        inner: Vec<JsonSchema>,
        // min_length: usize,
        lengths: BTreeSet<usize>,
    },
    Either(HashMap<JsonType, JsonSchema>),
}

impl Default for JsonSchema {
    fn default() -> Self {
        JsonSchema::Empty
    }
}

impl JsonSchema {
    pub fn as_object_mut(&mut self) -> Option<&mut JsonSchema> {
        match self {
            JsonSchema::Object { .. } => Some(self),
            JsonSchema::Either(ref mut inner) => inner.get_mut(&JsonType::Object),
            _ => None,
        }
    }

    pub fn descend(&mut self, path: &JsonPathSegment) -> &mut JsonSchema {
        match self {
            JsonSchema::Object {
                ref mut inner,
                ref mut key_count,
                ref mut total_count,
            } => {
                let key = path
                    .as_key()
                    .expect("path segment must be an object type")
                    .to_owned();
                // *key_count.entry(key.clone()).or_default() += 1;
                // TODO keep unwrap()?
                inner.entry(key.into_owned()).or_default()
            }
            JsonSchema::Array { ref mut inner, .. } => {
                // TODO keep unwrap()?
                // If you set index = 0, then you are using the mode where you can treat an array
                // as a homogeneous array.
                // let index = 0;
                let index = path.as_index().expect("path segment must be array type");
                if index >= inner.len() {
                    inner.push(Default::default());
                }
                &mut inner[index]
            }
            JsonSchema::Either(ref mut inner) => {
                let path_type = if path.is_index() {
                    JsonType::Array
                } else {
                    JsonType::Object
                };
                inner
                    .entry(path_type)
                    .or_insert_with(|| path_type.into())
                    .descend(path)
            }
            _ => panic!("Cannot descend into not an array or object"),
        }
    }

    pub fn descend_path<'a, I: Iterator<Item = &'a JsonPathSegment<'a>>>(
        &mut self,
        path: I,
    ) -> &mut JsonSchema {
        let mut schema = self;
        for part in path {
            debug!("descending into {:?} at {:?}", schema, part,);
            schema = schema.descend(part);
        }
        schema
    }

    pub fn descend_homogeneous(&mut self, path: &JsonPathSegment) -> &mut JsonSchema {
        match self {
            JsonSchema::Object {
                ref mut inner,
                ref mut key_count,
                ref mut total_count,
            } => {
                let key = path
                    .as_key()
                    .expect("path segment must be an object type")
                    .to_owned();
                // *key_count.entry(key.clone()).or_default() += 1;
                // TODO keep unwrap()?
                inner.entry(key.into_owned()).or_default()
            }
            JsonSchema::Array { ref mut inner, .. } => {
                // TODO keep unwrap()?
                // If you set index = 0, then you are using the mode where you can treat an array
                // as a homogeneous array.
                let index = 0;
                if index >= inner.len() {
                    inner.push(Default::default());
                }
                &mut inner[index]
            }
            JsonSchema::Either(ref mut inner) => {
                let path_type = if path.is_index() {
                    JsonType::Array
                } else {
                    JsonType::Object
                };
                inner
                    .entry(path_type)
                    .or_insert_with(|| path_type.into())
                    .descend_homogeneous(path)
            }
            _ => panic!("Cannot descend into not an array or object"),
        }
    }

    pub fn descend_path_homogeneous<'a, I: Iterator<Item = &'a JsonPathSegment<'a>>>(
        &mut self,
        path: I,
    ) -> &mut JsonSchema {
        let mut schema = self;
        for part in path {
            debug!("descending into {:?} at {:?}", schema, part,);
            schema = schema.descend_homogeneous(part);
        }
        schema
    }

    pub fn is_same_type(&self, other: &JsonSchema) -> bool {
        use std::mem::discriminant;
        discriminant(self) == discriminant(other)
        // match (self, other) {
        //     (JsonSchema::Object { .. }, JsonSchema::Object { .. })
        //     | (JsonSchema::Array { .. }, JsonSchema::Array { .. })
        //     | (JsonSchema::Number, JsonSchema::Number)
        //     | (JsonSchema::String, JsonSchema::String)
        //     | (JsonSchema::Null, JsonSchema::Null)
        //     | (JsonSchema::Bool, JsonSchema::Bool) => true,
        //     _ => false,
        // }
    }

    pub fn is_type(&self, other: JsonType) -> bool {
        match (self, other) {
            (JsonSchema::Object { .. }, JsonType::Object)
            | (JsonSchema::Array { .. }, JsonType::Array)
            | (JsonSchema::Number, JsonType::Number)
            | (JsonSchema::String, JsonType::String)
            | (JsonSchema::Null, JsonType::Null)
            | (JsonSchema::Bool, JsonType::Bool) => true,
            _ => false,
        }
    }

    pub fn get_type(&self) -> Option<JsonType> {
        Some(match self {
            JsonSchema::Object { .. } => JsonType::Object,
            JsonSchema::Array { .. } => JsonType::Array,
            JsonSchema::Number => JsonType::Number,
            JsonSchema::String => JsonType::String,
            JsonSchema::Null => JsonType::Null,
            JsonSchema::Bool => JsonType::Bool,
            _ => return None,
        })
    }
}

impl From<JsonType> for JsonSchema {
    fn from(type_: JsonType) -> Self {
        match type_ {
            JsonType::Object => JsonSchema::Object {
                inner: Default::default(),
                key_count: Default::default(),
                total_count: Default::default(),
            },
            JsonType::Array => JsonSchema::Array {
                inner: Default::default(),
                lengths: Default::default(),
            },
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

    pub fn either(&mut self, other: JsonType) -> &mut Self {
        match self {
            JsonSchema::Empty => {
                *self = other.into();
            }
            JsonSchema::Either(ref mut inner) => {
                inner.entry(other).or_insert_with(|| other.into());
            }
            // TODO need to handle Object({}).either(Object({...}))
            // _ if self == &other => self,
            // _ if self.is_same_type(&other) => self,
            _ if self.is_type(other) => (),
            _ => {
                let old = std::mem::replace(self, JsonSchema::Empty);
                *self = JsonSchema::Either({
                    let mut inner = HashMap::new();
                    inner.insert(
                        old.get_type()
                            .expect("Either: old doesn't have a json type"),
                        old,
                    );
                    inner.insert(other, other.into());
                    inner
                });
                // let old = std::mem::replace(self, JsonSchema::Either(HashMap::new()));
                // if let JsonSchema::Either(ref mut inner) = self {
                //     inner.insert(
                //         old.get_type()
                //             .expect("Either: old doesn't have a json type"),
                //         old,
                //     );
                //     inner.insert(other, other.into());
                // }
            }
        }
        self
    }
}

/// # Overview
///
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
///
/// # Strategies:
///
/// - Do what the command does and store the types for each path and then try to merge it
/// at the end by traversing the topologically sorted paths in linearized order.
/// - Try to build up the merge tree during the process.
///
/// # It should probably accept some options for questions without a single answer such as:
///
/// - How to deal with array elements: try to treat them as a single object or each index
/// as individual?
///     - Is there an automatic strategy/heuristic to decide which one is the better strategy
///     like which one leads to a more specific schema. (specificity is a heuristic that I
///     think I could make)
///
fn derive_schema(input: &[u8], opt: Opt) -> Result<JsonSchema, Error> {
    let mut validator = Validator::new();
    let mut last_state = ValidationState::Incomplete;

    let mut schema = JsonSchema::Empty;

    let mut path = Vec::new();

    // TODO use these instead of descending the full path each time.
    // let mut schema_ref = &mut schema;
    // let mut schema_ref_stack: Vec<&mut JsonSchema> = Vec::new();

    let mut section = ByteSection::new(input);
    while let Ok(token) = compress_next_token(&mut section, is_whitespace) {
        if token.is_whitespace() {
            continue;
        }
        debug!(
            "pre|token={:?}, state={:?}, context={:?}",
            token,
            last_state,
            validator.current_context()
        );

        last_state = validator.process_token(&token)?;
        let current_context = validator.current_context();

        let token_is_start = token.is_value_start();
        let token_is_close = token.is_close();

        // let schema_ref = once_cell::unsync::Lazy::new(|| schema.descend_path(path.iter()));

        // let is_start_of_value =
        //     token.is_value_start() && current_context != Some(ValidationContext::ObjectEntryKey);
        let is_start_of_value =
            token_is_start && current_context != Some(ValidationContext::ObjectEntryKey);

        // Results in printing values, arrays, and objects at the start.
        if is_start_of_value {
            // Edge case:
            // echo '[{}, 1][1,2][{"a":1}]'  | cargo run --release --example derive-schema
            // Schema: Array(
            //     [
            //         Either(
            //             {
            //                 Number: Number,
            //                 Object: Object(
            //                     {
            //                         "\"a\"": Number,
            //                     },
            //                 ),
            //             },
            //         ),
            //         Number,
            //     ],
            // )
            //
            // Empty objects requires all descendents be Either(Null, self)

            // let schema = schema.descend_path(path.iter());
            let schema_ref = if opt.homogeneous_arrays {
                schema.descend_path_homogeneous(path.iter())
            } else {
                schema.descend_path(path.iter())
            };

            // if let Some(JsonType::Object) | Some(JsonType::Array) = token.value_type() {
            //     match schema_ref {
            //         JsonSchema::Object {
            //             ref mut inner,
            //             ref mut key_count,
            //         } => {
            //             // key_count;
            //         }
            //         JsonSchema::Array {
            //             ref mut inner,
            //             ref mut lengths,
            //         } => {
            //             // *schema = JsonSchema::EmptyArray;
            //         }
            //         _ => (),
            //     }
            // }

            debug!("schema is {:?}", schema_ref,);
            debug!("splitting with {:?}", token.value_type(),);
            schema_ref.either(token.value_type().unwrap());
            debug!("schema is now {:?}", schema_ref,);

            debug!(
                ">\t{:?}\t{}",
                token.value_type(),
                path.iter()
                    .map(|x: &JsonPathSegment| x.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            );
            debug!("pre|path={:?},schema={:?}", path, schema_ref);
        }
        // token.print(&mut stdout)?;
        // stdout.write_all(b"\n")?;

        // MUST run before path.pop to have access to index
        if let Token::ArrayClose = token {
            let array_length = path
                .last_mut()
                .and_then(|x| x.as_index())
                .expect("expected index at path segment");
            // Must be popped before doing the post-visit below
            path.pop();
            // schema_ref = schema.descend_path(path.iter());
            // schema_ref = schema_ref_stack.pop().unwrap();
            let schema_ref = if opt.homogeneous_arrays {
                schema.descend_path_homogeneous(path.iter())
            } else {
                schema.descend_path(path.iter())
            };
            match schema_ref {
                JsonSchema::Array {
                    ref mut inner,
                    ref mut lengths,
                } => {
                    lengths.insert(array_length);
                }
                // Further edge case:
                // - If a new array has length smaller than the existing array, then elements after
                // the end must be either null.
                // - If a new Object has keys which are not shared, then the disjoint values must
                // be either null.
                // TODO add required fields to objects
                // TODO add min-length to array
                // TODO add a post-processing step to allow converting between
                // required/min-length to either(null, *)
                _ => (),
            }
        } else if token == Token::ObjectClose {
            // Must be popped before doing the post-visit below
            path.pop();
            // schema_ref = schema.descend_path(path.iter());
            // schema_ref = schema_ref_stack.pop().unwrap();
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

        // It's a value.
        if is_start_of_value && is_end_of_value {
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
            // Use the end of the value to do some convex region tightening for objects
            // and arrays on their required keys and length, respectively.

            debug!(
                "<\t{:?}\t{}",
                token.value_type(),
                path.iter()
                    .map(|x: &JsonPathSegment| x.to_string())
                    .collect::<Vec<_>>()
                    .join(".")
            );
            // debug!("post|path={:?},schema={:?}", path, schema_ref);
        }
        debug!(
            "post|token={:?}, state={:?}, context={:?}",
            token, last_state, current_context
        );

        // Path changes should occur:
        // - At the start of an array, push 0
        // - After a comma for an array element, increment 1
        // - After array end, pop
        // - At object start, push null
        // - At an object key, change key
        // - After an object close, pop
        match validator.current_context() {
            Some(ValidationContext::ObjectStart) => {
                path.push(JsonPathSegment::Key("".into()));
                let path_iter = path[..path.len() - 1].iter();
                let schema_ref = if opt.homogeneous_arrays {
                    schema.descend_path_homogeneous(path_iter)
                } else {
                    schema.descend_path(path_iter)
                };
                if let Some(JsonSchema::Object {
                    inner,
                    ref mut key_count,
                    ref mut total_count,
                }) = schema_ref.as_object_mut()
                {
                    *total_count += 1;
                } else {
                    panic!(
                        "Expected object for key_count:\n\tpath={:?}\n\tsubpath={:?}\n\tsub_schema={:?}",
                        path,
                        &path[..path.len() - 1],
                        schema_ref
                    );
                }
            }
            Some(ValidationContext::ObjectEntryKey) => {
                if let Token::String(new_key) = token {
                    let new_key = match new_key {
                        Cow::Borrowed(bytes) => {
                            Cow::Borrowed(unsafe { std::str::from_utf8_unchecked(bytes) })
                        }
                        Cow::Owned(bytes) => {
                            Cow::Owned(unsafe { String::from_utf8_unchecked(bytes) })
                        }
                    };
                    // TODO this is doodoo
                    let path_iter = path[..path.len() - 1].iter();
                    let schema_ref = if opt.homogeneous_arrays {
                        schema.descend_path_homogeneous(path_iter)
                    } else {
                        schema.descend_path(path_iter)
                    };
                    if let Some(JsonSchema::Object {
                        inner,
                        ref mut key_count,
                        ref mut total_count,
                    }) = schema_ref.as_object_mut()
                    {
                        *key_count.entry(new_key.to_owned().into()).or_default() += 1;
                    } else {
                        panic!(
                            "Expected object for key_count:\n\tpath={:?}\n\tsubpath={:?}\n\tsub_schema={:?}",
                            path,
                            &path[..path.len() - 1],
                            schema_ref
                        );
                    }
                    if let Some(ref mut segment) = path.last_mut() {
                        // if let Some(key) = segment.as_key_mut() {
                        if let JsonPathSegment::Key(ref mut key) = segment {
                            *key = new_key;
                        }
                        // *key_count.entry(key.clone()).or_default() += 1;
                        // schema_ref = schema_ref.descend(segment);
                    }
                }
            }
            // Some(ValidationContext::ObjectEnd) | Some(ValidationContext::ArrayEnd) => { }
            Some(ValidationContext::ArrayStart) => {
                path.push(JsonPathSegment::Index(0));

                // If I descend here, it will create an empty node at 0 which
                // will require cleanup later.
                // TODO is there a better place to do this? Only allocated when it
                // is used via path?
                // schema_ref.descend(&JsonPathSegment::Index(0));
            }
            Some(ValidationContext::ArrayValue) => {
                // if !opt.homogeneous_arrays {
                if let Some(JsonPathSegment::Index(ref mut n)) = path.last_mut() {
                    *n += 1;
                }
                // }
            }
            _ => (),
        }
    }
    validator.finish()?;
    Ok(schema)
}

#[derive(Debug, Default)]
struct Opt {
    homogeneous_arrays: bool,
}

fn parse_options() -> kargs::Result<Opt> {
    use kargs::*;
    let mut opt = Opt::default();
    for arg in ParserOptions::default()
        .need("a", Type::Bool)
        .need("homogeneous-arrays", Type::Bool)
        .build_from_args()
    {
        let arg = arg?;
        use EmittedRef::*;
        match arg.as_ref() {
            Named("a", Value::Bool(v)) | Named("homogeneous-arrays", Value::Bool(v)) => {
                if opt.homogeneous_arrays {
                    return Err(arg.was_duplicate());
                }
                opt.homogeneous_arrays = *v;
            }
            _ => return Err(arg.was_extra()),
        }
    }
    Ok(opt)
}

fn main() -> Result<(), Error> {
    env_logger::init();

    let opt = parse_options()?;
    let mut stdin = stdin();
    let mut buffer = Vec::new();
    stdin.read_to_end(&mut buffer)?;
    let schema = derive_schema(&buffer, opt)?;
    println!("Schema: {:#?}", schema);
    Ok(())
}
