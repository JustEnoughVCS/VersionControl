use std::cmp::Ordering;

/// Compare two `Vec<String>` according to a specific ordering:
/// 1. ASCII symbols (excluding letters and digits)
/// 2. Letters (Aa-Zz, case‑sensitive, uppercase before lowercase)
/// 3. Digits (0‑9) - compared numerically
/// 4. All other Unicode characters (in their natural order)
///
/// The comparison is lexicographic: the first differing element determines the order.
pub fn compare_vec_string(a: &[String], b: &[String]) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    for (left, right) in a.iter().zip(b.iter()) {
        match compare_string(left, right) {
            Ordering::Equal => continue,
            ord => return ord,
        }
    }
    // If all compared elements are equal, the shorter vector comes first.
    a.len().cmp(&b.len())
}

/// Compare two individual strings with the same ordering rules.
pub fn compare_string(a: &str, b: &str) -> std::cmp::Ordering {
    let mut a_chars = a.chars().peekable();
    let mut b_chars = b.chars().peekable();

    loop {
        match (a_chars.peek(), b_chars.peek()) {
            (Some(&ca), Some(&cb)) => {
                if ca.is_ascii_digit() && cb.is_ascii_digit() {
                    // Parse both numbers and compare numerically
                    let a_num = parse_number(&mut a_chars);
                    let b_num = parse_number(&mut b_chars);
                    match a_num.cmp(&b_num) {
                        Ordering::Equal => continue,
                        ord => return ord,
                    }
                } else {
                    // Non-digit comparison
                    let ord = compare_char(ca, cb);
                    if ord != Ordering::Equal {
                        return ord;
                    }
                    a_chars.next();
                    b_chars.next();
                }
            }
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (None, None) => return Ordering::Equal,
        }
    }
}

/// Parse a number from the character iterator.
/// Consumes consecutive ASCII digits and returns the parsed u64.
/// Assumes the first character is already verified to be a digit.
pub fn parse_number<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>) -> u64 {
    let mut num = 0;
    while let Some(&c) = chars.peek() {
        if c.is_ascii_digit() {
            num = num * 10 + (c as u64 - '0' as u64);
            chars.next();
        } else {
            break;
        }
    }
    num
}

/// Compare two characters according to the ordering:
/// 1. ASCII symbols (non‑letter, non‑digit)
/// 2. Letters (A‑Z then a‑z)
/// 3. Digits (0‑9) - note: digits are handled specially in compare_string
/// 4. Other Unicode
pub fn compare_char(a: char, b: char) -> std::cmp::Ordering {
    let group_a = char_group(a);
    let group_b = char_group(b);

    if group_a != group_b {
        return group_a.cmp(&group_b);
    }

    // Same group: compare within the group.
    match group_a {
        CharGroup::AsciiSymbol => a.cmp(&b), // ASCII symbols in natural order
        CharGroup::Letter => {
            // Uppercase letters (A‑Z) come before lowercase (a‑z)
            if a.is_ascii_uppercase() && b.is_ascii_lowercase() {
                Ordering::Less
            } else if a.is_ascii_lowercase() && b.is_ascii_uppercase() {
                Ordering::Greater
            } else {
                a.cmp(&b)
            }
        }
        CharGroup::Digit => {
            // Digits should be compared numerically, but this is only reached
            // when comparing single digits (not part of a longer number).
            a.cmp(&b)
        }
        CharGroup::Other => a.cmp(&b), // Other Unicode (natural order)
    }
}

/// Classification of a character for ordering.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CharGroup {
    AsciiSymbol = 0, // !"#$%&'()*+,-./:;<=>?@[\]^_`{|}~
    Letter = 1,      // A‑Z, a‑z
    Digit = 2,       // 0‑9
    Other = 3,       // Other
}

pub fn char_group(c: char) -> CharGroup {
    if c.is_ascii_punctuation() {
        CharGroup::AsciiSymbol
    } else if c.is_ascii_alphabetic() {
        CharGroup::Letter
    } else if c.is_ascii_digit() {
        CharGroup::Digit
    } else {
        CharGroup::Other
    }
}

#[test]
pub fn test_compare_char_groups() {
    assert!(compare_string("!", "A") == Ordering::Less);
    assert!(compare_string("A", "a") == Ordering::Less);
    assert!(compare_string("a", "0") == Ordering::Less);
    assert!(compare_string("9", "你") == Ordering::Less);
}

#[test]
pub fn test_numeric_ordering() {
    // Test numeric ordering
    assert!(compare_string("0", "1") == Ordering::Less);
    assert!(compare_string("1", "0") == Ordering::Greater);
    assert!(compare_string("9", "10") == Ordering::Less);
    assert!(compare_string("10", "9") == Ordering::Greater);
    assert!(compare_string("99", "100") == Ordering::Less);
    assert!(compare_string("100", "99") == Ordering::Greater);
    assert!(compare_string("001", "1") == Ordering::Equal); // "001" numerically equals "1"
    assert!(compare_string("01", "1") == Ordering::Equal); // "01" numerically equals "1"

    // Test mixed strings
    assert!(compare_string("Frame-9", "Frame-10") == Ordering::Less);
    assert!(compare_string("Frame-10", "Frame-9") == Ordering::Greater);
    assert!(compare_string("Frame-99", "Frame-100") == Ordering::Less);
    assert!(compare_string("Frame-100", "Frame-99") == Ordering::Greater);

    // Test that numbers are compared as whole numbers, not digit-by-digit
    assert!(compare_string("123", "23") == Ordering::Greater); // 123 > 23
    assert!(compare_string("23", "123") == Ordering::Less); // 23 < 123
}
