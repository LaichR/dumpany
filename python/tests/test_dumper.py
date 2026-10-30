import io

import pytest

from dumpany import Dumper


def test_dump_and_finalization():
    stream = io.StringIO()
    dumper = Dumper(stream)
    dumper.dump("héllo ")
    dumper.dump("世界 🦀")
    dumper.dump_all()
    assert stream.getvalue() == "héllo 世界 🦀\n"
    assert not stream.closed
    dumper.dump_all()
    assert stream.getvalue() == "héllo 世界 🦀\n"
    with pytest.raises(OSError, match="finalized"):
        dumper.dump("too late")


def test_constructor_configuration():
    stream = io.StringIO()
    dumper = Dumper(stream, line_width=5, new_line="\r\n")
    dumper.dump("hello world")
    dumper.dump_all()
    assert stream.getvalue() == "hello\r\nworld\r\n"


def test_negative_line_width_is_rejected():
    with pytest.raises(OverflowError):
        Dumper(io.StringIO(), line_width=-1)


def test_stream_errors_are_reported():
    class BrokenStream:
        def write(self, text):
            raise ValueError("write failed")

    dumper = Dumper(BrokenStream())
    dumper.dump("hello")
    with pytest.raises(ValueError, match="write failed"):
        dumper.dump_all()
