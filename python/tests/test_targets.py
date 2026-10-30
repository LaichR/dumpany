import gc
import io
import weakref

import pytest

from dumpany import Context, Dumper, Epilogue, Join, Prologue


def test_targets_render_before_and_after_body(render):
    assert render(Context([
        "body",
        Epilogue("root", ":after"),
        Prologue("root", "before:"),
        Prologue("root", "again:"),
        "tail",
    ], name="root")) == "before:again:bodytail:after\n"


def test_closed_child_remains_targetable(render):
    name = "".join(["run", "time"])
    assert render(Context([
        Context("body", name=name),
        Prologue(name, "before:"),
        Epilogue(name, ":after"),
        "tail",
    ])) == "before:body:after\ntail\n"


def test_targets_accept_nested_operations(render):
    assert render(Context([
        "body",
        Prologue("root", [Join(",", [1, 2]), ":"]),
        Epilogue("root", Context("after", prefix="- ")),
    ], name="root")) == "1,2:body- after\n\n"


@pytest.mark.parametrize("operation", [Prologue, Epilogue])
def test_lazy_iterable_content(render, operation):
    calls = []

    def visitor():
        calls.append("started")
        yield "a"
        yield "b"

    target = operation("root", visitor())
    assert calls == []
    assert render(Context([target], name="root")) == "ab\n"
    assert calls == ["started"]
    assert render(Context([target], name="root")) == "\n"


@pytest.mark.parametrize("operation", [Prologue, Epilogue])
def test_selection_validation(render, operation):
    with pytest.raises(OSError, match="open context"):
        render(operation("root", "content"))
    with pytest.raises(OSError, match="unknown context"):
        render(Context([operation("missing", "content")], name="root"))
    with pytest.raises(TypeError):
        operation(123, "content")


@pytest.mark.parametrize("outer", [Prologue, Epilogue])
@pytest.mark.parametrize("inner", [Prologue, Epilogue])
def test_nested_target_selection_is_rejected(render, outer, inner):
    with pytest.raises(OSError, match="main body"):
        render(Context([outer("root", inner("root", "no"))], name="root"))


@pytest.mark.parametrize("operation", [Prologue, Epilogue])
def test_content_errors_preserve_exception_and_restore_main(operation):
    failure = ValueError("content failed")
    stream = io.StringIO()
    dumper = Dumper(stream)

    def broken():
        yield "partial"
        raise failure

    def body():
        yield "body"
        try:
            dumper.dump(operation("root", broken()))
        except ValueError as caught:
            assert caught is failure
        else:
            pytest.fail("expected content exception")
        yield "tail"

    dumper.dump(Context(body(), name="root"))
    dumper.dump_all()
    expected = "partialbodytail\n" if operation is Prologue else "bodytailpartial\n"
    assert stream.getvalue() == expected


def test_callbacks_can_write_to_selected_target():
    stream = io.StringIO()
    dumper = Dumper(stream)

    class Label:
        def __str__(self):
            dumper.dump("before:")
            return "label:"

    dumper.dump(Context(["body", Prologue("root", Label())], name="root"))
    dumper.dump_all()
    assert stream.getvalue() == "before:label:body\n"


@pytest.mark.parametrize("operation", [Prologue, Epilogue])
def test_cycle_detection_and_garbage_collection(render, operation):
    class Items(list):
        pass

    content = Items()
    target = operation("root", content)
    content.append(target)
    with pytest.raises(ValueError, match="cycle"):
        render(Context([target], name="root"))
    reference = weakref.ref(content)
    del content, target
    gc.collect()
    assert reference() is None


def test_repeated_target_operation_is_not_a_cycle(render):
    target = Prologue("root", "a")
    assert render(Context([target, target, "body"], name="root")) == "aabody\n"
