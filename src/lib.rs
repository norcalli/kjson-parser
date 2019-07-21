#![warn(clippy::all)]

pub mod section;
pub mod tokenizer;
pub mod validator;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub enum JsonType {
    Array,
    Object,
    String,
    Number,
    Bool,
    Null,
}

use std::borrow::{Borrow, Cow};

// #[derive(Debug)]
// pub enum JsonPathSegment<'a> {
//     // Root,
//     ArrayValue(usize),
//     ObjectValue(Cow<'a, str>),
// }

#[derive(Debug, derive_more::Display)]
pub enum JsonPathSegment<'a> {
    // Root,
    #[display(fmt = "{}", _0)]
    Array(usize),
    #[display(fmt = "{}", _0)]
    Object(Cow<'a, str>),
}

impl<'a> JsonPathSegment<'a> {
    pub fn as_key(&self) -> Option<&str> {
        use JsonPathSegment::*;
        match self {
            Object(ref s) => Some(s.borrow()),
            Array(_) => None,
        }
    }

    pub fn as_index(&self) -> Option<usize> {
        use JsonPathSegment::*;
        match self {
            Object(_) => None,
            Array(index) => Some(*index),
        }
    }

    pub fn is_array(&self) -> bool {
        if let JsonPathSegment::Array(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_object(&self) -> bool {
        !self.is_array()
    }
}

// pub type JsonPath<'a> = Vec<JsonPathSegment<'a>>;
// pub type JsonPath<'a> = Cow<'a, [JsonPathSegment<'a>]>;

#[cfg(test)]
mod tests {

    // #[test]
    // fn it_parses_a_stream() {
    //     let tests = [
    //         (r"1 1", [1, 1]),
    //         (r#"1 "123""#, ["1", r#""123""#]),
    //         (r#"1"123""#, ["1", r#""123""#]),
    //         (r#"[1]{"a": null}"#, ["[1]", r#"{"a":null}"#]),
    //     ];
    // }

    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
