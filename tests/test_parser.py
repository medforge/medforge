"""Tests for ironpipe HL7v2 parser."""

import json
import time

import ironpipe


# -- Sample messages ----------------------------------------------------------

SIMPLE_ADT = (
    "MSH|^~\\&|SENDER|FAC|RECV|FAC|20230101120000||ADT^A01|12345|P|2.5\r"
    "PID|1||MRN123^^^MRN||DOE^JOHN^M||19800101|M\r"
    "PV1|1|I|4EAST^401^1"
)

MULTI_DG1 = (
    "MSH|^~\\&|SENDER|FAC|RECV|FAC|20230101||ADT^A01|99|P|2.5\r"
    "PID|1||MRN||DOE^JANE\r"
    "DG1|1||I10^Hypertension^ICD10||20230101|A\r"
    "DG1|2||E11^Diabetes^ICD10||20230101|A"
)


# -- Basic parsing ------------------------------------------------------------


class TestBasicParsing:
    def test_parse_simple_adt(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert len(msg.segments) == 3
        assert msg.segments[0].name == "MSH"
        assert msg.segments[1].name == "PID"
        assert msg.segments[2].name == "PV1"

    def test_parse_returns_message_type(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert isinstance(msg, ironpipe.Message)

    def test_raw_preserved(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert "MSH|" in msg.raw
        assert "PID|" in msg.raw

    def test_repr(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        r = repr(msg)
        assert "MSH" in r
        assert "PID" in r

    def test_len(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert len(msg) == 3


# -- Segment access -----------------------------------------------------------


class TestSegmentAccess:
    def test_segment_by_name(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        assert pid.name == "PID"

    def test_segments_by_name(self):
        msg = ironpipe.parse(MULTI_DG1)
        dg1s = msg.segments_by_name("DG1")
        assert len(dg1s) == 2
        assert dg1s[0].name == "DG1"
        assert dg1s[1].name == "DG1"

    def test_segment_not_found(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        try:
            msg.segment("ZZZ")
            assert False, "Should have raised KeyError"
        except KeyError:
            pass

    def test_segment_repr(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        assert "PID" in repr(pid)

    def test_segment_len(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        assert len(pid) > 0


# -- Field & Component access -------------------------------------------------


class TestFieldComponents:
    def test_msh_field_separator(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        msh = msg.segment("MSH")
        assert msh.field(1).value == "|"

    def test_msh_encoding_chars(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        msh = msg.segment("MSH")
        assert msh.field(2).value == "^~\\&"

    def test_patient_name_components(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        name = pid.field(5)
        assert name.component(1).value == "DOE"
        assert name.component(2).value == "JOHN"
        assert name.component(3).value == "M"

    def test_field_string(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        assert str(pid.field(5)) == "DOE^JOHN^M"

    def test_field_getitem(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        assert pid[5][1].value == "DOE"

    def test_field_index_error(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        try:
            pid.field(999)
            assert False, "Should have raised IndexError"
        except IndexError:
            pass

    def test_component_index_error(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        name = pid.field(5)
        try:
            name.component(999)
            assert False, "Should have raised IndexError"
        except IndexError:
            pass


# -- Repetitions --------------------------------------------------------------


class TestRepetitions:
    def test_repetition_field(self):
        raw = (
            "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\r"
            "PID|1||MRN1^^^MRN~DEA1^^^DEA"
        )
        msg = ironpipe.parse(raw)
        pid = msg.segment("PID")
        id_field = pid.field(3)
        assert len(id_field.repetitions) == 2
        assert id_field.repetitions[0].components[0].value == "MRN1"
        assert id_field.repetitions[1].components[0].value == "DEA1"

    def test_no_repetition(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        name = pid.field(5)
        assert len(name.repetitions) == 0


# -- Sub-components -----------------------------------------------------------


class TestSubComponents:
    def test_subcomponents(self):
        raw = (
            "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\r"
            "PID|1||ID&CHECK^^^AUTH"
        )
        msg = ironpipe.parse(raw)
        pid = msg.segment("PID")
        comp = pid.field(3).component(1)
        assert len(comp.sub_components) == 2
        assert comp.sub_components[0] == "ID"
        assert comp.sub_components[1] == "CHECK"


# -- Encoding chars & empty fields --------------------------------------------


class TestEdgeCases:
    def test_empty_fields_preserved(self):
        raw = "MSH|^~\\&|||||20230101||ADT^A01|1|P|2.5\r" "PID|1||MRN|||||||"
        msg = ironpipe.parse(raw)
        pid = msg.segment("PID")
        assert len(pid.fields) >= 9

    def test_newline_delimiter(self):
        raw = "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\nPID|1||MRN"
        msg = ironpipe.parse(raw)
        assert len(msg.segments) == 2

    def test_crlf_delimiter(self):
        raw = "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\r\nPID|1||MRN"
        msg = ironpipe.parse(raw)
        assert len(msg.segments) == 2


# -- Escape sequences ---------------------------------------------------------


class TestEscapeSequences:
    def test_field_sep_escape(self):
        raw = (
            "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\r"
            "OBX|1|ST|CODE||value\\F\\more"
        )
        msg = ironpipe.parse(raw)
        obx = msg.segment("OBX")
        assert "|" in obx.field(5).value

    def test_component_sep_escape(self):
        raw = (
            "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\r"
            "OBX|1|ST|CODE||a\\S\\b"
        )
        msg = ironpipe.parse(raw)
        obx = msg.segment("OBX")
        assert "^" in obx.field(5).value


# -- Terser-style access ------------------------------------------------------


class TestTerserAccess:
    def test_basic_terser(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert msg["PID-5-1"] == "DOE"
        assert msg["PID-5-2"] == "JOHN"

    def test_terser_field_only(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert "DOE" in msg["PID-5"]

    def test_terser_get_method(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert msg.get("PID-5-1") == "DOE"

    def test_terser_segment_repetition(self):
        msg = ironpipe.parse(MULTI_DG1)
        # First DG1
        assert "I10" in msg["DG1-3-1"]
        # Access second DG1 via repetition syntax
        assert "E11" in msg["DG1(1)-3-1"]

    def test_terser_invalid_path(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        try:
            msg["ZZZ-1"]
            assert False, "Should have raised KeyError"
        except KeyError:
            pass


# -- MSH convenience properties -----------------------------------------------


class TestMSHProperties:
    def test_message_type(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert msg.message_type == ("ADT", "A01")

    def test_control_id(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert msg.control_id == "12345"

    def test_version(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert msg.version == "2.5"

    def test_sending_application(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert msg.sending_application == "SENDER"

    def test_sending_facility(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        assert msg.sending_facility == "FAC"


# -- MLLP framing -------------------------------------------------------------


class TestMLLP:
    def test_mllp_framed(self):
        raw = "\x0bMSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\rPID|1||MRN\x1c\r"
        msg = ironpipe.parse(raw)
        assert len(msg.segments) == 2

    def test_mllp_framed_with_cr(self):
        raw = "\x0bMSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5\x1c\r"
        msg = ironpipe.parse(raw)
        assert msg.segment("MSH") is not None


# -- Serialization ------------------------------------------------------------


class TestSerialization:
    def test_to_dict(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        d = msg.to_dict()
        assert "segments" in d
        assert len(d["segments"]) == 3
        assert d["segments"][1]["name"] == "PID"

    def test_to_json(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        j = msg.to_json()
        parsed = json.loads(j)
        assert len(parsed["segments"]) == 3

    def test_field_to_dict(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        pid = msg.segment("PID")
        d = pid.field(5).to_dict()
        assert "value" in d
        assert "components" in d

    def test_segment_to_dict(self):
        msg = ironpipe.parse(SIMPLE_ADT)
        d = msg.segment("PID").to_dict()
        assert d["name"] == "PID"
        assert "fields" in d


# -- Performance --------------------------------------------------------------


class TestPerformance:
    def test_large_message_performance(self):
        """Parse a message with 10k segments in reasonable time."""
        raw = "MSH|^~\\&|S|F|R|F|20230101||ADT^A01|1|P|2.5"
        for i in range(10_000):
            raw += f"\rOBX|{i}|NM|CODE-{i}||{i * 7}|unit|0-100||||F"

        start = time.time()
        msg = ironpipe.parse(raw)
        elapsed = (time.time() - start) * 1000  # ms

        assert len(msg.segments) == 10_001
        # Debug builds are ~3-5x slower; use 2s threshold as smoke test.
        # Release builds should be <100ms for this.
        assert elapsed < 2000, f"Took {elapsed:.1f}ms, expected < 2000ms"


# -- Error handling -----------------------------------------------------------


class TestErrors:
    def test_empty_message(self):
        try:
            ironpipe.parse("")
            assert False, "Should have raised ValueError"
        except ValueError:
            pass

    def test_no_msh(self):
        try:
            ironpipe.parse("PID|1||MRN")
            assert False, "Should have raised ValueError"
        except ValueError:
            pass
