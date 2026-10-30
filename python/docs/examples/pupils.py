import io
import json
from collections.abc import Iterable
from dataclasses import dataclass

from dumpany import Context, Dumper, NewLine, Prologue


@dataclass
class Pupil:
    name: str
    surname: str


@dataclass
class SchoolClass:
    name: str
    pupils: Iterable[Pupil]


def class_yaml(school_class):
    count = 0
    yield NewLine(increment=1)
    for pupil in school_class.pupils:
        count += 1
        yield Context([
            "name: ", json.dumps(pupil.name), ...,
            "surname: ", json.dumps(pupil.surname),
        ], prefix="- ")
    yield NewLine(decrement=1)
    yield Prologue("school-class", [
        "name: ", json.dumps(school_class.name), ...,
        "pupil_count: ", count, ...,
        "pupils:", " []" if count == 0 else "",
    ])


school_class = SchoolClass("Class 4B", (
    pupil for pupil in [Pupil("Alice", "Smith"), Pupil("Ben", "Jones")]
))
stream = io.StringIO()
dumper = Dumper(stream)
dumper.dump(Context(class_yaml(school_class), name="school-class"))
dumper.dump_all()
print(stream.getvalue(), end="")
