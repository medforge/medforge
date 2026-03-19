# medforge 🔧

High-performance HL7v2 message parser for Python, powered by Rust.

## Features

- **Fast** — Rust core for 10-50× faster parsing vs pure-Python HL7 libraries
- **Complete** — Full HL7v2 hierarchy: Message → Segment → Field → Component → Sub-component
- **Ergonomic** — Pythonic API with terser-style path access (`msg["PID-5-1"]`)
- **MLLP-aware** — Automatic detection and stripping of MLLP framing
- **Escape-aware** — Decodes HL7 escape sequences (`\F\`, `\S\`, `\T\`, `\R\`, `\E\`)
- **Serializable** — Built-in `.to_dict()` and `.to_json()` methods

## Installation

```bash
pip install medforge
```

### From source (requires Rust)

```bash
git clone https://github.com/your-org/medforge.git
cd medforge
nix develop   # or install Rust + maturin manually
maturin develop
```

## Quick Start

```python
import medforge

msg = medforge.parse(
    "MSH|^~\\&|EPIC|HOSPITAL|RECV|FAC|20260318||ADT^A01^ADT_A01|MSG001|P|2.5.1\r"
    "PID|1||MRN12345^^^MRN||DOE^JANE^M^^DR||19850315|F\r"
    "PV1|1|I|4EAST^401^1^^^N||||1234^SMITH^ROBERT^J^^^MD\r"
    "DG1|1||I10^Essential Hypertension^ICD10||20260318|A"
)

# Segment access
pid = msg.segment("PID")
all_dg1 = msg.segments_by_name("DG1")

# Field access (1-indexed, per HL7 spec)
patient_name = pid.field(5)           # Field("DOE^JANE^M^^DR")
patient_name.component(1)            # "DOE"
patient_name.component(2)            # "JANE"

# Terser-style shorthand
msg["PID-5-1"]                       # "DOE"
msg["MSH-9-1"]                       # "ADT"

# MSH convenience properties
msg.message_type                     # ("ADT", "A01")
msg.control_id                       # "MSG001"
msg.version                          # "2.5.1"

# Serialization
msg.to_dict()                        # Python dict
msg.to_json()                        # JSON string
```

## Development

```bash
nix develop                          # enter dev shell
maturin develop                      # build + install in dev mode
cargo test                           # Rust tests
python -m pytest tests/ -v           # Python tests
```

## License

MIT
