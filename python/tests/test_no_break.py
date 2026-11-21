from io import StringIO

from dumpany import Context, Dumper, NoBreak


def test_no_break_operation_and_method():
    for contextual in (False, True):
        stream = StringIO()
        dumper = Dumper(stream, line_width=9)
        content = ["abcd ", NoBreak("ef gh ij kl")]
        dumper.dump(Context(content) if contextual else content)
        dumper.no_break("ééé 界界")
        dumper.dump_all()
        assert stream.getvalue() == "abcd\nef gh ij kl\nééé 界界\n"


def test_no_break_counts_characters_and_preserves_existing_newlines():
    stream = StringIO()
    dumper = Dumper(stream, line_width=5)
    dumper.dump(["é ", NoBreak("界 界")])
    dumper.no_break("")
    dumper.dump_all()
    assert stream.getvalue() == "é 界 界\n"

    stream = StringIO()
    dumper = Dumper(stream)
    dumper.no_break("one\ntwo")
    dumper.dump_all()
    assert stream.getvalue() == "one\ntwo\n"
