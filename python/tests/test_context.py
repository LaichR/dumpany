import gc
import io
import weakref

import pytest

from dumpany import Context, Dumper, Join, NewLine


def test_context_prefix_and_wrapping(render):
    assert render(Context("hello world again", prefix="- "), line_width=12) == (
        "- hello\n  world\n  again\n"
    )


def test_nested_contexts_and_joins(render):
    value = Context([
        Context("hello", prefix="* "),
        Context(Join(", ", [1, 2, 3]), prefix="* "),
    ], prefix="- ")
    assert render(value) == "- * hello\n* 1, 2, 3\n\n"


def test_runtime_names_and_reuse_after_root_flush(render):
    operation = Context("body", prefix="".join(["-", " "]), name="".join(["run", "time"]))
    assert render([operation, operation]) == "- body\n- body\n"
    with pytest.raises(OSError, match="already registered"):
        render(Context([operation, operation]))


def test_generator_is_lazy_and_one_shot(render):
    calls = []

    def visitor():
        calls.append("started")
        yield "hello"
        yield " world"

    operation = Context(visitor())
    assert calls == []
    assert render(operation) == "hello world\n"
    assert calls == ["started"]
    assert render(operation) == "\n"


def test_content_error_closes_context_and_preserves_exception():
    failure = ValueError("content failed")

    def visitor():
        yield "before"
        raise failure

    stream = io.StringIO()
    dumper = Dumper(stream)
    with pytest.raises(ValueError) as caught:
        dumper.dump(Context(visitor(), prefix="- "))
    assert caught.value is failure
    dumper.dump("after")
    dumper.dump_all()
    assert stream.getvalue() == "- before\nafter\n"


def test_content_error_takes_precedence_over_close_error():
    failure = ValueError("content failed")

    class BrokenStream:
        def write(self, text):
            raise RuntimeError("close failed")

    def visitor():
        yield "before"
        raise failure

    dumper = Dumper(BrokenStream())
    with pytest.raises(ValueError) as caught:
        dumper.dump(Context(visitor()))
    assert caught.value is failure


def test_close_error_is_reported():
    class BrokenStream:
        def write(self, text):
            raise RuntimeError("close failed")

    with pytest.raises(RuntimeError, match="close failed"):
        Dumper(BrokenStream()).dump(Context("hello"))


def test_cycles_depth_and_gc(render):
    class Items(list):
        pass

    items = Items()
    operation = Context(items)
    items.append(operation)
    with pytest.raises(ValueError, match="cycle"):
        render(operation)
    reference = weakref.ref(items)
    del items, operation
    gc.collect()
    assert reference() is None

    value = "leaf"
    for _ in range(101):
        value = Context(value)
    with pytest.raises(RecursionError, match="nesting"):
        render(value)


def test_callbacks_can_use_dumper():
    stream = io.StringIO()
    dumper = Dumper(stream)

    def visitor():
        dumper.dump("before:")
        yield "body"

    dumper.dump(Context(visitor()))
    dumper.dump_all()
    assert stream.getvalue() == "before:body\n"


def test_metadata_requires_strings():
    with pytest.raises(TypeError):
        Context("hello", prefix=123)
    with pytest.raises(TypeError):
        Context("hello", name=123)


def test_indentation_changes_after_closing_without_an_extra_newline(render):
    assert render([
        Context("heading", increment=2),
        Context("body", decrement=1),
        "tail",
    ]) == "heading\n        body\n    tail\n"


def test_nested_context_close_indentation_is_recorded(render):
    assert render(Context([
        Context("heading", increment=1),
        Context("body", decrement=1),
        "tail",
    ])) == "heading\n    body\ntail\n"


@pytest.mark.parametrize("nested", [False, True])
def test_close_decrement_stops_at_zero(render, nested):
    content = [Context("body", decrement=255), "tail"]
    assert render(Context(content) if nested else content) == "body\ntail\n"


def test_indentation_is_not_implicitly_restored(render):
    assert render([Context(["heading", NewLine(increment=1), "body"]), "tail"]) == (
        "heading\nbody\n    tail\n"
    )


@pytest.mark.parametrize("count", [0, 1])
def test_close_indentation_directions_are_mutually_exclusive(count):
    with pytest.raises(ValueError, match="either increment or decrement"):
        Context("body", increment=count, decrement=count)


@pytest.mark.parametrize("keyword", ["increment", "decrement"])
@pytest.mark.parametrize("count", [-1, 256])
def test_close_indentation_counts_must_fit_u8(keyword, count):
    with pytest.raises(OverflowError):
        Context("body", **{keyword: count})


def test_close_indentation_applies_during_content_error_cleanup():
    def broken():
        yield "before"
        raise ValueError("content failed")

    stream = io.StringIO()
    dumper = Dumper(stream)
    with pytest.raises(ValueError, match="content failed"):
        dumper.dump(Context(broken(), increment=1))
    dumper.dump("after")
    dumper.dump_all()
    assert stream.getvalue() == "before\n    after\n"
