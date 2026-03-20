use pyo3::prelude::*;

mod batch;
mod escape;
mod mllp;
mod parser;
mod timestamp;
mod types;

use batch::parse_batch;
use timestamp::{parse_date, parse_datetime};
use types::{Component, Field, Message, MessageIterator, Segment, SegmentIterator};

/// Parse a raw HL7v2 message string into a `Message` object.
///
/// Handles MLLP-framed input automatically. Supports `\r`, `\n`, and `\r\n`
/// segment delimiters.
///
/// # Example (Python)
///
/// ```python
/// import medforge
///
/// msg = medforge.parse("MSH|^~\\&|SENDER|FAC|RECV|FAC|20230101||ADT^A01|123|P|2.5\\rPID|1||MRN||DOE^JOHN")
/// print(msg.segment("PID").field(5).component(1))  # "DOE"
/// ```
#[pyfunction]
fn parse(raw: &str) -> PyResult<Message> {
    parser::parse_message(raw).map_err(pyo3::exceptions::PyValueError::new_err)
}

/// medforge — High-performance HL7v2 message parser.
#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // Core types
    m.add_class::<Message>()?;
    m.add_class::<Segment>()?;
    m.add_class::<Field>()?;
    m.add_class::<Component>()?;
    m.add_class::<MessageIterator>()?;
    m.add_class::<SegmentIterator>()?;

    // Functions
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_function(wrap_pyfunction!(parse_batch, m)?)?;
    m.add_function(wrap_pyfunction!(parse_datetime, m)?)?;
    m.add_function(wrap_pyfunction!(parse_date, m)?)?;

    Ok(())
}
