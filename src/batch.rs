//! Batch/file mode parsing for HL7v2 messages.
//!
//! Handles:
//! - FHS/BHS/BTS/FTS wrapped files containing multiple messages
//! - Simple multi-message streams separated by MSH segments

use pyo3::prelude::*;

use crate::parser::parse_message;
use crate::types::Message;

/// Parse a batch of HL7v2 messages from a raw string.
///
/// Handles:
/// - FHS/BHS/BTS/FTS wrapped files (batch headers/trailers are stripped)
/// - Multiple messages separated by `MSH` segment headers
///
/// Returns a list of parsed `Message` objects.
#[pyfunction]
#[pyo3(signature = (raw))]
pub fn parse_batch(raw: &str) -> PyResult<Vec<Message>> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Ok(vec![]);
    }

    // Split the raw input into individual messages at each MSH boundary
    let messages = split_into_messages(raw);

    let mut results = Vec::with_capacity(messages.len());
    for msg_str in messages {
        let msg_str = msg_str.trim();
        if msg_str.is_empty() || !msg_str.starts_with("MSH") {
            continue;
        }
        match parse_message(msg_str) {
            Ok(msg) => results.push(msg),
            Err(e) => {
                return Err(pyo3::exceptions::PyValueError::new_err(format!(
                    "Error parsing message in batch: {}",
                    e
                )));
            }
        }
    }

    Ok(results)
}

/// Split raw input into individual message strings at MSH boundaries.
///
/// Strips FHS, BHS, BTS, FTS header/trailer lines.
fn split_into_messages(raw: &str) -> Vec<String> {
    let mut messages: Vec<String> = Vec::new();
    let mut current_msg = String::new();

    // Split on any line ending
    let lines: Vec<&str> = raw.split(['\r', '\n']).filter(|s| !s.is_empty()).collect();

    for line in lines {
        let seg_type = if line.len() >= 3 { &line[..3] } else { line };

        // Skip batch/file header and trailer segments
        match seg_type {
            "FHS" | "BHS" | "BTS" | "FTS" => continue,
            "MSH" => {
                // Start of a new message — flush the current one
                if !current_msg.is_empty() {
                    messages.push(current_msg);
                }
                current_msg = line.to_string();
            }
            _ => {
                // Append to current message
                if !current_msg.is_empty() {
                    current_msg.push('\r');
                }
                current_msg.push_str(line);
            }
        }
    }

    // Flush the last message
    if !current_msg.is_empty() {
        messages.push(current_msg);
    }

    messages
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_simple_batch() {
        let raw = "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rPID|1||MRN1\rMSH|^~\\&|S|F|R|F|20230102||ADT^A01|2|P|2.5\rPID|1||MRN2";
        let msgs = split_into_messages(raw);
        assert_eq!(msgs.len(), 2);
        assert!(msgs[0].starts_with("MSH"));
        assert!(msgs[0].contains("MRN1"));
        assert!(msgs[1].starts_with("MSH"));
        assert!(msgs[1].contains("MRN2"));
    }

    #[test]
    fn test_split_with_fhs_wrapper() {
        let raw = "FHS|^~\\&|BATCH\rBHS|^~\\&|BATCH\rMSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rPID|1||MRN1\rBTS|1\rFTS|1";
        let msgs = split_into_messages(raw);
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].starts_with("MSH"));
    }

    #[test]
    fn test_split_empty() {
        let msgs = split_into_messages("");
        assert!(msgs.is_empty());
    }

    #[test]
    fn test_split_single_message() {
        let raw = "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rPID|1||MRN";
        let msgs = split_into_messages(raw);
        assert_eq!(msgs.len(), 1);
    }
}
