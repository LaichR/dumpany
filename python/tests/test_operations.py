import io

import pytest

from dumpany import Dumper, NewLine


def test_nested_iterables_and_scalars(render):
    assert render(["Name: ", "Ada", ..., ("Count: ", 42), ...]) == (
        "Name: Ada\nCount: 42\n"
    )
    assert render([True, " ", None, " ", 1.5]) == "True None 1.5\n"
    assert render(iter(["a", "b"])) == "ab\n"
    assert render({"first": 1, "second": 2}) == "firstsecond\n"


@pytest.mark.parametrize("value", ["hello", b"hello", bytearray(b"hello"), memoryview(b"hello")])
def test_strings_and_bytes_are_atomic(render, value):
    assert render(value) == str(value) + "\n"


def test_newlines_and_indentation(render):
    assert render([
        "Heading", NewLine(increment=1),
        "Indented", ..., "Still indented", NewLine(decrement=1),
        "End", NewLine(),
    ]) == "Heading\n    Indented\n    Still indented\nEnd\n"


def test_newline_validation(render):
    with pytest.raises(ValueError, match="not both"):
        NewLine(increment=0, decrement=0)
    for count in [-1, 256]:
        with pytest.raises(OverflowError):
            NewLine(increment=count)
        with pytest.raises(OverflowError):
            NewLine(decrement=count)
    with pytest.raises(TypeError):
        NewLine(1)
    assert render(["a", NewLine(increment=0), "b"]) == "a\nb\n"


def test_cycles_but_not_repeated_references_are_rejected(render):
    value = []
    value.append(value)
    with pytest.raises(ValueError, match="cycle"):
        render(value)
    first, second = [], []
    first.append(second)
    second.append(first)
    with pytest.raises(ValueError, match="cycle"):
        render(first)
    shared = ["hello"]
    assert render([shared, shared]) == "hellohello\n"


def test_deep_nesting_is_rejected(render):
    value = "hello"
    for _ in range(101):
        value = [value]
    with pytest.raises(RecursionError, match="nesting"):
        render(value)


def test_custom_str_and_sequence_protocol(render):
    class Label:
        def __str__(self):
            return "label"

    class Sequence:
        def __getitem__(self, index):
            if index >= 2:
                raise IndexError
            return Label()

    assert render(Sequence()) == "labellabel\n"


def test_iteration_takes_precedence_over_str(render):
    class Items:
        def __iter__(self):
            yield "items"

        def __str__(self):
            return "not used"

    assert render(Items()) == "items\n"


def test_python_errors_propagate_unchanged():
    failure = TypeError("custom failure")

    class BrokenString:
        def __str__(self):
            raise failure

    class BrokenIterable:
        def __iter__(self):
            raise failure

    def broken_generator():
        yield "before\n"
        raise failure

    for value in [BrokenString(), BrokenIterable(), broken_generator()]:
        stream = io.StringIO()
        dumper = Dumper(stream)
        with pytest.raises(TypeError) as caught:
            dumper.dump(value)
        assert caught.value is failure
    assert stream.getvalue() == "before\n"


def test_callbacks_can_use_dumper_without_borrow_conflicts():
    stream = io.StringIO()
    dumper = Dumper(stream)

    class Label:
        def __str__(self):
            dumper.text("prefix:")
            return "label"

    def items():
        dumper.text("items:")
        yield Label()

    dumper.dump(items())
    dumper.dump_all()
    assert stream.getvalue() == "items:prefix:label\n"
