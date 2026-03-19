use crate::escape::decode_escapes;
use crate::mllp::strip_mllp;
use crate::types::{Component, EncodingChars, Field, Message, Segment};

/// Parse a raw HL7v2 message string into a [`Message`].
///
/// Handles MLLP-framed input automatically. Supports `\r`, `\n`, and `\r\n`
/// segment delimiters.
pub fn parse_message(raw: &str) -> Result<Message, String> {
    // Strip MLLP framing if present
    let raw = strip_mllp(raw);

    if raw.is_empty() {
        return Err("Empty message".to_string());
    }

    // Normalize line endings: split on \r, \n, or \r\n
    let segment_strs: Vec<&str> = split_segments(raw);

    if segment_strs.is_empty() {
        return Err("No segments found".to_string());
    }

    // Extract encoding characters from MSH
    let first = segment_strs[0];
    if !first.starts_with("MSH") {
        return Err(format!(
            "Message must start with MSH segment, got: '{}'",
            &first[..first.len().min(10)]
        ));
    }

    let enc = extract_encoding_chars(first)?;

    // Parse all segments
    let mut segments = Vec::with_capacity(segment_strs.len());
    for seg_str in &segment_strs {
        if seg_str.is_empty() {
            continue;
        }
        let segment = parse_segment(seg_str, &enc)?;
        segments.push(segment);
    }

    Ok(Message {
        raw: raw.to_string(),
        segments,
    })
}

/// Extract encoding characters from the MSH segment.
///
/// MSH layout: `MSH|^~\&|...`
///   - Position 3:   field separator (|)
///   - Position 4:   component separator (^)
///   - Position 5:   repetition separator (~)
///   - Position 6:   escape character (\)
///   - Position 7:   sub-component separator (&)
fn extract_encoding_chars(msh: &str) -> Result<EncodingChars, String> {
    let bytes = msh.as_bytes();

    if bytes.len() < 8 {
        return Err("MSH segment too short to extract encoding characters".to_string());
    }

    Ok(EncodingChars {
        field_sep: bytes[3] as char,
        component_sep: bytes[4] as char,
        repetition_sep: bytes[5] as char,
        escape_char: bytes[6] as char,
        subcomponent_sep: bytes[7] as char,
    })
}

/// Split raw message into segment strings, handling \r, \n, and \r\n.
fn split_segments(raw: &str) -> Vec<&str> {
    raw.split(|c| c == '\r' || c == '\n')
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a single segment string into a [`Segment`].
fn parse_segment(seg_str: &str, enc: &EncodingChars) -> Result<Segment, String> {
    let is_msh = seg_str.starts_with("MSH");

    let field_sep = enc.field_sep;
    let parts: Vec<&str> = seg_str.splitn(2, field_sep).collect();

    let name = parts[0].to_string();

    if parts.len() < 2 {
        // Segment with just a name, no fields
        return Ok(Segment {
            name,
            fields: vec![],
        });
    }

    let fields_str = parts[1];

    // For MSH, field 1 is the field separator itself, and field 2 is the
    // encoding characters — we handle these specially.
    let mut fields: Vec<Field>;

    if is_msh {
        // MSH-1 = field separator
        let sep_field = Field {
            value: field_sep.to_string(),
            components: vec![Component {
                value: field_sep.to_string(),
                sub_components: vec![field_sep.to_string()],
            }],
            repetitions: vec![],
        };

        // Split remaining by field separator
        let raw_fields: Vec<&str> = fields_str.split(field_sep).collect();

        // MSH-2 = encoding characters (first element, before the first |)
        fields = Vec::with_capacity(raw_fields.len() + 1);
        fields.push(sep_field);

        for raw_field in &raw_fields {
            fields.push(parse_field(raw_field, enc));
        }
    } else {
        let raw_fields: Vec<&str> = fields_str.split(field_sep).collect();
        fields = Vec::with_capacity(raw_fields.len());
        for raw_field in &raw_fields {
            fields.push(parse_field(raw_field, enc));
        }
    }

    Ok(Segment { name, fields })
}

/// Parse a single field string, handling repetitions and components.
fn parse_field(raw: &str, enc: &EncodingChars) -> Field {
    // Check for repetitions
    let rep_parts: Vec<&str> = raw.split(enc.repetition_sep).collect();

    let repetitions = if rep_parts.len() > 1 {
        rep_parts
            .iter()
            .map(|r| parse_single_field(r, enc))
            .collect()
    } else {
        vec![]
    };

    // Parse the first (or only) value
    let mut field = parse_single_field(rep_parts[0], enc);
    field.repetitions = repetitions;

    // The top-level value is the full raw string (including repetition separators)
    field.value = decode_escapes(raw, enc);
    field
}

/// Parse a single field value (no repetition handling) into components.
fn parse_single_field(raw: &str, enc: &EncodingChars) -> Field {
    let comp_parts: Vec<&str> = raw.split(enc.component_sep).collect();

    let components: Vec<Component> = comp_parts
        .iter()
        .map(|c| parse_component(c, enc))
        .collect();

    Field {
        value: decode_escapes(raw, enc),
        components,
        repetitions: vec![],
    }
}

/// Parse a component string, extracting sub-components.
fn parse_component(raw: &str, enc: &EncodingChars) -> Component {
    let sub_parts: Vec<String> = raw
        .split(enc.subcomponent_sep)
        .map(|s| decode_escapes(s, enc))
        .collect();

    Component {
        value: decode_escapes(raw, enc),
        sub_components: sub_parts,
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_ADT: &str =
        "MSH|^~\\&|SENDER|FAC|RECV|FAC|20230101120000||ADT^A01|12345|P|2.5\rPID|1||MRN123^^^MRN||DOE^JOHN^M||19800101|M\rPV1|1|I|4EAST^401^1";

    #[test]
    fn test_parse_basic_message() {
        let msg = parse_message(SAMPLE_ADT).unwrap();
        assert_eq!(msg.segments.len(), 3);
        assert_eq!(msg.segments[0].name, "MSH");
        assert_eq!(msg.segments[1].name, "PID");
        assert_eq!(msg.segments[2].name, "PV1");
    }

    #[test]
    fn test_msh_fields() {
        let msg = parse_message(SAMPLE_ADT).unwrap();
        let msh = &msg.segments[0];

        // MSH-1 = field separator
        assert_eq!(msh.fields[0].value, "|");
        // MSH-2 = encoding characters
        assert_eq!(msh.fields[1].value, "^~\\&");
        // MSH-3 = sending application
        assert_eq!(msh.fields[2].value, "SENDER");
        // MSH-9 = message type
        assert_eq!(msh.fields[8].components[0].value, "ADT");
        assert_eq!(msh.fields[8].components[1].value, "A01");
        // MSH-10 = control ID
        assert_eq!(msh.fields[9].value, "12345");
        // MSH-12 = version
        assert_eq!(msh.fields[11].value, "2.5");
    }

    #[test]
    fn test_pid_patient_name() {
        let msg = parse_message(SAMPLE_ADT).unwrap();
        let pid = &msg.segments[1];

        // PID-5 = patient name (field index 4 in 0-based, but we store 1-indexed)
        let name_field = &pid.fields[4]; // PID-5
        assert_eq!(name_field.components[0].value, "DOE");
        assert_eq!(name_field.components[1].value, "JOHN");
        assert_eq!(name_field.components[2].value, "M");
    }

    #[test]
    fn test_encoding_chars_extraction() {
        let enc = extract_encoding_chars("MSH|^~\\&|rest").unwrap();
        assert_eq!(enc.field_sep, '|');
        assert_eq!(enc.component_sep, '^');
        assert_eq!(enc.repetition_sep, '~');
        assert_eq!(enc.escape_char, '\\');
        assert_eq!(enc.subcomponent_sep, '&');
    }

    #[test]
    fn test_custom_encoding_chars() {
        let enc = extract_encoding_chars("MSH#^~\\&#rest").unwrap();
        assert_eq!(enc.field_sep, '#');
    }

    #[test]
    fn test_newline_delimiter() {
        let raw = "MSH|^~\\&|SENDER|FAC|RECV|FAC|20230101||ADT^A01|123|P|2.5\nPID|1||MRN|||DOE^JOHN";
        let msg = parse_message(raw).unwrap();
        assert_eq!(msg.segments.len(), 2);
    }

    #[test]
    fn test_crlf_delimiter() {
        let raw = "MSH|^~\\&|SENDER|FAC|RECV|FAC|20230101||ADT^A01|123|P|2.5\r\nPID|1||MRN|||DOE^JOHN";
        let msg = parse_message(raw).unwrap();
        assert_eq!(msg.segments.len(), 2);
    }

    #[test]
    fn test_empty_fields() {
        let raw = "MSH|^~\\&|||||20230101||ADT^A01|123|P|2.5\rPID|1||MRN|||||||";
        let msg = parse_message(raw).unwrap();
        let pid = &msg.segments[1];
        // Trailing empty fields should be preserved
        assert!(pid.fields.len() >= 9);
    }

    #[test]
    fn test_repetition() {
        let raw =
            "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rPID|1||MRN1^^^MRN~DEA1^^^DEA";
        let msg = parse_message(raw).unwrap();
        let pid = &msg.segments[1];
        let id_field = &pid.fields[2]; // PID-3
        assert_eq!(id_field.repetitions.len(), 2);
        assert_eq!(id_field.repetitions[0].components[0].value, "MRN1");
        assert_eq!(id_field.repetitions[1].components[0].value, "DEA1");
    }

    #[test]
    fn test_subcomponents() {
        let raw = "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rPID|1||ID&CHECK^^^AUTH";
        let msg = parse_message(raw).unwrap();
        let pid = &msg.segments[1];
        let id_field = &pid.fields[2]; // PID-3
        let first_comp = &id_field.components[0];
        assert_eq!(first_comp.sub_components.len(), 2);
        assert_eq!(first_comp.sub_components[0], "ID");
        assert_eq!(first_comp.sub_components[1], "CHECK");
    }

    #[test]
    fn test_mllp_framed_message() {
        let raw = format!(
            "\x0bMSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rPID|1||MRN\x1c\r"
        );
        let msg = parse_message(&raw).unwrap();
        assert_eq!(msg.segments.len(), 2);
    }

    #[test]
    fn test_error_no_msh() {
        let result = parse_message("PID|1||MRN");
        assert!(result.is_err());
    }

    #[test]
    fn test_error_empty() {
        let result = parse_message("");
        assert!(result.is_err());
    }

    #[test]
    fn test_escape_sequences_in_fields() {
        let raw = "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rOBX|1|ST|CODE||value\\F\\with\\S\\special";
        let msg = parse_message(raw).unwrap();
        let obx = &msg.segments[1];
        let value_field = &obx.fields[4]; // OBX-5
        assert_eq!(value_field.value, "value|with^special");
    }

    #[test]
    fn test_performance_large_message() {
        // Build a message with 10k segments
        let mut raw = String::from("MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5");
        for i in 0..10_000 {
            raw.push('\r');
            raw.push_str(&format!(
                "OBX|{}|NM|CODE-{}||{}|unit|0-100||||F",
                i, i, i * 7
            ));
        }

        let start = std::time::Instant::now();
        let msg = parse_message(&raw).unwrap();
        let elapsed = start.elapsed();

        assert_eq!(msg.segments.len(), 10_001); // MSH + 10k OBX
        assert!(
            elapsed.as_millis() < 500,
            "Parsing took {}ms, expected < 500ms",
            elapsed.as_millis()
        );
    }
}
