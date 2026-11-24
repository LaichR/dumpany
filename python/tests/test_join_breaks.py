import io

import pytest

from dumpany import BreakPolicy, Context, Dumper, Epilogue, Join, NewLine, NoBreakRegion, Prologue


def render(value, width=12, new_line="\n"):
    stream = io.StringIO()
    dumper = Dumper(stream, line_width=width, new_line=new_line)
    dumper.dump(value)
    dumper.dump_all()
    return stream.getvalue()


@pytest.mark.parametrize("policy, expected", [
    (BreakPolicy.Inherit, "call(aaa, bbb, ccc)\n"),
    (BreakPolicy.Always, "call(aaa,\nbbb,\nccc)\n"),
    (BreakPolicy.AsNeeded, "call(aaa,\nbbb, ccc)\n"),
])
def test_break_policy_inside_no_break_region(policy, expected):
    assert render(NoBreakRegion([
        "call(", Join(", ", ["aaa", "bbb", "ccc"], breaks=policy), ")",
    ])) == expected


def test_as_needed_respects_late_sections_and_context_alignment():
    value = Context([
        NoBreakRegion([
            Context(Join(", ", ["a", Context("b", name="item", new_line=False)],
                         breaks=BreakPolicy.AsNeeded), prefix="call(", new_line=False),
            ")",
        ]),
        Prologue("item", "long"),
        Epilogue("item", "!"),
    ])
    assert render(value) == "call(a,\n     longb!)\n"


@pytest.mark.parametrize("policy", [BreakPolicy.Inherit, BreakPolicy.Always, BreakPolicy.AsNeeded])
def test_empty_single_and_nested_joins(policy):
    assert render(Join(", ", [], breaks=policy)) == ""
    assert render(Join(", ", ["one"], breaks=policy)) == "one\n"
    assert render(Join("; ", ["one", Join(", ", ["a", "b"], breaks=policy)],
                       breaks=BreakPolicy.Always)) == (
        "one;\na,\nb\n" if policy == BreakPolicy.Always else "one;\na, b\n"
    )


def test_unicode_exact_fit_and_custom_line_ending():
    assert render(Join(", ", ["éé", "界界"], breaks=BreakPolicy.AsNeeded), width=6) == "éé, 界界\n"
    assert render(Join(", ", ["aaa", "bbb"], breaks=BreakPolicy.AsNeeded),
                  width=7, new_line="\r\n") == "aaa,\r\nbbb\r\n"


@pytest.mark.parametrize("policy", [BreakPolicy.Always, BreakPolicy.AsNeeded])
def test_explicit_newlines_and_oversized_items(policy):
    assert render(Join(..., ["first", "second"], breaks=policy), width=2) == "first\nsecond\n"
    assert render(Join(", ", ["first item", "second item"], breaks=policy),
                  width=3) == "first item,\nsecond item\n"


def test_callbacks_and_delimiters_are_recorded_once_per_gap():
    calls = []

    class Delimiter:
        def __str__(self):
            calls.append("delimiter")
            return ", "

    def items():
        for text in ("aaa", "bbb", "ccc"):
            calls.append(text)
            yield text

    assert render(Join(Delimiter(), items(), breaks=BreakPolicy.AsNeeded), width=8) == "aaa, bbb,\nccc\n"
    assert calls == ["aaa", "bbb", "delimiter", "ccc", "delimiter"]


def test_first_line_of_multiline_item_controls_fit():
    assert render(Join(", ", ["aaa", "b\nlong continuation"], breaks=BreakPolicy.AsNeeded),
                  width=8) == "aaa, b\nlong continuation\n"


def test_invalid_policy_is_rejected():
    with pytest.raises(TypeError):
        Join(", ", [], breaks="always")


@pytest.mark.parametrize("policy", [BreakPolicy.Inherit, BreakPolicy.Always, BreakPolicy.AsNeeded])
def test_indentation_changes_inside_items_are_terminal(policy):
    stream = io.StringIO()
    dumper = Dumper(stream)
    with pytest.raises(OSError, match="indentation"):
        dumper.dump(Join(", ", ["first", NewLine(increment=0)], breaks=policy))
    assert stream.getvalue() == ("first,\n" if policy == BreakPolicy.Always else "")
    with pytest.raises(OSError, match="failed"):
        dumper.dump_all()
