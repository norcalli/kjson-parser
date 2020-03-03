// Copyright © 2019 Ashkan Kiani

// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use crate::section::{ByteSection, PeekSeek};
use crate::JsonType;

use std::borrow::{Cow, ToOwned};
use std::io;

// #[derive(Debug, PartialEq, Eq, derive_more::From)]
#[derive(Clone, Debug, derive_more::From)]
pub enum TokenizeError {
    UnexpectedByte(u8),
    UnexpectedByteWithContext {
        byte: u8,
        context: TokenContext,
    },
    UnexpectedEndOfInput,
    UnexpectedEndOfInputWithContext {
        context: Option<TokenContext>,
        expected_byte: Option<u8>,
        recovery_point: Option<usize>,
        token_start: Option<usize>,
    },
    // TODO make this &str?
    InvalidStringUnicodeEscape(Vec<u8>),
    InvalidStringEscape(u8),
    InvalidStringCodepoint(u32),
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
/// This is only relevant for multi-byte tokens, so those are the only
/// ones here.
pub enum TokenContext {
    String,
    // StringUtf8Byte,
    // StringEscapeCharacter,
    // StringUtf8EscapeCharacter,
    Number,
    // NumberFraction,
    // NumberExponent,
    True,
    False,
    Null,
}

// impl TokenContext {
//     pub fn is_string(self) -> bool {
//         match self {
//             TokenContext::String
//             | TokenContext::StringUtf8Byte
//             | TokenContext::StringEscapeCharacter
//             | TokenContext::StringUtf8EscapeCharacter => true,
//             _ => false,
//         }
//     }
//     pub fn is_number(self) -> bool {
//         match self {
//             TokenContext::Number
//             | TokenContext::NumberFraction
//             | TokenContext::NumberExponent => true,
//             _ => false,
//         }
//     }
// }

impl TokenizeError {
    pub fn is_eof(&self) -> bool {
        use TokenizeError::*;
        match self {
            UnexpectedEndOfInput => true,
            UnexpectedEndOfInputWithContext { .. } => true,
            _ => false,
        }
    }

    pub fn recovery_point(&self) -> Option<usize> {
        use TokenizeError::*;
        match self {
            UnexpectedEndOfInputWithContext { recovery_point, .. } => *recovery_point,
            _ => None,
        }
    }

    pub fn token_start(&self) -> Option<usize> {
        use TokenizeError::*;
        match self {
            UnexpectedEndOfInputWithContext { token_start, .. } => *token_start,
            _ => None,
        }
    }

    pub fn context(&self) -> Option<TokenContext> {
        use TokenizeError::*;
        match self {
            UnexpectedEndOfInputWithContext { context, .. } => *context,
            UnexpectedByteWithContext { context, .. } => Some(*context),
            _ => None,
        }
    }

    pub fn with_recovery_point(self, n: usize) -> TokenizeError {
        use TokenizeError::*;
        match self {
            UnexpectedEndOfInput => UnexpectedEndOfInputWithContext {
                context: None,
                expected_byte: None,
                recovery_point: Some(n),
                token_start: None,
            },
            UnexpectedEndOfInputWithContext {
                context,
                expected_byte,
                token_start,
                ..
            } => UnexpectedEndOfInputWithContext {
                context,
                expected_byte,
                recovery_point: Some(n),
                token_start,
            },
            other => other,
        }
    }

    pub fn with_token_start(self, n: usize) -> TokenizeError {
        use TokenizeError::*;
        match self {
            UnexpectedEndOfInput => UnexpectedEndOfInputWithContext {
                context: None,
                expected_byte: None,
                recovery_point: None,
                token_start: Some(n),
            },
            UnexpectedEndOfInputWithContext {
                context,
                expected_byte,
                recovery_point,
                ..
            } => UnexpectedEndOfInputWithContext {
                context,
                expected_byte,
                recovery_point,
                token_start: Some(n),
            },
            other => other,
        }
    }

    pub fn with_context(self, context: TokenContext) -> TokenizeError {
        use TokenizeError::*;
        match self {
            UnexpectedByte(byte) => UnexpectedByteWithContext { byte, context },
            UnexpectedEndOfInput => UnexpectedEndOfInputWithContext {
                context: Some(context),
                expected_byte: None,
                recovery_point: None,
                token_start: None,
            },
            UnexpectedEndOfInputWithContext {
                expected_byte,
                recovery_point,
                token_start,
                ..
            } => UnexpectedEndOfInputWithContext {
                context: Some(context),
                expected_byte,
                recovery_point,
                token_start,
            },
            other => other,
        }
    }
}

#[derive(Debug)]
pub struct TokenizeErrorContext {
    expected_byte: Option<u8>,
    token_context: Option<TokenContext>,
    recovery_point: Option<usize>,
}

// impl TokenizeError {
//     pub fn rewind_count(&self) -> Option<usize> {
//         match self {
//     UnexpectedByte(u8) =>
//     UnexpectedCharacter(char),
//     UnexpectedEndOfInput,
//     // TODO make this &str?
//     InvalidStringUnicodeEscape(Vec<u8>),
//     // InvalidStringUnicodeEscape(String),
//     // InvalidStringEscape(char),
//     InvalidStringEscape(u8),
//     InvalidStringCharacter(char),
//     Io(io::Error),
//         }
//     }
// }

pub type TokenizeResult<T> = std::result::Result<T, TokenizeError>;

#[derive(Debug, PartialEq, Clone)]
pub enum Token<'a> {
    /// Compress up to 255 sequential spaces to 1 byte.
    Spaces(u8),
    Whitespace(char),
    ObjectOpen,
    ObjectClose,
    Comma,
    Colon,
    // String(Cow<'a, str>),
    // Number(Cow<'a, str>),
    String(Cow<'a, [u8]>),
    Number(Cow<'a, [u8]>),
    ArrayOpen,
    ArrayClose,
    // Bool(bool),
    True,
    False,
    Null,
}

// enum Whitespace {
//     Newline,
//     Tab,
//     Space,
// }

impl<'a> Token<'a> {
    #[inline]
    pub fn into_owned(self) -> Token<'static> {
        match self {
            Token::Spaces(x) => Token::Spaces(x),
            Token::Whitespace(x) => Token::Whitespace(x),
            Token::String(x) => Token::String(Cow::Owned(x.into_owned())),
            Token::Number(x) => Token::Number(Cow::Owned(x.into_owned())),
            Token::ObjectOpen => Token::ObjectOpen,
            Token::ObjectClose => Token::ObjectClose,
            Token::Comma => Token::Comma,
            Token::Colon => Token::Colon,
            Token::ArrayOpen => Token::ArrayOpen,
            Token::ArrayClose => Token::ArrayClose,
            Token::Null => Token::Null,
            Token::True => Token::True,
            Token::False => Token::False,
        }
    }

    #[inline]
    pub fn is_whitespace(&self) -> bool {
        match self {
            Token::Whitespace(_) | Token::Spaces(_) => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_close(&self) -> bool {
        match self {
            Token::ArrayClose | Token::ObjectClose => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_open(&self) -> bool {
        match self {
            Token::ArrayOpen | Token::ObjectOpen => true,
            _ => false,
        }
    }

    pub fn value_type(&self) -> Option<JsonType> {
        Some(match self {
            Token::String(_) => JsonType::String,
            Token::Number(_) => JsonType::Number,
            Token::ObjectOpen | Token::ObjectClose => JsonType::Object,
            Token::ArrayOpen | Token::ArrayClose => JsonType::Array,
            Token::Null => JsonType::Null,
            Token::True | Token::False => JsonType::Bool,
            _ => return None,
        })
    }

    #[inline]
    pub fn is_value_start(&self) -> bool {
        match self {
            Token::False
            | Token::True
            | Token::Null
            | Token::Number(_)
            | Token::String(_)
            | Token::ArrayOpen
            | Token::ObjectOpen => true,
            _ => false,
        }
    }

    #[inline]
    pub fn is_complete_value(&self) -> bool {
        match self {
            Token::False | Token::True | Token::Null | Token::Number(_) | Token::String(_) => true,
            _ => false,
        }
    }

    // #[inline]
    // pub unsafe fn into_string_unchecked(self) -> Option<Cow<'a, str>> {
    //     match self {
    //         // Token::String(c) => Some(c),
    //         // TODO use unchecked?
    //         Token::String(c) => Some(match c {
    //             Cow::Borrowed(bytes) => {
    //                 Cow::Borrowed(unsafe { std::str::from_utf8_unchecked(bytes) })
    //             }
    //             Cow::Owned(bytes) => Cow::Owned(unsafe { String::from_utf8_unchecked(bytes) }),
    //         }),
    //         _ => None,
    //     }
    // }

    // #[inline]
    // // This assumes that my utf8 validation is correct lmao.
    // pub fn into_string_unchecked(self) -> Option<Cow<'a, str>> {
    //     match self {
    //         // Token::String(c) => Some(c),
    //         // TODO use unchecked?
    //         Token::String(c) => Some(match c {
    //             Cow::Borrowed(bytes) => {
    //                 Cow::Borrowed(unsafe { std::str::from_utf8_unchecked(bytes) })
    //             }
    //             Cow::Owned(bytes) => Cow::Owned(unsafe { String::from_utf8_unchecked(bytes) }),
    //         }),
    //         _ => None,
    //     }
    // }

    #[inline]
    pub fn as_string(&self) -> Option<&str> {
        match self {
            // Token::String(c) => Some(c),
            // TODO use unchecked?
            Token::String(c) => std::str::from_utf8(c).ok(),
            _ => None,
        }
    }

    #[inline]
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Token::True => Some(true),
            Token::False => Some(false),
            _ => None,
        }
    }

    #[inline]
    pub fn as_number(&self) -> Option<&[u8]> {
        match self {
            Token::Number(x) => Some(&x),
            _ => None,
        }
    }

    #[inline]
    /// Returns true if this can be matched completely, but that
    /// match could be a false positive. Meaning that if you actually
    /// had more data incoming, you should restart the token matching
    /// from the beginning of the last token.
    pub fn potential_false_positive(&self) -> bool {
        match self {
            Token::Number(_) => true,
            _ => false,
        }
    }

    // #[inline]
    // pub fn byte_count(&self) -> usize {
    //     match self {
    //         Token::Number(ref c) | Token::String(ref c) => c.len(),
    //         Token::Spaces(ref c) => c.len(),
    //         _ => false,
    //     }
    // }

    #[inline]
    pub fn char_count(&self) -> usize {
        use Token::*;

        match self {
            String(ref s) | Number(ref s) => s.len(),
            Spaces(ref n) => *n as usize,
            Null | True => 4,
            False => 5,
            ObjectOpen | ObjectClose | Comma | Colon | ArrayOpen | ArrayClose | Whitespace(_) => 1,
        }
    }

    #[inline]
    pub fn print<W: io::Write>(&self, mut writer: W) -> io::Result<()> {
        match self {
            // Token::String(x) | Token::Number(x) => write!(writer, "{}", x),
            // Token::Whitespace(x) => write!(writer, "{}", x),
            Token::String(x) | Token::Number(x) => writer.write_all(x),
            Token::Whitespace(x) => write!(writer, "{}", x),
            Token::Spaces(x) => {
                for _ in 0..*x {
                    writer.write_all(b" ")?;
                }
                Ok(())
            }
            Token::ObjectOpen => writer.write_all(b"{"),
            Token::ObjectClose => writer.write_all(b"}"),
            Token::Comma => writer.write_all(b","),
            Token::Colon => writer.write_all(b":"),
            Token::ArrayOpen => writer.write_all(b"["),
            Token::ArrayClose => writer.write_all(b"]"),
            Token::Null => writer.write_all(b"null"),
            Token::False => writer.write_all(b"false"),
            Token::True => writer.write_all(b"true"),
        }
    }
}

trait TokenizerTape: PeekSeek<Item = u8> {
    #[inline]
    fn expect_next(&mut self, target: Self::Item) -> TokenizeResult<()> {
        match self.next() {
            Some(c) if c == target => Ok(()),
            Some(c) => Err(TokenizeError::UnexpectedByte(c)),
            None => Err(TokenizeError::UnexpectedEndOfInput),
        }
    }

    #[inline]
    fn expect(&mut self) -> TokenizeResult<u8> {
        self.next().ok_or(TokenizeError::UnexpectedEndOfInput)
    }

    #[inline]
    fn expect_next_pattern<F: Fn(u8) -> bool>(&mut self, f: F) -> TokenizeResult<u8> {
        match self.next() {
            Some(c) if f(c) => Ok(c),
            Some(c) => Err(TokenizeError::UnexpectedByte(c)),
            None => Err(TokenizeError::UnexpectedEndOfInput),
        }
    }
}

impl<'a> TokenizerTape for ByteSection<'a> {}

pub mod utils {
    use super::*;

    use crate::utf8;

    /// https://kevinlynagh.com/notes/match-vs-lookup/
    use crate::lookup_tables::{
        DIGIT_TABLE, HEXDIGIT_TABLE, SINGLE_ESCAPE_CHARACTERS, STRING_TERMINALS, WHITESPACE_TABLE,
    };

    // NO DIGIT LUT seems faster
    const USE_DIGIT_LUT: bool = false;
    const USE_STRING_ESCAPE_BINARY_SEARCH: bool = false;
    const USE_WHITESPACE_LUT: bool = false;

    #[inline]
    pub fn invalid_input_err(s: Option<u8>) -> TokenizeError {
        match s {
            Some(c) => TokenizeError::UnexpectedByte(c),
            None => TokenizeError::UnexpectedEndOfInput,
        }
    }

    // TODO double check that this is complete.
    #[inline]
    pub fn is_whitespace(c: u8) -> bool {
        if USE_WHITESPACE_LUT {
            WHITESPACE_TABLE[c as usize]
        } else {
            c == b' ' || c == b'\t' || c == b'\n'
        }
    }

    #[inline]
    pub fn is_hexdigit(c: u8) -> bool {
        HEXDIGIT_TABLE[c as usize]
    }

    #[inline]
    pub fn is_digit(c: u8) -> bool {
        if USE_DIGIT_LUT {
            DIGIT_TABLE[c as usize]
        } else {
            c ^ 0x30 < 10
        }
    }

    // #[inline]
    // pub fn is_nonzero_digit(c: u8) -> bool {
    //     NONZERO_DIGIT_TABLE[c as usize]
    // }

    #[inline]
    pub fn section_digits(s: &mut ByteSection<'_>) -> TokenizeResult<usize> {
        let n = s.n;
        while s.check_next_pattern(is_digit) {}

        // TODO make sure these compile down to the same thing.
        // while let Some(c) = s.peek() {
        //     // if !DIGIT_TABLE[c as usize] {
        //     if !is_digit(c) {
        //         break;
        //     }
        //     s.next();
        // }
        Ok(s.n - n)
    }

    #[inline]
    pub fn section_hexdigits(s: &mut ByteSection<'_>) -> TokenizeResult<usize> {
        let n = s.n;
        while s.check_next_pattern(is_hexdigit) {}
        Ok(s.n - n)
    }

    #[inline]
    pub fn section_number_frac(s: &mut ByteSection<'_>) -> TokenizeResult<()> {
        if s.check_next(b'.') {
            s.expect_next_pattern(is_digit)?;
            section_digits(s)?;
        }
        Ok(())
    }

    #[inline]
    pub fn section_number_exp(s: &mut ByteSection<'_>) -> TokenizeResult<()> {
        // TODO add even more fine grained context, such as InExponent, in Fraction, etc?
        if s.check_next_pattern(|c| (c | 0x20) == b'e') {
            // TODO should I add is_digit in this pattern check here as well?
            // TODO I could probably replace this with a bit op to make it one operation.
            // c ^ 41 => (43, 2) (45, 4) ...
            s.check_next_pattern(|c| c == b'-' || c == b'+');
            s.expect_next_pattern(is_digit)?;
            section_digits(s)?;
        }
        Ok(())
    }

    #[inline]
    pub fn section_positive_number(s: &mut ByteSection<'_>) -> TokenizeResult<()> {
        if s.check_next(b'0') {
            section_number_frac(s)?;
            section_number_exp(s)?;
            return Ok(());
        }
        s.expect_next_pattern(is_digit)?;
        section_digits(s)?;
        section_number_frac(s)?;
        section_number_exp(s)?;
        Ok(())
    }

    /// Consume number according to JSON RFC 7159.
    /// Grammar:
    ///   number = [ minus ] int [ frac ] [ exp ]
    ///   decimal-point = %x2E       ; .
    ///   digit1-9 = %x31-39         ; 1-9
    ///   e = %x65 / %x45            ; e E
    ///   exp = e [ minus / plus ] 1*DIGIT
    ///   frac = decimal-point 1*DIGIT
    ///   int = zero / ( digit1-9 *DIGIT )
    ///   minus = %x2D               ; -
    ///   plus = %x2B                ; +
    ///   zero = %x30                ; 0
    #[inline]
    pub fn section_number(s: &mut ByteSection<'_>) -> TokenizeResult<()> {
        s.check_next(b'-');
        section_positive_number(s)
    }

    /// Sorted. ['"', '/', '\\', 'b', 'f', 'n', 'r', 't']
    const ESCAPE_CHARACTERS: &[u8] = &[34, 47, 92, 98, 102, 110, 114, 116];

    #[inline]
    fn section_unicode_escape(s: &mut ByteSection<'_>) -> TokenizeResult<()> {
        let buf = s.take(4);
        if buf.len() < 4 {
            return Err(TokenizeError::UnexpectedEndOfInput);
        }
        if !(is_hexdigit(buf[0])
            && is_hexdigit(buf[1])
            && is_hexdigit(buf[2])
            && is_hexdigit(buf[3]))
        {
            return Err(TokenizeError::InvalidStringUnicodeEscape(buf.to_vec()));
        }
        Ok(())
    }

    #[inline]
    pub fn section_inside_string(s: &mut ByteSection<'_>) -> TokenizeResult<()> {
        let start = s.n;
        loop {
            // We only care about escape characters and codepoints for special processing.
            // Otherwise, just skip ahead.
            s.skip_until_pattern(|c| STRING_TERMINALS[c as usize]);

            let recovery_point = s.n;
            let error_handler = move |e: TokenizeError| {
                e.with_context(TokenContext::String)
                    .with_recovery_point(recovery_point)
                    // TODO keep this?
                    .with_token_start(start)
            };

            match s.expect().map_err(error_handler)? {
                // End of string
                b'"' => {
                    return Ok(());
                }
                // Escape character.
                b'\\' => {
                    match s.expect().map_err(error_handler)? {
                        b'u' => section_unicode_escape(s).map_err(error_handler)?,
                        c => {
                            // TODO idk which one of these is faster. I need to godbolt it.
                            if USE_STRING_ESCAPE_BINARY_SEARCH {
                                ESCAPE_CHARACTERS
                                    .binary_search(&c)
                                    .map_err(|_| TokenizeError::UnexpectedByte(c))
                                    .map_err(error_handler)?;
                            } else {
                                if !SINGLE_ESCAPE_CHARACTERS[c as usize] {
                                    return Err(error_handler(TokenizeError::UnexpectedByte(c)));
                                }
                            }
                        }
                    }
                }
                x if x < 0x20 => {
                    return Err(TokenizeError::InvalidStringCodepoint(x as u32));
                }
                x => {
                    let mut width = utf8::utf8_char_width(x) as usize;
                    if width == 0 {
                        return Err(TokenizeError::InvalidStringCodepoint((x as u32) << 28));
                    }
                    // We already have one.
                    width -= 1;
                    // TODO optimize this
                    let buf = s.take(width);
                    if buf.len() < width {
                        return Err(TokenizeError::UnexpectedEndOfInput).map_err(error_handler);
                    }
                    // TODO this is inconsistent with above where I don't actually bother with
                    // validation.
                    // TODO should this potentially error out?
                    // let codepoint = utf8::definitely_next_codepoint(x, buf);
                }
            }
        }
    }

    /// Consume string according to JSON RFC 7159.
    /// Grammar:
    ///   string = quotation-mark *char quotation-mark
    ///   char = unescaped /
    ///       escape (
    ///           %x22 /          ; "    quotation mark  U+0022
    ///           %x5C /          ; \    reverse solidus U+005C
    ///           %x2F /          ; /    solidus         U+002F
    ///           %x62 /          ; b    backspace       U+0008
    ///           %x66 /          ; f    form feed       U+000C
    ///           %x6E /          ; n    line feed       U+000A
    ///           %x72 /          ; r    carriage return U+000D
    ///           %x74 /          ; t    tab             U+0009
    ///           %x75 4HEXDIG )  ; uXXXX                U+XXXX
    ///   escape = %x5C              ; \
    ///   quotation-mark = %x22      ; "
    ///   unescaped = %x20-21 / %x23-5B / %x5D-10FFFF
    #[inline]
    pub fn section_string(s: &mut ByteSection<'_>) -> TokenizeResult<()> {
        let start = s.n;
        if !s.check_next(b'"') {
            return Err(invalid_input_err(s.peek())
                .with_context(TokenContext::String)
                .with_token_start(start)
                .with_recovery_point(s.n));
        }

        section_inside_string(s).map_err(|e| e.with_token_start(start))
    }

    /* TODO benchmark against this/check the ASM output on the whitespace handling...?
    pub fn next_token<'a>(s: &mut Section<'a>) -> Result<Option<Token<'a>>, TokenizeError> {
        Ok(Some(match s.peek() {
            Some(&c) => match c {
                '\n' | '\t' => {
                    s.next();
                    Token::Whitespace(c)
                }
                ' ' => {
                    let mut n = 1;
                    s.next();
                    while let Some(' ') = s.peek() {
                        s.next();
                        n += 1;
                        if n >= 255 {
                            break;
                        }
                    }
                    Token::Spaces(n)
                }
                // Valid number starters
                '-' | '0'..='9' => {
                    let n = utils::section_number(s)?;
                    Token::Number(s.slice_before(n))
                }
                '"' => {
                    let n = utils::section_string(s)?;
                    Token::String(s.slice_before(n))
                }
                // '[' => Token::ArrayOpen,
                // ']' => Token::ArrayClose,
                // '{' => Token::ObjectOpen,
                // '}' => Token::ObjectClose,
                // ':' => Token::Colon,
                // ',' => Token::Comma,
                'n' => {
                    s.next();
                    // TODO peek then next instead?
                    // Does consuming prematurely matter?
                    if s.next() == Some('u') && s.next() == Some('l') && s.next() == Some('l') {
                        Token::Null
                    } else {
                        return Err(utils::invalid_input_err(s.peek()));
                    }
                }
                e => {
                    let token = match e {
                        '[' => Token::ArrayOpen,
                        ']' => Token::ArrayClose,
                        '{' => Token::ObjectOpen,
                        '}' => Token::ObjectClose,
                        ':' => Token::Colon,
                        ',' => Token::Comma,
                        e => return Err(utils::invalid_input_err(Some(&e))),
                    };
                    s.next();
                    token
                }
            },
            None => return Ok(None),
        }))
    }
    */
    // TODO have an option/variant to get a next_token which also decodes the String rather than
    // finding the edges and returning it as is.

    
    #[inline]
    pub fn compress_next_token<'a, F: Fn(u8) -> bool>(
        s: &mut ByteSection<'a>,
        compressed_whitespace: F,
    ) -> TokenizeResult<Token<'a>> {
        // TODO use this?
        // Should I mark the start of a token?
        // let recovery_point = s.n;
        let start = s.n;
        (|| -> TokenizeResult<Token<'a>> {
            Ok(match s.expect()? {
                c if compressed_whitespace(c) => {
                    let mut n = 1;
                    // TODO I could probs clean this up.
                    while let Some(c) = s.peek() {
                        if !is_whitespace(c) {
                            break;
                        }
                        s.next();
                        n += 1;
                        if n == 255 {
                            break;
                        }
                    }
                    Token::Spaces(n)
                }
                c @ b' ' | c @ b'\n' | c @ b'\t' => Token::Whitespace(c as char),
                b'-' => {
                    let recovery_point = start;
                    let error_handler = move |e: TokenizeError| {
                        e.with_context(TokenContext::Number)
                            .with_recovery_point(recovery_point)
                    };

                    section_positive_number(s).map_err(error_handler)?;
                    Token::Number(s.src[start..s.n].into())
                }
                b'0' => {
                    let recovery_point = start;
                    let error_handler = move |e: TokenizeError| {
                        e.with_context(TokenContext::Number)
                            .with_recovery_point(recovery_point)
                    };

                    section_number_frac(s).map_err(error_handler)?;
                    section_number_exp(s).map_err(error_handler)?;
                    Token::Number(s.src[start..s.n].into())
                }
                // Valid number starters
                b'1'..=b'9' => {
                    let recovery_point = start;
                    let error_handler = move |e: TokenizeError| {
                        e.with_context(TokenContext::Number)
                            .with_recovery_point(recovery_point)
                    };

                    section_digits(s).map_err(error_handler)?;
                    section_number_frac(s).map_err(error_handler)?;
                    section_number_exp(s).map_err(error_handler)?;
                    Token::Number(s.src[start..s.n].into())
                }
                // b'-' | b'0'..=b'9' => {
                //     // TODO is there a way to avoid reprocessing that first byte?
                //     s.n -= 1;
                //     let n = s.n;
                //     section_number(s)?;
                //     Token::Number(s.src[n..s.n].into())
                // }
                b'"' => {
                    section_inside_string(s)?;
                    Token::String(s.src[start..s.n].into())
                }
                b'n' => {
                    let recovery_point = start;
                    let error_handler = move |e: TokenizeError| {
                        e.with_context(TokenContext::Null)
                            .with_recovery_point(recovery_point)
                    };

                    s.expect_next(b'u').map_err(error_handler)?;
                    s.expect_next(b'l').map_err(error_handler)?;
                    s.expect_next(b'l').map_err(error_handler)?;
                    Token::Null
                }
                b't' => {
                    let recovery_point = start;
                    let error_handler = move |e: TokenizeError| {
                        e.with_context(TokenContext::True)
                            .with_recovery_point(recovery_point)
                    };
                    s.expect_next(b'r').map_err(error_handler)?;
                    s.expect_next(b'u').map_err(error_handler)?;
                    s.expect_next(b'e').map_err(error_handler)?;
                    Token::True
                }
                b'f' => {
                    let recovery_point = start;
                    let error_handler = move |e: TokenizeError| {
                        e.with_context(TokenContext::False)
                            .with_recovery_point(recovery_point)
                    };
                    s.expect_next(b'a').map_err(error_handler)?;
                    s.expect_next(b'l').map_err(error_handler)?;
                    s.expect_next(b's').map_err(error_handler)?;
                    s.expect_next(b'e').map_err(error_handler)?;
                    Token::False

                    // // TODO peek then next instead?
                    // // Does consuming prematurely matter?
                    // // 2019-07-24: Turns out, yes. This means that I can't use it for certain
                    // // applications like extracting json from a stream of maybe bytes.
                    // if s.check_next(b'a')
                    //     && s.check_next(b'l')
                    //     && s.check_next(b's')
                    //     && s.check_next(b'e')
                    // {
                    //     Token::False
                    // } else {
                    //     return Err(invalid_input_err_with_context(s.peek(), JsonType::Bool));
                    // }
                }
                b'[' => Token::ArrayOpen,
                b']' => Token::ArrayClose,
                b'{' => Token::ObjectOpen,
                b'}' => Token::ObjectClose,
                b':' => Token::Colon,
                b',' => Token::Comma,
                e => return Err(TokenizeError::UnexpectedByte(e)),
            })
        })()
        .map_err(|e| e.with_token_start(start))
    }

    #[inline]
    pub fn next_token<'a>(s: &mut ByteSection<'a>) -> TokenizeResult<Token<'a>> {
        compress_next_token(s, |c| c == b' ')
    }

    // TODO I should probably use this
    // pub fn peek_next_token<'a>(s: &mut Section<'a>) -> Result<Option<Token<'a>>, TokenizeError> {
    //     next_token(&mut s.clone())
    // }

    // // TODO assume that last_token is from the same section as before? If it is a new
    // // section, then the lifetimes have to be different.
    // pub fn try_resume<'a>(s: &mut Section<'a>, last_token: Token<'a>) -> Token<'a> {
    //     if last_token.partial_false_positive() {
    //         match last_token {
    //             Token::Number(c) => {
    //                 let buffer = c.clone().into_owned();
    //                 let mut section = s.clone();
    //                 section.
    //             }
    //         }
    //     }
    // }

    #[cfg(test)]
    mod tests {
        use super::*;
        use matches::*;

        #[test]
        fn section_number_matches_full() {
            for (input, remaining) in &[
                ("1", ""),
                ("1a", "a"),
                ("1\n", "\n"),
                ("1.0", ""),
                ("1.1231231231", ""),
                ("1e0", ""),
                ("1e10", ""),
                ("1e-10", ""),
                ("1E0", ""),
                ("1E10", ""),
                ("1E-10", ""),
            ] {
                let mut s = Section::new(input);
                // assert_matches!(section_number(&mut s), Ok(_));
                assert_eq!(
                    section_number(&mut s),
                    Ok(input.len() - remaining.len()),
                    "SECTION: {:?}",
                    s
                );
                assert_eq!(remaining, &s.after());
            }
        }

        #[test]
        fn section_number_invalid_input() {
            for (input, err) in vec![
                ("1ea", invalid_input_err(Some(&'a'))),
                ("1Ea", invalid_input_err(Some(&'a'))),
                // TODO hmmm
                // ("01", invalid_input_err(Some(&'1'))),
                ("0.", invalid_input_err(None)), // TODO should I make this a "missing fraction" error?
                ("0.-", invalid_input_err(Some(&'-'))),
            ] {
                let mut s = Section::new(input);
                assert_eq!(section_number(&mut s), Err(err), "{:?}", s);
            }
        }

        #[test]
        fn section_string_matches_full() {
            let input = r#"
            "foo"
            "bar"
            "baz"
            "¥12,110"
            "hello world"
            "test \n\t\r\b\f\\\""
            "test \t"
            "test \"321"
            "test \u0000"
            "test \u1221"
            "test \ua221"
            "test \uA22A"
            "test \uABCD"
            "test \uEF01"
            "#;
            for x in input
                .split('\n')
                .map(|x| x.trim())
                .filter(|x| !x.is_empty())
            {
                let s = &mut Section::new(x);
                println!("{:?}", s);
                assert_matches!(section_string(s), Ok(_));
                assert_eq!(x, s.before());
            }
        }

        #[test]
        fn section_string_invalid() {
            let input = r#"
            "test \ua22 "
            "test \ua22"
            "test \uZ22A"
            "test \uZ22A
            "#;
            for x in input
                .split('\n')
                .map(|x| x.trim())
                .filter(|x| !x.is_empty())
            {
                let s = &mut Section::new(x);
                println!("{:?}", s);
                // TODO be more specific?
                assert_matches!(section_string(s), Err(_));
            }
        }

        #[test]
        fn next_token_spaces() -> Result<(), TokenizeError> {
            use Token::*;

            let input = " ".repeat(320);
            let s = &mut Section::new(&input);
            assert_eq!(next_token(s)?, Some(Spaces(255)));
            assert_eq!(next_token(s)?, Some(Spaces(65)));
            assert_eq!(next_token(s)?, None);

            let input = "   \n\t   ";
            let s = &mut Section::new(&input);
            assert_eq!(
                compress_next_token(s, utils::is_whitespace)?,
                Some(Spaces(input.len() as u8))
            );
            assert_eq!(next_token(s)?, None);
            Ok(())
        }

        #[test]
        fn next_token_valid() -> Result<(), TokenizeError> {
            let input = r#"
                1 1.0 1.1231231231 1e0 1e10
                null null
                "test \n\t\r\b\f\\\"" "test \t" "test \"321" "test \u0000"
                { "a": 123 } [1,2,"a",{"a":321}]
            321"#;

            use Token::*;

            let mut expected = vec![
                Whitespace('\n'),
                Spaces(16),
                Number("1".into()),
                Spaces(1),
                Number("1.0".into()),
                Spaces(1),
                Number("1.1231231231".into()),
                Spaces(1),
                Number("1e0".into()),
                Spaces(1),
                Number("1e10".into()),
                Whitespace('\n'),
                Spaces(16),
                Null,
                Spaces(1),
                Null,
                Whitespace('\n'),
                Spaces(16),
                String("\"test \\n\\t\\r\\b\\f\\\\\\\"\"".into()),
                Spaces(1),
                String("\"test \\t\"".into()),
                Spaces(1),
                String("\"test \\\"321\"".into()),
                Spaces(1),
                String("\"test \\u0000\"".into()),
                Whitespace('\n'),
                Spaces(16),
                ObjectOpen,
                Spaces(1),
                String("\"a\"".into()),
                Colon,
                Spaces(1),
                Number("123".into()),
                Spaces(1),
                ObjectClose,
                Spaces(1),
                ArrayOpen,
                Number("1".into()),
                Comma,
                Number("2".into()),
                Comma,
                String("\"a\"".into()),
                Comma,
                ObjectOpen,
                String("\"a\"".into()),
                Colon,
                Number("321".into()),
                ObjectClose,
                ArrayClose,
                Whitespace('\n'),
                Spaces(12),
                Number("321".into()),
            ];

            expected.reverse();

            let s = &mut Section::new(input);

            while let Some(token) = next_token(s)? {
                assert_eq!(expected.pop().unwrap(), token);
                println!("{:?},", token);
                if let Number(s) = token {
                    s.parse::<f64>().unwrap();
                }
            }

            Ok(())
        }

    }
}

struct Location {
    line: usize,
    column: usize,
}

impl Location {
    fn new() -> Self {
        Location { line: 0, column: 0 }
    }

    fn advance_char(&mut self) {
        self.column += 1;
    }

    fn advance(&mut self, n: usize) {
        self.column += n;
    }

    fn advance_line(&mut self) {
        self.line += 1;
        self.column = 0;
    }

    fn advance_token(&mut self, token: &Token<'_>) {
        use Token::*;

        match token {
            String(s) | Number(s) => self.advance(s.len()),
            Spaces(n) => self.advance(*n as usize),
            Whitespace('\n') => self.advance_line(),
            Null | &Whitespace('\t') => self.advance(4),
            _ => self.advance_char(),
        }
    }
}

pub use utils::{compress_next_token, next_token};

#[cfg(test)]
mod tests {
    use super::*;
    use utils::*;

    // #[test]
    // fn example_validate_and_compress() {
    //     let input = r#"
    //     { "a" :
    //     1 }
    //     [  1, 4e0, 5 ]
    //     "#;

    //     let s = &mut Section::new(input);

    //     while let Some(token) =
    // }

    // #[test]
    // fn example_amend_number_token() -> Result<(), TokenizeError> {
    //     let mut input = "123".to_string();
    //     let s = &mut Section::new(&input);
    //     let token = next_token(s)?.unwrap();

    //     input += "e10";

    //     token_length(&token);

    //     Ok(())
    // }

    #[test]
    fn example_token_iterator() -> Result<(), TokenizeError> {
        use Token::*;
        let input = "[1,2,3, \"321\"]";
        let s = &mut Section::new(&input);
        assert_eq!(
            vec![
                ArrayOpen,
                Number("1".into()),
                Comma,
                Number("2".into()),
                Comma,
                Number("3".into()),
                Comma,
                Spaces(1),
                String("\"321\"".into()),
                ArrayClose
            ],
            std::iter::from_fn(|| next_token(s).transpose())
                .collect::<Result<Vec<_>, TokenizeError>>()?
        );
        Ok(())
    }

    // #[test]
    // fn parse_test() -> Result<(), TokenizeError> {
    //     use Token::*;

    //     let input = "[1,2,3, \"321\"]";
    //     let s = &mut Section::new(&input);
    //     while let Some(token) = next_token(s)? {
    //         match token {
    //             ArrayOpen => {}
    //         }
    //     }
    //     assert_eq!(
    //         vec![
    //             ArrayOpen,
    //             Number("1"),
    //             Comma,
    //             Number("2"),
    //             Comma,
    //             Number("3"),
    //             Comma,
    //             Spaces(1),
    //             String("\"321\""),
    //             ArrayClose
    //         ],
    //         std::iter::from_fn(|| next_token(s).transpose())
    //             .collect::<Result<Vec<_>, TokenizeError>>()?
    //     );
    //     Ok(())
    // }

}
