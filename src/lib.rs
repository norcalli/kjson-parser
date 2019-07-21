#![warn(clippy::all)]

pub mod section;
pub mod tokenizer;
pub mod validator;

use std::borrow::Cow;
use std::fmt;

#[derive(Hash, Eq, PartialEq, Debug, Copy, Clone)]
pub enum JsonType {
    Array,
    Object,
    String,
    Number,
    Bool,
    Null,
}

#[derive(Hash, Eq, PartialEq, Clone, Debug, derive_more::Display, derive_more::From)]
pub enum JsonPathSegment<'a> {
    // Root,
    #[display(fmt = "{}", _0)]
    Index(usize),
    #[display(fmt = "{}", _0)]
    Key(Cow<'a, str>),
}

impl<'a> JsonPathSegment<'a> {
    pub fn as_key(&self) -> Option<Cow<'a, str>> {
        use JsonPathSegment::*;
        match self {
            // TODO do I need to clone?
            Key(c) => Some(c.clone()),
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

pub const EMPTY_KEY: JsonPathSegment<'static> = JsonPathSegment::Key(Cow::Borrowed(""));
// pub const EMPTY_INDEX: JsonPathSegment<'static> = JsonPathSegment::Index(std::usize::MAX);

#[derive(
    Hash,
    Clone,
    Eq,
    PartialEq,
    derive_more::From,
    derive_more::Constructor,
    derive_deref::Deref,
    derive_deref::DerefMut,
)]
pub struct JsonPath<'a>(Cow<'a, [JsonPathSegment<'a>]>);

impl<'a> JsonPath<'a> {
    // TODO optimize
    pub fn parent(&self) -> Self {
        let mut slice = self.0.clone().into_owned();
        slice.pop();
        JsonPath::new(slice.into())
    }
}

impl<'a> fmt::Display for JsonPath<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "@")?;
        for part in self.0.iter() {
            write!(f, ".{}", part)?;
        }
        Ok(())
    }
}
