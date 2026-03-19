use pyo3::prelude::*;
use pyo3::types::PyDict;
use serde::Serialize;

/// HL7v2 encoding characters extracted from MSH segment.
#[derive(Debug, Clone)]
pub struct EncodingChars {
    pub field_sep: char,
    pub component_sep: char,
    pub repetition_sep: char,
    pub escape_char: char,
    pub subcomponent_sep: char,
}

impl Default for EncodingChars {
    fn default() -> Self {
        Self {
            field_sep: '|',
            component_sep: '^',
            repetition_sep: '~',
            escape_char: '\\',
            subcomponent_sep: '&',
        }
    }
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

/// A single component within an HL7v2 field.
///
/// Components may contain sub-components separated by `&`.
#[pyclass]
#[derive(Debug, Clone, Serialize)]
pub struct Component {
    #[pyo3(get)]
    pub value: String,

    #[pyo3(get)]
    pub sub_components: Vec<String>,
}

#[pymethods]
impl Component {
    /// Return sub-component at 1-based index.
    #[pyo3(signature = (index))]
    fn sub_component(&self, index: usize) -> PyResult<String> {
        if index == 0 || index > self.sub_components.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "Sub-component index {} out of range (1..{})",
                index,
                self.sub_components.len()
            )));
        }
        Ok(self.sub_components[index - 1].clone())
    }

    fn __repr__(&self) -> String {
        format!("Component('{}')", self.value)
    }

    fn __str__(&self) -> String {
        self.value.clone()
    }
}

// ---------------------------------------------------------------------------
// Field
// ---------------------------------------------------------------------------

/// A single field within an HL7v2 segment.
///
/// Fields may contain components (separated by `^`) and repetitions (separated
/// by `~`).
#[pyclass]
#[derive(Debug, Clone, Serialize)]
pub struct Field {
    #[pyo3(get)]
    pub value: String,

    #[pyo3(get)]
    pub components: Vec<Component>,

    #[pyo3(get)]
    pub repetitions: Vec<Field>,
}

#[pymethods]
impl Field {
    /// Return component at 1-based index.
    #[pyo3(signature = (index))]
    fn component(&self, index: usize) -> PyResult<Component> {
        if index == 0 || index > self.components.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "Component index {} out of range (1..{})",
                index,
                self.components.len()
            )));
        }
        Ok(self.components[index - 1].clone())
    }

    fn __repr__(&self) -> String {
        format!("Field('{}')", self.value)
    }

    fn __str__(&self) -> String {
        self.value.clone()
    }

    fn __len__(&self) -> usize {
        self.components.len()
    }

    fn __getitem__(&self, index: usize) -> PyResult<Component> {
        self.component(index)
    }

    /// Serialize to a Python dict.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("value", &self.value)?;
        let comp_list: Vec<String> = self.components.iter().map(|c| c.value.clone()).collect();
        dict.set_item("components", comp_list)?;
        if !self.repetitions.is_empty() {
            let rep_list: PyResult<Vec<PyObject>> =
                self.repetitions.iter().map(|r| r.to_dict(py)).collect();
            dict.set_item("repetitions", rep_list?)?;
        }
        Ok(dict.into())
    }
}

// ---------------------------------------------------------------------------
// Segment
// ---------------------------------------------------------------------------

/// A single segment within an HL7v2 message (e.g. MSH, PID, OBX).
#[pyclass]
#[derive(Debug, Clone, Serialize)]
pub struct Segment {
    #[pyo3(get)]
    pub name: String,

    #[pyo3(get)]
    pub fields: Vec<Field>,
}

#[pymethods]
impl Segment {
    /// Return field at 1-based index.
    ///
    /// For MSH, field(1) returns the field separator "|".
    #[pyo3(signature = (index))]
    pub fn field(&self, index: usize) -> PyResult<Field> {
        if index == 0 || index > self.fields.len() {
            return Err(pyo3::exceptions::PyIndexError::new_err(format!(
                "Field index {} out of range (1..{})",
                index,
                self.fields.len()
            )));
        }
        Ok(self.fields[index - 1].clone())
    }

    fn __repr__(&self) -> String {
        format!("Segment('{}', fields={})", self.name, self.fields.len())
    }

    fn __str__(&self) -> String {
        self.name.clone()
    }

    fn __len__(&self) -> usize {
        self.fields.len()
    }

    fn __getitem__(&self, index: usize) -> PyResult<Field> {
        self.field(index)
    }

    /// Serialize to a Python dict.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        dict.set_item("name", &self.name)?;
        let field_list: PyResult<Vec<PyObject>> =
            self.fields.iter().map(|f| f.to_dict(py)).collect();
        dict.set_item("fields", field_list?)?;
        Ok(dict.into())
    }
}

// ---------------------------------------------------------------------------
// Message
// ---------------------------------------------------------------------------

/// A parsed HL7v2 message.
#[pyclass]
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    #[pyo3(get)]
    pub raw: String,

    #[pyo3(get)]
    pub segments: Vec<Segment>,
}

#[pymethods]
impl Message {
    /// Return the first segment matching the given name.
    #[pyo3(signature = (name))]
    pub fn segment(&self, name: &str) -> PyResult<Segment> {
        self.segments
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .ok_or_else(|| {
                pyo3::exceptions::PyKeyError::new_err(format!("Segment '{}' not found", name))
            })
    }

    /// Return all segments matching the given name.
    #[pyo3(signature = (name))]
    fn segments_by_name(&self, name: &str) -> Vec<Segment> {
        self.segments
            .iter()
            .filter(|s| s.name == name)
            .cloned()
            .collect()
    }

    // -- MSH convenience properties ------------------------------------------

    /// Message type, e.g. ("ADT", "A01").
    #[getter]
    fn message_type(&self) -> PyResult<(String, String)> {
        let msh = self.segment("MSH")?;
        let msg_type_field = msh.field(9)?;
        let event_type = if msg_type_field.components.len() >= 1 {
            msg_type_field.components[0].value.clone()
        } else {
            String::new()
        };
        let trigger = if msg_type_field.components.len() >= 2 {
            msg_type_field.components[1].value.clone()
        } else {
            String::new()
        };
        Ok((event_type, trigger))
    }

    /// Message control ID from MSH-10.
    #[getter]
    fn control_id(&self) -> PyResult<String> {
        let msh = self.segment("MSH")?;
        let field = msh.field(10)?;
        Ok(field.value.clone())
    }

    /// HL7 version from MSH-12.
    #[getter]
    fn version(&self) -> PyResult<String> {
        let msh = self.segment("MSH")?;
        let field = msh.field(12)?;
        Ok(field.value.clone())
    }

    /// Sending application from MSH-3.
    #[getter]
    fn sending_application(&self) -> PyResult<String> {
        let msh = self.segment("MSH")?;
        let field = msh.field(3)?;
        Ok(field.value.clone())
    }

    /// Sending facility from MSH-4.
    #[getter]
    fn sending_facility(&self) -> PyResult<String> {
        let msh = self.segment("MSH")?;
        let field = msh.field(4)?;
        Ok(field.value.clone())
    }

    // -- Terser-style path access --------------------------------------------

    /// Access fields via terser path notation: "PID-5-1" or "PID-5".
    ///
    /// Format: `SEGMENT-FIELD[-COMPONENT[-SUBCOMPONENT]]`
    fn __getitem__(&self, path: &str) -> PyResult<String> {
        self.terser_get(path)
    }

    /// Terser path access method. Same as bracket notation.
    #[pyo3(signature = (path))]
    fn get(&self, path: &str) -> PyResult<String> {
        self.terser_get(path)
    }

    // -- Serialization -------------------------------------------------------

    /// Serialize the entire message to a Python dict.
    fn to_dict(&self, py: Python<'_>) -> PyResult<PyObject> {
        let dict = PyDict::new(py);
        let seg_list: PyResult<Vec<PyObject>> =
            self.segments.iter().map(|s| s.to_dict(py)).collect();
        dict.set_item("segments", seg_list?)?;
        Ok(dict.into())
    }

    /// Serialize the entire message to a JSON string.
    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            pyo3::exceptions::PyValueError::new_err(format!("JSON serialization error: {}", e))
        })
    }

    fn __repr__(&self) -> String {
        let seg_names: Vec<&str> = self.segments.iter().map(|s| s.name.as_str()).collect();
        format!("Message(segments=[{}])", seg_names.join(", "))
    }

    fn __len__(&self) -> usize {
        self.segments.len()
    }
}

impl Message {
    /// Internal terser path resolver.
    ///
    /// Path format: `SEGMENT-FIELD[-COMPONENT[-SUBCOMPONENT]]`
    /// Can also handle segment repetition via `SEGMENT(n)` (0-based).
    fn terser_get(&self, path: &str) -> PyResult<String> {
        let parts: Vec<&str> = path.split('-').collect();
        if parts.is_empty() {
            return Err(pyo3::exceptions::PyValueError::new_err(
                "Empty terser path",
            ));
        }

        // Parse segment name and optional repetition index
        let (seg_name, seg_rep) = parse_segment_ref(parts[0]);

        // Find the segment
        let matching: Vec<&Segment> = self
            .segments
            .iter()
            .filter(|s| s.name == seg_name)
            .collect();

        if matching.is_empty() {
            return Err(pyo3::exceptions::PyKeyError::new_err(format!(
                "Segment '{}' not found",
                seg_name
            )));
        }

        let seg = matching.get(seg_rep).ok_or_else(|| {
            pyo3::exceptions::PyIndexError::new_err(format!(
                "Segment '{}' repetition {} not found (have {})",
                seg_name,
                seg_rep,
                matching.len()
            ))
        })?;

        // If no field specified, return segment name
        if parts.len() == 1 {
            return Ok(seg.name.clone());
        }

        let field_idx: usize = parts[1].parse().map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid field index: '{}'",
                parts[1]
            ))
        })?;
        let field = seg.field(field_idx)?;

        if parts.len() == 2 {
            return Ok(field.value.clone());
        }

        let comp_idx: usize = parts[2].parse().map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid component index: '{}'",
                parts[2]
            ))
        })?;
        let comp = field.component(comp_idx)?;

        if parts.len() == 3 {
            return Ok(comp.value.clone());
        }

        let sub_idx: usize = parts[3].parse().map_err(|_| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid sub-component index: '{}'",
                parts[3]
            ))
        })?;
        comp.sub_component(sub_idx)
    }
}

/// Parse a segment reference like "PID" or "OBX(1)" into (name, repetition_index).
fn parse_segment_ref(s: &str) -> (&str, usize) {
    if let Some(paren_start) = s.find('(') {
        if let Some(paren_end) = s.find(')') {
            let name = &s[..paren_start];
            let idx: usize = s[paren_start + 1..paren_end].parse().unwrap_or(0);
            return (name, idx);
        }
    }
    (s, 0)
}
