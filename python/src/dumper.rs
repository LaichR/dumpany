use crate::writer::PythonWriter;
use dumpany::core::{Dumper, DumperConfig};
use pyo3::prelude::*;

/// A text dumper writing to a Python text stream without closing it.
#[pyclass(name = "Dumper", module = "dumpany", unsendable)]
pub struct PyDumper {
    inner: Dumper<PythonWriter>,
}

#[pymethods]
impl PyDumper {
    #[new]
    #[pyo3(signature = (stream, *, line_width=120, new_line="\n"))]
    fn new(stream: Py<PyAny>, line_width: usize, new_line: &str) -> Self {
        Self {
            inner: Dumper::new(
                PythonWriter::new(stream),
                DumperConfig {
                    line_width,
                    new_line: new_line.to_owned(),
                    ..DumperConfig::default()
                },
            ),
        }
    }

    /// Append text to the dump. Output may be buffered until dump_all().
    fn text(&mut self, text: &str) -> PyResult<()> {
        self.inner.text(text).map_err(PyErr::from)
    }

    /// Emit pending text and flush, but never close, the stream.
    /// Successful repeated calls do nothing. No further text may be added.
    fn dump_all(&mut self) -> PyResult<()> {
        self.inner.dump_all().map_err(PyErr::from)
    }
}
