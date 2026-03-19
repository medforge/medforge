"""Benchmarks for ironpipe HL7v2 parser.

Run with:
    python -m pytest benches/bench_parser.py -v --benchmark-group-by=func

Requires: pip install pytest-benchmark
"""

import ironpipe

# -- Sample messages of varying sizes ----------------------------------------

SIMPLE_ADT = (
    "MSH|^~\\&|SENDER|FAC|RECV|FAC|20230101120000||ADT^A01|12345|P|2.5\r"
    "EVN|A01|20230101120000\r"
    "PID|1||MRN12345^^^MRN||DOE^JANE^M^^DR||19850315|F|||123 MAIN ST^^CHICAGO^IL^60601\r"
    "PV1|1|I|4EAST^401^1^^^N||||1234^SMITH^ROBERT^J^^^MD\r"
    "DG1|1||I10^Essential Hypertension^ICD10||20230101|A\r"
    "DG1|2||E11.9^Type 2 Diabetes^ICD10||20230101|A"
)

MEDIUM_ORU = (
    "MSH|^~\\&|LAB|HOSPITAL|EMR|HOSPITAL|20230101120000||ORU^R01|MSG001|P|2.5.1\r"
    "PID|1||MRN99999^^^MRN||SMITH^JOHN^A||19700515|M\r"
    "PV1|1|O|CLINIC^^^MAIN\r"
    "ORC|RE|ORD001|FIL001\r"
    "OBR|1|ORD001|FIL001|CBC^Complete Blood Count^L|||20230101\r"
)

# Add 20 OBX segments
for i in range(20):
    MEDIUM_ORU += f"OBX|{i+1}|NM|CODE-{i}^Test {i}^L||{10+i}|mg/dL|5-25||||F\r"


def _build_large_message(segment_count: int) -> str:
    """Build a message with N OBX segments."""
    raw = "MSH|^~\\&|S|F|R|F|20230101120000||ORU^R01|1|P|2.5\rPID|1||MRN||DOE^JOHN"
    for i in range(segment_count):
        raw += f"\rOBX|{i}|NM|CODE-{i}^Test^L||{i * 7}|unit|0-100||||F"
    return raw


LARGE_1K = _build_large_message(1_000)
LARGE_10K = _build_large_message(10_000)


# -- Benchmarks ---------------------------------------------------------------


def test_bench_parse_simple_adt(benchmark):
    """Parse a typical 6-segment ADT message."""
    benchmark(ironpipe.parse, SIMPLE_ADT)


def test_bench_parse_medium_oru(benchmark):
    """Parse a 25-segment ORU message."""
    benchmark(ironpipe.parse, MEDIUM_ORU)


def test_bench_parse_large_1k(benchmark):
    """Parse a message with 1,000 OBX segments."""
    benchmark(ironpipe.parse, LARGE_1K)


def test_bench_parse_large_10k(benchmark):
    """Parse a message with 10,000 OBX segments."""
    benchmark(ironpipe.parse, LARGE_10K)


def test_bench_terser_access(benchmark):
    """Terser path access on a parsed message."""
    msg = ironpipe.parse(SIMPLE_ADT)
    benchmark(msg.get, "PID-5-1")


def test_bench_to_json(benchmark):
    """Serialize a parsed ADT message to JSON."""
    msg = ironpipe.parse(SIMPLE_ADT)
    benchmark(msg.to_json)


def test_bench_to_dict(benchmark):
    """Serialize a parsed ADT message to dict."""
    msg = ironpipe.parse(SIMPLE_ADT)
    benchmark(msg.to_dict)


def test_bench_segment_lookup(benchmark):
    """Look up a segment by name on a large message."""
    msg = ironpipe.parse(LARGE_1K)
    benchmark(msg.segment, "PID")


# -- Comparison helper (run manually) ----------------------------------------

def test_bench_vs_python_hl7(benchmark):
    """Compare against python-hl7 if installed.

    Install with: pip install hl7
    Skip if not available.
    """
    try:
        import hl7
    except ImportError:
        import pytest
        pytest.skip("python-hl7 not installed")

    # python-hl7 uses \r as separator and its own parse function
    raw = SIMPLE_ADT
    benchmark(hl7.parse, raw)
