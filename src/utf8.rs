use crate::section::PeekSeek;

const CONT_MASK: u8 = 0b0011_1111;
const TAG_CONT_U8: u8 = 0b1000_0000;
const NONASCII_MASK: usize = 0x80808080_80808080u64 as usize;

#[rustfmt::skip]
static UTF8_CHAR_WIDTH: [u8; 256] = [
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,// 0x1F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,// 0x3F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,// 0x5F
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,
1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,// 0x7F
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,// 0x9F
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,
0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,// 0xBF
0,0,2,2,2,2,2,2,2,2,2,2,2,2,2,2,
2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,// 0xDF
3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,3,// 0xEF
4,4,4,4,4,0,0,0,0,0,0,0,0,0,0,0,// 0xFF
];

/// Returns the initial codepoint accumulator for the first byte.
/// The first byte is special, only want bottom 5 bits for width 2, 4 bits
/// for width 3, and 3 bits for width 4.
#[inline]
fn utf8_first_byte(byte: u8, width: u32) -> u32 {
    (byte & (0x7F >> width)) as u32
}

/// Returns the value of `ch` updated with continuation byte `byte`.
#[inline]
fn utf8_acc_cont_byte(ch: u32, byte: u8) -> u32 {
    (ch << 6) | (byte & CONT_MASK) as u32
}

/// Checks whether the byte is a UTF-8 continuation byte (i.e., starts with the
/// bits `10`).
#[inline]
fn utf8_is_cont_byte(byte: u8) -> bool {
    (byte & !CONT_MASK) == TAG_CONT_U8
}

// #[inline]
// fn unwrap_or_0(opt: Option<&u8>) -> u8 {
//     match opt {
//         Some(&byte) => byte,
//         None => 0,
//     }
// }

#[inline]
fn unwrap_or_0(opt: Option<u8>) -> u8 {
    match opt {
        Some(byte) => byte,
        None => 0,
    }
}

/// Reads the next code point out of a byte iterator (assuming a
/// UTF-8-like encoding).
#[inline]
// pub fn next_code_point<'a, I: Iterator<Item = &'a u8>>(bytes: &mut I) -> Option<u32> {
// pub fn next_code_point<I: PeekSeek<Item = u8>>(bytes: &mut I) -> Option<u32> {
pub fn next_code_point(bytes: &[u8]) -> Option<u32> {
    // Decode UTF-8
    // let x = *bytes.next()?;
    let x = bytes[0];
    if x < 128 {
        return Some(x as u32);
    }

    Some(definitely_next_codepoint(x, &bytes[1..]))

    //     // Multibyte case follows
    //     // Decode from a byte combination out of: [[[x y] z] w]
    //     // NOTE: Performance is sensitive to the exact formulation here
    //     let init = utf8_first_byte(x, 2);
    //     let y = unwrap_or_0(bytes.next());
    //     let mut ch = utf8_acc_cont_byte(init, y);
    //     if x >= 0xE0 {
    //         // [[x y z] w] case
    //         // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
    //         let z = unwrap_or_0(bytes.next());
    //         let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
    //         ch = init << 12 | y_z;
    //         if x >= 0xF0 {
    //             // [x y z w] case
    //             // use only the lower 3 bits of `init`
    //             let w = unwrap_or_0(bytes.next());
    //             ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
    //         }
    //     }

    //     Some(ch)
}

pub fn utf8_char_width(x: u8) -> u8 {
    UTF8_CHAR_WIDTH[x as usize]
}

// trait HasNext {
//     fn next(&mut self) -> Option<u8>;
// }

// impl<T: PeekSeek<Item = u8>> HasNext for T {}
// impl<T: Iterator<Item = u8>> HasNext for T {}

/// Reads the next code point out of a byte iterator (assuming a
/// UTF-8-like encoding).
#[inline]
pub fn definitely_next_codepoint(x: u8, bytes: &[u8]) -> u32 {
    // Multibyte case follows
    // Decode from a byte combination out of: [[[x y] z] w]
    // NOTE: Performance is sensitive to the exact formulation here
    let init = utf8_first_byte(x, 2);
    let y = bytes[0];
    let mut ch = utf8_acc_cont_byte(init, y);
    if x >= 0xE0 {
        // [[x y z] w] case
        // 5th bit in 0xE0 .. 0xEF is always clear, so `init` is still valid
        let z = bytes[1];
        let y_z = utf8_acc_cont_byte((y & CONT_MASK) as u32, z);
        ch = init << 12 | y_z;
        if x >= 0xF0 {
            // [x y z w] case
            // use only the lower 3 bits of `init`
            let w = bytes[2];
            ch = (init & 7) << 18 | utf8_acc_cont_byte(y_z, w);
        }
    }

    ch
}

// /// Reads the last code point out of a byte iterator (assuming a
// /// UTF-8-like encoding).
// #[inline]
// fn next_code_point_reverse<'a, I>(bytes: &mut I) -> Option<u32>
//     where I: DoubleEndedIterator<Item = &'a u8>,
// {
//     // Decode UTF-8
//     let w = match *bytes.next_back()? {
//         next_byte if next_byte < 128 => return Some(next_byte as u32),
//         back_byte => back_byte,
//     };

//     // Multibyte case follows
//     // Decode from a byte combination out of: [x [y [z w]]]
//     let mut ch;
//     let z = unwrap_or_0(bytes.next_back());
//     ch = utf8_first_byte(z, 2);
//     if utf8_is_cont_byte(z) {
//         let y = unwrap_or_0(bytes.next_back());
//         ch = utf8_first_byte(y, 3);
//         if utf8_is_cont_byte(y) {
//             let x = unwrap_or_0(bytes.next_back());
//             ch = utf8_first_byte(x, 4);
//             ch = utf8_acc_cont_byte(ch, y);
//         }
//         ch = utf8_acc_cont_byte(ch, z);
//     }
//     ch = utf8_acc_cont_byte(ch, w);

//     Some(ch)
// }
