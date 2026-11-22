import io

import pytest

from dumpany import Context, Dumper, NoBreakRegion


@pytest.mark.parametrize("new_line", ["\n", "\r\n", "<br>"])
@pytest.mark.parametrize("explicit_break", [False, True])
def test_finalize_fragment_preserves_explicit_breaks(new_line, explicit_break):
    stream = io.StringIO()
    dumper = Dumper(stream, new_line=new_line)
    dumper.dump("fragment")
    if explicit_break:
        dumper.dump(...)
    dumper.dump_all(new_line=False)
    dumper.dump_all()
    stream.write(" suffix")
    assert stream.getvalue() == "fragment" + (new_line if explicit_break else "") + " suffix"
    assert not stream.closed
    with pytest.raises(OSError):
        dumper.dump("more")


def test_finalize_empty_fragment():
    stream = io.StringIO()
    dumper = Dumper(stream)
    dumper.dump_all(new_line=False)
    assert stream.getvalue() == ""


@pytest.mark.parametrize("closing_break", [False, True])
def test_context_closing_break_is_independent_of_finalization(closing_break):
    stream = io.StringIO()
    dumper = Dumper(stream)
    dumper.dump(Context("body", new_line=closing_break))
    dumper.dump_all(new_line=False)
    assert stream.getvalue() == "body" + ("\n" if closing_break else "")


def test_no_break_region_can_be_finalized_as_fragment():
    stream = io.StringIO()
    dumper = Dumper(stream)
    dumper.dump(NoBreakRegion(["a", " + ", "b"]))
    dumper.dump_all(new_line=False)
    assert stream.getvalue() == "a + b"
