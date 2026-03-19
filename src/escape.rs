use crate::types::EncodingChars;

/// Decode HL7v2 escape sequences in a field value.
///
/// Standard HL7 escape sequences:
/// - `\F\` → field separator (|)
/// - `\S\` → component separator (^)
/// - `\T\` → sub-component separator (&)
/// - `\R\` → repetition separator (~)
/// - `\E\` → escape character (\)
/// - `\X...\` → hex data (preserved as-is for now)
/// - `\.br\` → line break (→ \n)
pub fn decode_escapes(value: &str, enc: &EncodingChars) -> String {
    let esc = enc.escape_char;

    // Fast path: no escape character present
    if !value.contains(esc) {
        return value.to_string();
    }

    let mut result = String::with_capacity(value.len());
    let chars: Vec<char> = value.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        if chars[i] == esc && i + 2 < len {
            // Look for the closing escape character
            if let Some(close) = chars[i + 1..].iter().position(|&c| c == esc) {
                let seq: String = chars[i + 1..i + 1 + close].iter().collect();
                match seq.as_str() {
                    "F" => result.push(enc.field_sep),
                    "S" => result.push(enc.component_sep),
                    "T" => result.push(enc.subcomponent_sep),
                    "R" => result.push(enc.repetition_sep),
                    "E" => result.push(enc.escape_char),
                    ".br" => result.push('\n'),
                    s if s.starts_with('X') => {
                        // Hex escape — decode hex bytes
                        let hex_str = &s[1..];
                        let mut j = 0;
                        let hex_chars: Vec<char> = hex_str.chars().collect();
                        while j + 1 < hex_chars.len() {
                            let byte_str: String =
                                hex_chars[j..j + 2].iter().collect();
                            if let Ok(byte) = u8::from_str_radix(&byte_str, 16) {
                                result.push(byte as char);
                            }
                            j += 2;
                        }
                    }
                    _ => {
                        // Unknown escape — preserve as-is
                        result.push(esc);
                        result.push_str(&seq);
                        result.push(esc);
                    }
                }
                i += close + 2; // skip past closing escape char
            } else {
                // No closing escape char found — preserve literally
                result.push(chars[i]);
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_enc() -> EncodingChars {
        EncodingChars::default()
    }

    #[test]
    fn test_no_escapes() {
        assert_eq!(decode_escapes("hello world", &default_enc()), "hello world");
    }

    #[test]
    fn test_field_sep_escape() {
        assert_eq!(decode_escapes("before\\F\\after", &default_enc()), "before|after");
    }

    #[test]
    fn test_component_sep_escape() {
        assert_eq!(decode_escapes("a\\S\\b", &default_enc()), "a^b");
    }

    #[test]
    fn test_subcomponent_sep_escape() {
        assert_eq!(decode_escapes("a\\T\\b", &default_enc()), "a&b");
    }

    #[test]
    fn test_repetition_sep_escape() {
        assert_eq!(decode_escapes("a\\R\\b", &default_enc()), "a~b");
    }

    #[test]
    fn test_escape_char_escape() {
        assert_eq!(decode_escapes("a\\E\\b", &default_enc()), "a\\b");
    }

    #[test]
    fn test_line_break_escape() {
        assert_eq!(decode_escapes("line1\\.br\\line2", &default_enc()), "line1\nline2");
    }

    #[test]
    fn test_multiple_escapes() {
        assert_eq!(
            decode_escapes("a\\F\\b\\S\\c", &default_enc()),
            "a|b^c"
        );
    }

    #[test]
    fn test_hex_escape() {
        // \X0D\ = carriage return
        assert_eq!(decode_escapes("a\\X0D\\b", &default_enc()), "a\rb");
    }
}
