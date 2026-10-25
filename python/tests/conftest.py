import io

import pytest

from dumpany import Dumper


@pytest.fixture
def render():
    """Dump a value to text using optional constructor configuration."""
    def render_value(value, **config):
        stream = io.StringIO()
        dumper = Dumper(stream, **config)
        dumper.dump(value)
        dumper.dump_all()
        return stream.getvalue()

    return render_value
