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

#[derive(Debug, derive_more::Display)]
pub enum JsonPathSegment<'a> {
    // Root,
    #[display(fmt = "{}", _0)]
    Index(usize),
    #[display(fmt = "{}", _0)]
    Key(Cow<'a, str>),
}

impl<'a> JsonPathSegment<'a> {
    pub fn as_key(&self) -> Option<&str> {
        use JsonPathSegment::*;
        match self {
            Key(ref s) => Some(s.borrow()),
            Index(_) => None,
        }
    }

    pub fn as_index(&self) -> Option<usize> {
        use JsonPathSegment::*;
        match self {
            Key(_) => None,
            Index(index) => Some(*index),
        }
    }

    pub fn as_key_mut(&mut self) -> Option<&mut Cow<'a, str>> {
        use JsonPathSegment::*;
        match self {
            Key(ref mut s) => Some(s),
            Index(_) => None,
        }
    }

    pub fn as_index_mut(&mut self) -> Option<&mut usize> {
        use JsonPathSegment::*;
        match self {
            Key(_) => None,
            Index(ref mut index) => Some(index),
        }
    }

    pub fn is_index(&self) -> bool {
        if let JsonPathSegment::Index(_) = self {
            true
        } else {
            false
        }
    }

    pub fn is_key(&self) -> bool {
        !self.is_index()
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
