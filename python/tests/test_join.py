import gc
import io
import weakref

import pytest

from dumpany import Dumper, Join


@pytest.mark.parametrize("items, expected", [([], ""), (["one"], "one\n"), ([1, 2, 3], "1, 2, 3\n")])
def test_join_items(render, items, expected):
    assert render(Join(", ", items)) == expected


def test_recursive_items_and_delimiters(render):
    assert render(Join([",", ...], [["a", 1], ["b", 2]])) == "a1,\nb2\n"
    assert render(Join(" / ", [Join(",", [1, 2]), Join(",", [3, 4])])) == "1,2 / 3,4\n"


def test_lazy_one_shot_generator(render):
    calls = []

    def items():
        calls.append("started")
        yield "a"
        yield "b"

    operation = Join("-", items())
    assert calls == []
    assert render(operation) == "a-b\n"
    assert render(operation) == ""
    assert calls == ["started"]


def test_reusable_join_and_atomic_iterable_items(render):
    operation = Join("-", ["a", "b"])
    assert render([operation, ..., operation]) == "a-b\na-b\n"
    assert render(Join("-", "ab")) == "a-b\n"
    assert render(Join("-", [b"ab", b"cd"])) == "b'ab'-b'cd'\n"


def test_invalid_iterable_and_iteration_error(render):
    with pytest.raises(TypeError):
        render(Join(",", 123))
    failure = ValueError("iteration failed")

    def items():
        yield "first\n"
        raise failure

    stream = io.StringIO()
    dumper = Dumper(stream)
    with pytest.raises(ValueError) as caught:
        dumper.dump(Join("unwanted separator", items()))
    assert caught.value is failure
    assert stream.getvalue() == ""
    with pytest.raises(OSError, match="failed"):
        dumper.dump_all()


def test_join_cycles_and_depth(render):
    items = []
    operation = Join(",", items)
    items.append(operation)
    with pytest.raises(ValueError, match="cycle"):
        render(operation)
    delimiter = []
    operation = Join(delimiter, [1, 2])
    delimiter.append(operation)
    with pytest.raises(ValueError, match="cycle"):
        render(operation)
    value = "leaf"
    for _ in range(101):
        value = Join(",", [value])
    with pytest.raises(RecursionError):
        render(value)


def test_cycles_are_garbage_collected():
    class Items(list):
        pass

    items = Items()
    reference = weakref.ref(items)
    items.append(Join(",", items))
    del items
    gc.collect()
    assert reference() is None
