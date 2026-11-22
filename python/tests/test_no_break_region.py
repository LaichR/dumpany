import gc
import io
import weakref

import pytest

from dumpany import Context, Dumper, Join, NoBreakRegion


def render(content, width=8):
    stream = io.StringIO()
    dumper = Dumper(stream, line_width=width)
    dumper.dump(content)
    dumper.dump_all()
    return stream.getvalue()


@pytest.mark.parametrize("recorded", [False, True])
def test_generator_statement_is_measured_and_consumed_once(recorded):
    calls = []

    def statement():
        for text in ("a", " + ", "b"):
            calls.append(text)
            yield text

    content = ["long ", NoBreakRegion(statement())]
    assert render(Context(content) if recorded else content) == "long\na + b\n"
    assert calls == ["a", " + ", "b"]


def test_nested_regions_joins_and_explicit_newlines():
    assert render(NoBreakRegion([
        "first ", NoBreakRegion(Join(", ", [1, 2, 3])),
        ..., "second line is long",
    ]), width=4) == "first 1, 2, 3\nsecond line is long\n"


def test_contexts_inside_region_preserve_explicit_closing_breaks():
    assert render(NoBreakRegion([
        Context("a b c d e", prefix="- "),
        Context("x y z", prefix="(", new_line=False), ")",
    ]), width=4) == "- a b c d e\n(x y z)\n"


def test_callback_error_permanently_fails_dumper():
    stream = io.StringIO()
    dumper = Dumper(stream, line_width=4)

    def broken():
        yield "first "
        raise ValueError("failed")

    with pytest.raises(ValueError, match="failed"):
        dumper.dump(NoBreakRegion(broken()))
    with pytest.raises(OSError, match="failed"):
        dumper.no_break("tail")
    with pytest.raises(OSError, match="failed"):
        dumper.dump_all()
    assert stream.getvalue() == ""


def test_cycles_are_detected_and_collected():
    class Content(list):
        pass

    content = Content()
    region = NoBreakRegion(content)
    content.append(region)
    with pytest.raises(ValueError, match="cycle"):
        render(region)
    reference = weakref.ref(content)
    del region, content
    gc.collect()
    assert reference() is None
