use pyo3::prelude::*;

mod escape;
mod mllp;
mod parser;
mod types;

use types::{Component, Field, Message, Segment};

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
    m.add_function(wrap_pyfunction!(parse, m)?)?;
    m.add_class::<Message>()?;
    m.add_class::<Segment>()?;
    m.add_class::<Field>()?;
    m.add_class::<Component>()?;
    Ok(())
}
