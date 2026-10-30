use crate::writer::PythonWriter;
use dumpany_rs::core::{Dumper, DumperConfig};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};
use std::collections::HashSet;

mod dispatch;

#[gen_stub_pyclass]
/// Write structured, width-aware text to a Python text stream.
///
/// Output may be buffered. Call dump_all() explicitly to finish writing and
/// flush the stream; the dumper never closes the stream.
#[pyclass(name = "Dumper", module = "dumpany", unsendable)]
pub struct PyDumper {
    inner: Dumper<PythonWriter>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyDumper {
    /// Create a dumper for a stream supporting write(str) and flush().
    ///
    /// line_width is the preferred wrapping width; new_line is the line-ending
    /// sequence. Normal indentation uses four spaces per level.
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

    /// Append text to the dump.
    fn text(&mut self, text: &str) -> PyResult<()> {
        self.inner.text(text).map_err(PyErr::from)
    }

    /// Dump any Python object as text or as a sequence of dump operations.
    ///
    /// NewLine, Join, Context, Prologue, and Epilogue control the output as
    /// described in their class documentation. Ellipsis emits a newline.
    ///
    /// Strings and bytes-like objects are converted to text without iterating
    /// over their contents. Other iterables are dumped recursively, without
    /// implicit separators; mappings yield their keys. Remaining objects are
    /// converted with str(). Iteration takes precedence over __str__().
    ///
    /// Cycles raise ValueError and excessive nesting raises RecursionError.
    /// Python exceptions propagate without rolling back preceding output.
    fn dump(slf: &Bound<'_, Self>, value: &Bound<'_, PyAny>) -> PyResult<()> {
        dispatch::dispatch(slf, value, &mut HashSet::new())
    }

    /// Finish the dump, emit pending text, and flush the Python stream.
    ///
    /// The stream remains open. After successful completion, repeated calls do
    /// nothing and no further text may be added.
    fn dump_all(&mut self) -> PyResult<()> {
        self.inner.dump_all().map_err(PyErr::from)
    }
}
