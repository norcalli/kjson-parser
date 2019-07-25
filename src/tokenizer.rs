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

use crate::section::{ISection, Section};

use std::borrow::{Cow, ToOwned};
use std::io;

#[derive(Debug, PartialEq, Eq)]
pub enum TokenizeError {
    UnexpectedCharacter(char),
    UnexpectedEndOfInput,
    // TODO make this &str?
    InvalidStringUnicodeEscape(String),
    InvalidStringEscape(char),
    InvalidStringCharacter(char),
}

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
    String(Cow<'a, str>),
    Number(Cow<'a, str>),
    ArrayOpen,
    ArrayClose,
    // Bool(bool),
    True,
    False,
    Null,
}

use crate::JsonType;

impl Token<'_> {
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

    pub fn value_type(&self) -> Option<crate::JsonType> {
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
    pub fn as_string(&self) -> Option<&str> {
        match self {
            Token::String(c) => Some(c),
            _ => None,
        }
    }

    #[inline]
    /// Returns true if this can be matched completely, but that
    /// match could be a false positive. Meaning that if you actually
    /// had more data incoming, you should restart the token matching
    /// from the beginning of the last token.
    pub fn partial_false_positive(&self) -> bool {
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
            Token::String(x) | Token::Number(x) => write!(writer, "{}", x),
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

// enum Whitespace {
//     Newline,
//     Tab,
//     Space,
// }

pub mod utils {
    use super::*;

    // TODO double check
    #[inline]
    pub fn is_whitespace(c: char) -> bool {
        c == ' ' || c == '\t' || c == '\n'
    }

    #[inline]
    fn section_digits(s: &mut Section<'_>) -> usize {
        let start = s.offset();
        while let Some('0'..='9') = s.peek() {
            s.next();
        }
        s.offset() - start
    }

    #[inline]
    // TODO convert to (&u8) and use lookup table.
    pub fn is_hexdigit(c: char) -> bool {
        if let '0'..='9' | 'A'..='F' | 'a'..='f' = c {
            true
        } else {
            false
        }
    }

    #[inline]
    fn section_hexdigits(s: &mut Section<'_>) -> usize {
        let start = s.offset();
        while let Some('0'..='9') | Some('A'..='F') | Some('a'..='f') = s.peek() {
            s.next();
        }
        s.offset() - start
    }

    #[inline]
    fn section_nonzero(s: &mut Section<'_>) -> usize {
        let start = s.offset();
        while let Some('1'..='9') = s.peek() {
            s.next();
        }
        s.offset() - start
    }

    #[inline]
    pub fn invalid_input_err(s: Option<&char>) -> TokenizeError {
        match s {
            Some(c) => TokenizeError::UnexpectedCharacter(*c),
            None => TokenizeError::UnexpectedEndOfInput,
        }
    }

    #[inline]
    fn section_number_exponent(s: &mut Section<'_>) -> Result<usize, TokenizeError> {
        let n = s.offset();
        match s.peek() {
            Some('e') | Some('E') => {
                s.next();
                match s.peek() {
                    Some('-') | Some('+') => {
                        s.next();
                        if section_digits(s) == 0 {
                            return Err(invalid_input_err(s.peek()));
                        }
                        Ok(s.offset() - n)
                    }
                    Some('0'..='9') => {
                        s.next();
                        section_digits(s);
                        return Ok(s.offset() - n);
                    }
                    e => Err(invalid_input_err(e)),
                }
            }
            e => Err(invalid_input_err(e)),
        }
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
    pub fn section_number(s: &mut Section<'_>) -> Result<(), TokenizeError> {
        match s.peek() {
            Some('-') => {
                s.next();
                return section_number(s);
            }
            // if it begins with 0, it must be fractional
            Some('0') => {
                s.next();
            }
            Some('1'..='9') => {
                section_digits(s);
            }
            e => return Err(invalid_input_err(e)),
        }
        if let Some('.') = s.peek() {
            s.next();
            if section_digits(s) == 0 {
                return Err(invalid_input_err(s.peek()));
            }
        }
        if let Some('e') | Some('E') = s.peek() {
            section_number_exponent(s)?;
        }
        Ok(())
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
    pub fn section_string(s: &mut Section<'_>) -> Result<(), TokenizeError> {
        match s.peek() {
            Some('"') => {
                s.next();
            }
            e => return Err(invalid_input_err(e)),
        }
        while let Some(c) = s.peek() {
            match c {
                // Escape sequences
                '\\' => {
                    s.next();
                    match s.peek() {
                        Some(c) => match c {
                            'n' | 't' | 'b' | 'f' | 'r' | '"' | '\\' | '/' => {
                                s.next();
                            }
                            'u' => {
                                s.next();
                                let offset = s.offset();
                                let buf = s.after();
                                if !(s.check_next_pattern(is_hexdigit)
                                    && s.check_next_pattern(is_hexdigit)
                                    && s.check_next_pattern(is_hexdigit)
                                    && s.check_next_pattern(is_hexdigit))
                                {
                                    return Err(TokenizeError::InvalidStringUnicodeEscape(
                                        buf[..s.offset() - offset].to_owned(),
                                    ));
                                }
                            }
                            e => return Err(TokenizeError::InvalidStringEscape(*e)),
                        },
                        e => return Err(invalid_input_err(e)),
                    };
                }
                '"' => {
                    s.next();
                    return Ok(());
                }
                '\x20'..='\x21' | '\x23'..='\x5B' | '\x5D'..='\u{10FFFF}' => {
                    s.next();
                }
                e => return Err(TokenizeError::InvalidStringCharacter(*e)),
            }
        }
        Err(invalid_input_err(None))
    }

    pub fn peek_next_token<'a>(s: &mut Section<'a>) -> Result<Option<Token<'a>>, TokenizeError> {
        next_token(&mut s.clone())
    }

    /* TODO benchmark against this/check the ASM output
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
    
    #[inline]
    pub fn compress_next_token<'a, F: Fn(char) -> bool>(
        s: &mut Section<'a>,
        compressed_whitespace: F,
    ) -> Result<Option<Token<'a>>, TokenizeError> {
        Ok(Some(match s.peek() {
            Some(&c) => match c {
                // ' ' | '\n' | '\t' if compressed_whitespace(c) => {
                c if compressed_whitespace(c) => {
                    let mut n = 1;
                    s.next();
                    while let Some(c) = s.peek() {
                        if !is_whitespace(*c) {
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
                ' ' | '\n' | '\t' => {
                    s.next();
                    Token::Whitespace(c)
                }
                // Valid number starters
                '-' | '0'..='9' => {
                    let offset = s.offset();
                    let buf = s.after();
                    utils::section_number(s)?;
                    let n = s.offset() - offset;
                    Token::Number(buf[..n].into())
                }
                '"' => {
                    let offset = s.offset();
                    let buf = s.after();
                    utils::section_string(s)?;
                    let n = s.offset() - offset;
                    Token::String(buf[..n].into())
                }
                'n' => {
                    s.next();
                    // TODO peek then next instead?
                    // Does consuming prematurely matter?
                    if s.check_next('u') && s.check_next('l') && s.check_next('l') {
                        Token::Null
                    } else {
                        return Err(utils::invalid_input_err(s.peek()));
                    }
                }
                't' => {
                    s.next();
                    // TODO peek then next instead?
                    // Does consuming prematurely matter?
                    if s.check_next('r') && s.check_next('u') && s.check_next('e') {
                        Token::True
                    } else {
                        return Err(utils::invalid_input_err(s.peek()));
                    }
                }
                'f' => {
                    s.next();
                    // TODO peek then next instead?
                    // Does consuming prematurely matter?
                    // 2019-07-24: Turns out, yes. This means that I can't use it for certain
                    // applications like extracting json from a stream of maybe bytes.
                    if s.check_next('a')
                        && s.check_next('l')
                        && s.check_next('s')
                        && s.check_next('e')
                    {
                        Token::False
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

    #[inline]
    pub fn next_token<'a>(s: &mut Section<'a>) -> Result<Option<Token<'a>>, TokenizeError> {
        compress_next_token(s, |c| c == ' ')
    }

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
