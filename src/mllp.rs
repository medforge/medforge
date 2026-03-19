/// MLLP (Minimal Lower Layer Protocol) framing utilities.
///
/// HL7 messages transmitted over TCP are wrapped in MLLP framing:
/// - Start: `\x0b` (vertical tab / VT)
/// - End:   `\x1c\r` (file separator + carriage return)
///
/// This module detects and strips MLLP framing.

const MLLP_START: u8 = 0x0B; // VT (vertical tab)
const MLLP_END: u8 = 0x1C; // FS (file separator)

/// Check if the raw bytes are wrapped in MLLP framing.
pub fn is_mllp_framed(data: &[u8]) -> bool {
    if data.len() < 3 {
        return false;
    }
    data[0] == MLLP_START
        && (data[data.len() - 1] == b'\r' && data[data.len() - 2] == MLLP_END
            || data[data.len() - 1] == MLLP_END)
}

/// Strip MLLP framing from raw bytes, returning the inner message.
///
/// If the data is not MLLP-framed, returns it unchanged.
pub fn strip_mllp(data: &str) -> &str {
    let bytes = data.as_bytes();
    if !is_mllp_framed(bytes) {
        return data;
    }

    let start = 1; // skip \x0b
    let end = if bytes[bytes.len() - 1] == b'\r' && bytes[bytes.len() - 2] == MLLP_END {
        bytes.len() - 2
    } else {
        bytes.len() - 1
    };

    &data[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_mllp() {
        let framed = format!("\x0bMSH|...\x1c\r");
        assert!(is_mllp_framed(framed.as_bytes()));
    }

    #[test]
    fn test_detect_mllp_no_trailing_cr() {
        let framed = format!("\x0bMSH|...\x1c");
        assert!(is_mllp_framed(framed.as_bytes()));
    }

    #[test]
    fn test_not_mllp() {
        assert!(!is_mllp_framed(b"MSH|..."));
    }

    #[test]
    fn test_strip_mllp() {
        let framed = "\x0bMSH|^~\\&|SENDER\x1c\r";
        assert_eq!(strip_mllp(framed), "MSH|^~\\&|SENDER");
    }

    #[test]
    fn test_strip_mllp_no_cr() {
        let framed = "\x0bMSH|^~\\&|SENDER\x1c";
        assert_eq!(strip_mllp(framed), "MSH|^~\\&|SENDER");
    }

    #[test]
    fn test_strip_no_mllp_passthrough() {
        let raw = "MSH|^~\\&|SENDER";
        assert_eq!(strip_mllp(raw), raw);
    }

    #[test]
    fn test_short_data() {
        assert!(!is_mllp_framed(b"AB"));
        assert_eq!(strip_mllp("AB"), "AB");
    }
}
