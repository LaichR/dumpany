import gc
import io
import weakref

import pytest

from dumpany import Context, Dumper, Join


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
