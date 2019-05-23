use std::iter::Peekable;
use std::str::Chars;
// use unicode_segmentation::UnicodeSegmentation;

// TODO make this a trait and implement for "rope" like structure.
// aka discontinuous strings.

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

    #[inline]
    pub fn peek(&mut self) -> Option<&char> {
        self.chars.peek()
    }

    #[inline]
    pub fn next(&mut self) -> Option<char> {
        self.chars.next().map(|c| {
            self.n += c.len_utf8();
            c
        })
    }

    // TODO keep?
    #[inline]
    pub fn skip(&mut self, n: usize) {
        // self.n += n;
        for _ in 0..n {
            if self.next().is_none() {
                break;
            }
        }
        // self.chars.skip(n);
    }

    #[inline]
    pub fn offset(&mut self) -> usize {
        self.n
    }

    #[inline]
    pub fn after(&self) -> &'a str {
        &self.s[self.n..]
    }

    #[inline]
    pub fn before(&self) -> &'a str {
        &self.s[0..self.n]
    }

    #[inline]
    pub fn slice_after(&self, length: usize) -> &'a str {
        &self.s[self.n..self.n + length]
    }

    #[inline]
    pub fn slice_before(&self, length: usize) -> &'a str {
        &self.s[self.n - length..self.n]
    }
}

impl<'a> AsRef<str> for Section<'a> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.before()
    }
}

// trait SectionHandler {
//     fn find(s: &mut Section<'_>);
// }

// TODO make this a trait and implement for "rope" like structure.
// aka discontinuous strings.

#[derive(Clone, Debug)]
pub struct ByteSection<'a> {
    n: usize,
    s: &'a [u8],
}

impl<'a> ByteSection<'a> {
    #[inline]
    pub fn new(s: &'a [u8]) -> Self {
        Self { s, n: 0 }
    }

    #[inline]
    pub fn new_with_offset(s: &'a [u8], n: usize) -> Self {
        let s = &s[n..];
        Self { s, n }
    }

    #[inline]
    pub fn expect_byte(&mut self, target: u8) -> bool {
        let n = self.n;
        self.n += 1;
        self.s[n] == target
        // if self.s[self.n] == target {
        //     self.n += 1;
        //     true
        // } else {
        //     false
        // }
    }

    // #[inline]
    // pub fn expect_sequence(&mut self, target: &[u8]) -> bool {
    //     // TODO simd comparison
    // }

    #[inline]
    pub fn peek(&mut self) -> Option<&u8> {
        self.s.get(self.n)
    }

    #[inline]
    pub fn next(&mut self) -> Option<&u8> {
        self.n += 1;
        self.peek()
    }

    // TODO keep?
    #[inline]
    pub fn skip(&mut self, n: usize) {
        self.n += n;
    }

    #[inline]
    pub fn offset(&mut self) -> usize {
        self.n
    }

    #[inline]
    pub fn after(&self) -> &'a str {
        let slice = &self.s[self.n..];
        unsafe {
            std::str::from_utf8_unchecked(slice)
        }
    }

    #[inline]
    pub fn before(&self) -> &'a str {
        let slice = &self.s[0..self.n];
        unsafe {
            std::str::from_utf8_unchecked(slice)
        }
    }

    #[inline]
    pub fn slice_after(&self, length: usize) -> &'a str {
        let slice = &self.s[self.n..self.n + length];
        unsafe {
            std::str::from_utf8_unchecked(slice)
        }
    }

    #[inline]
    pub fn slice_before(&self, length: usize) -> &'a str {
        let slice = &self.s[self.n - length..self.n];
        unsafe {
            std::str::from_utf8_unchecked(slice)
        }
    }
}

impl<'a> AsRef<str> for ByteSection<'a> {
    #[inline]
    fn as_ref(&self) -> &str {
        self.before()
    }
}

// trait SectionHandler {
//     fn find(s: &mut Section<'_>);
// }

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

    #[test]
    fn slice_test() {
        let input = "hello world";
        let mut s = Section::new(input);
        assert_eq!(s.slice_after(5), "hello");
        assert_eq!(s.slice_after(7), "hello w");
        assert_eq!(s.slice_after(0), "");
        assert_eq!(s.slice_before(0), "");
        s.skip(5);
        assert_eq!(s.slice_after(0), "");
        assert_eq!(s.slice_before(0), "");
        assert_eq!(s.slice_after(2), " w");
        assert_eq!(s.slice_before(2), "lo");
    }
}
