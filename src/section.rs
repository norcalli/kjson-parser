#![warn(clippy::all)]

use std::iter::Peekable;
use std::str::Chars;
// use unicode_segmentation::UnicodeSegmentation;

// TODO make this a trait and implement for "rope" like structure.
// aka discontinuous strings.

// TODO rename
pub trait ISection: Clone + Sized {
    type Item: Copy + PartialEq + Eq;
    type Slice;
    // type Iter: IntoIterator<Item = Self::Item>;

    fn peek(&mut self) -> Option<&Self::Item>;

    fn next(&mut self) -> Option<Self::Item>;

    #[inline]
    fn check_next(&mut self, target: Self::Item) -> bool {
        if self.peek() == Some(&target) {
            self.next();
            true
        } else {
            false
        }
    }

    #[inline]
    // fn check_next_pattern<F: Fn(&Self::Item) -> bool>(&mut self, f: F) -> bool {
    fn check_next_pattern<F: Fn(Self::Item) -> bool>(&mut self, f: F) -> bool {
        match self.peek() {
            Some(c) if f(*c) => {
                self.next();
                true
            }
            _ => false,
        }
    }

    // #[inline]
    // fn check_iter<I: IntoIterator<Item = Self::Item>>(&mut self, target: I) -> bool {
    //     for c in target.into_iter() {
    //         if !self.check_next(c) {
    //             return false;
    //         }
    //     }
    //     true
    // }

    fn skip(&mut self, n: usize) -> usize {
        for i in 0..n {
            if self.next().is_none() {
                return i;
            }
        }
        n
    }

    fn offset(&mut self) -> usize;

    fn after(&self) -> Self::Slice;

    #[cfg(test)]
    fn before(&self) -> Self::Slice;
}

#[derive(Clone, Debug)]
pub struct Section<'a> {
    n: usize,
    s: &'a str,
    chars: Peekable<Chars<'a>>,
}

impl<'a> Section<'a> {
    #[inline]
    pub fn new(s: &'a str) -> Self {
        Self {
            s,
            n: 0,
            chars: s.chars().peekable(),
        }
    }

    #[inline]
    pub fn new_with_offset(s: &'a str, n: usize) -> Self {
        let s = &s[n..];
        Self {
            s,
            n,
            chars: s.chars().peekable(),
        }
    }
}

impl<'a> ISection for Section<'a> {
    type Item = char;
    type Slice = &'a str;
    // type Iter = Peekable<Chars<'a>>;

    #[inline]
    fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    #[inline]
    fn next(&mut self) -> Option<char> {
        self.chars.next().map(|c| {
            self.n += c.len_utf8();
            c
        })
    }

    // #[inline]
    // TODO optimize this.
    // fn skip(&mut self, n: usize) -> usize;

    #[inline]
    fn offset(&mut self) -> usize {
        self.n
    }

    #[inline]
    fn after(&self) -> &'a str {
        &self.s[self.n..]
    }

    #[inline]
    #[cfg(test)]
    fn before(&self) -> &'a str {
        &self.s[0..self.n]
    }
}

#[cfg(test)]
impl<'a> AsRef<str> for Section<'a> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.before()
    }
}

// struct SectionOfSections<S: ISection> {
//     n: usize,
//     head: S,
//     sections: Vec<S>,
// }

// impl<S: ISection> SectionOfSections<S> {
//     fn next_section(&mut self) {
//         if let Some(section) = self.sections.pop() {
//             self.n += self.head.offset();
//             self.head = section;
//         }
//     }
// }

// impl<S: ISection> ISection for SectionOfSections<S> {
//     type Item = S::Item;
//     // type Slice = S::Slice;
//     type Slice = std::iter::Flatten<S::Slice>;

//     fn new(s: Self::Slice) -> Self {
//         Self {
//             n: 0,
//             head: s,
//             sections: Vec::new()
//         }
//     }

//     // fn new_with_offset(s: Self::Slice, n: usize) -> Self { }

//     fn peek(&mut self) -> Option<&Self::Item> {
//         self.head.peek()
//     }

//     fn next(&mut self) -> Option<Self::Item> {
//         match self.head.next() {
//             item @ Some(_) => item,
//             None if self.sections.is_empty() => None,
//             None => {
//                 self.next_section();
//                 // self.head.next()
//                 self.next()
//             }
//         }
//     }

//     // // TODO keep this method?
//     // // fn skip(&mut self, n: usize) -> usize;
//     // fn skip(&mut self, mut n: usize) {
//     //     while n > 0 {
//     //     let offset = self.head.offset();
//     //     self.head.skip(n);
//     //     n -= self.head.offset() - offset;
//     //     self.next_section();
//     //     }
//     // }

//     fn offset(&mut self) -> usize {
//         self.n + self.head.offset()
//     }

//     fn after(&self) -> Self::Slice {
//         self.head.after()
//     }

//     fn before(&self) -> Self::Slice {
//         self.head.before()
//     }

//     fn slice_after(&self, length: usize) -> Self::Slice {

//     }

//     fn slice_before(&self, length: usize) -> Self::Slice;
// }

// trait SectionHandler {
//     fn find(s: &mut Section<'_>);
// }

// TODO make this a trait and implement for "rope" like structure.
// aka discontinuous strings.

// use buffered_reader::BufferedReader;

// #[derive(Clone, Debug)]
// pub struct ByteSection< R: BufferedReader<()>> {
//     r: R,
//     n: usize,
// }

// impl<R: BufferedReader<()>> ByteSection<R> {
//     #[inline]
//     pub fn new<T: Into<R>>(t: T) -> Self {
//         Self { r: t.into(), n: 0 }
//     }

//     // #[inline]
//     // pub fn new_with_offset(s: &'a [u8], n: usize) -> Self {
//     //     let s = &s[n..];
//     //     Self { s, n }
//     // }
// }

// impl<R: BufferedReader<()>> ISection for ByteSection<R> {
//     type Item = u8;
//     type Slice = &'_ [u8];

//     // #[inline]
//     // #[inline]
//     // pub fn expect_sequence(&mut self, target: &[u8]) -> bool {
//     //     // TODO simd comparison
//     // }

//     #[inline]
//     fn peek(&mut self) -> Option<&u8> {
//         self.r.data(1).ok().and_then(|s| s.get(0))
//     }

//     #[inline]
//     fn next(&mut self) -> Option<u8> {
//         self.r
//             .data_consume_hard(1)
//             .ok()
//             .and_then(|s| s.get(0))
//             .copied()
//     }

//     // TODO keep?
//     #[inline]
//     fn skip(&mut self, n: usize) {
//         self.r.consume(n);
//     }

//     #[inline]
//     fn offset(&mut self) -> usize {
//         self.n
//     }

//     #[inline]
//     fn after(&self) -> &'a [u8] {
//         self.r.buffer()
//     }

//     // #[inline]
//     // pub fn before(&self) -> &'a str {
//     //     let slice = &self.s[0..self.n];
//     //     unsafe { std::str::from_utf8_unchecked(slice) }
//     // }

//     // #[inline]
//     // pub fn slice_after(&self, length: usize) -> &'a str {
//     //     let slice = &self.s[self.n..self.n + length];
//     //     unsafe { std::str::from_utf8_unchecked(slice) }
//     // }
// }

// impl<'a> AsRef<str> for ByteSection<'a> {
//     #[inline]
//     fn as_ref(&self) -> &str {
//         self.before()
//     }
// }

#[derive(Debug)]
pub struct ByteSection<'a> {
    pub n: usize,
    pub src: &'a [u8],
}

use std::fmt;

impl<'a> fmt::Display for ByteSection<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "ByteSection(n={}, ", self.n)?;
        if let Some(&c) = self.peek() {
            write!(f, "head={}/'{}', ", c, c as char)?;
        } else {
            write!(f, "head=None, ")?;
        }
        write!(f, "s={:?})", self.src)?;
        Ok(())
    }
}

// struct ByteSubSection<'a> {
//     parent: &'a mut ByteSection<'a>,
// }

// impl<'a> ByteSubSection<'a> {
//     pub
// }

impl<'a> ByteSection<'a> {
    // pub fn skip_until_delim(&mut self, terminals: &[u8]) -> usize {
    //     let n = self.n;
    //     self.src[n..].iter().position(
    // }

    // #[inline]
    // pub fn offset_from(&self, n: usize) -> usize {
    //     self.n - n
    // }

    #[inline]
    pub fn new(buf: &'a [u8]) -> ByteSection<'a> {
        ByteSection { n: 0, src: buf }
    }

    #[inline]
    pub fn take(&mut self, n: usize) -> &'a [u8] {
        let result = &self.src[self.n..self.src.len().min(self.n + n)];
        self.n += result.len();
        result
    }

    #[inline]
    pub fn skip_until<F: Fn(&u8) -> bool>(&mut self, f: F) -> usize {
        // let n = self.n;
        if let Some(i) = self.src.iter().skip(self.n).position(f) {
            self.n += i;
        } else {
            self.n = self.src.len();
        }
        // self.n - n
        self.n
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.n == self.src.len()
    }
}

pub trait PeekSeek: Sized {
    type Item: Copy + PartialEq + Eq;

    fn peek(&self) -> Option<&Self::Item>;

    fn next(&mut self) -> Option<Self::Item>;

    #[inline]
    fn check_next(&mut self, target: Self::Item) -> bool {
        if self.peek() == Some(&target) {
            self.next();
            true
        } else {
            false
        }
    }

    #[inline]
    fn check_next_pattern<F: Fn(Self::Item) -> bool>(&mut self, f: F) -> bool {
        match self.peek() {
            Some(c) if f(*c) => {
                self.next();
                true
            }
            _ => false,
        }
    }

    #[inline]
    fn skip(&mut self, n: usize) -> usize {
        for i in 0..n {
            if self.next().is_none() {
                return i;
            }
        }
        n
    }
}

impl PeekSeek for ByteSection<'_> {
    type Item = u8;

    #[inline]
    fn peek(&self) -> Option<&u8> {
        self.src.get(self.n)
    }

    #[inline]
    fn next(&mut self) -> Option<u8> {
        let result = self.peek().copied();
        self.n += 1;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_section_test() {
        let input = "hello world";
        let mut s = Section::new(input);
        assert_eq!(s.after(), input);
        assert_eq!(s.peek(), Some(&'h'));
        assert_eq!(s.next(), Some('h'));
        assert_eq!(s.offset(), 1);
        assert_eq!(s.before(), "h");
        for _ in 0..4 {
            s.next();
        }
        assert_eq!(s.peek(), Some(&' '));
        for _ in 0..10 {
            s.next();
        }
        assert_eq!(s.peek(), None);
        assert_eq!(s.offset(), input.len());
        assert_eq!(s.before(), input);
        assert_eq!(s.as_ref(), input);
        assert_eq!(s.after(), "");
    }
}
