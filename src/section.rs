#![warn(clippy::all)]

// TODO make this a trait and implement for "rope" like structure.
// aka discontinuous strings.

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
        let mut s = ByteSection::new(input.as_bytes());
        assert_eq!(&s.src[s.n..], input.as_bytes());
        assert_eq!(s.peek(), Some(&b'h'));
        assert_eq!(s.next(), Some(b'h'));
        assert_eq!(s.n, 1);
        assert_eq!(&s.src[..s.n], b"h");
        for _ in 0..4 {
            s.next();
        }
        assert_eq!(s.peek(), Some(&b' '));
        for _ in 0..10 {
            s.next();
        }
        assert_eq!(s.peek(), None);
        assert_eq!(s.n, input.len());
        assert_eq!(&s.src[..s.n], input.as_bytes());
        assert_eq!(s.src, input.as_bytes());
        assert_eq!(&s.src[s.n..], b"");
    }
}
