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
    /// :param stream: A Python text stream with write(str) and flush() methods.
    ///     The dumper never closes it.
    /// :param line_width: Preferred line width in Unicode characters.
    ///     Defaults to 120. Normal indentation uses four spaces per level.
    /// :param new_line: Line-ending sequence, such as ``"\n"`` or ``"\r\n"``.
    ///     Defaults to ``"\n"``. Must not be empty.
    #[new]
    #[pyo3(signature = (stream, *, line_width=120, new_line="\n"))]
    fn new(stream: Py<PyAny>, line_width: usize, new_line: &str) -> PyResult<Self> {
        Ok(Self {
            inner: Dumper::new(
                PythonWriter::new(stream),
                DumperConfig {
                    line_width,
                    new_line: new_line.to_owned(),
                    ..DumperConfig::default()
                },
            )
            .map_err(|error| pyo3::exceptions::PyValueError::new_err(error.to_string()))?,
        })
    }

    /// Dump any Python object as text or as a sequence of dump operations.
    ///
    /// NewLine, NoBreak, NoBreakRegion, Join, Context, Prologue, and Epilogue control the output as
    /// described in their class documentation. Ellipsis emits a newline.
    ///
    /// Strings and bytes-like objects are converted to text without iterating
    /// over their contents. Other iterables are dumped recursively, without
    /// implicit separators; mappings yield their keys. Remaining objects are
    /// converted with str(). Iteration takes precedence over __str__().
    ///
    /// Cycles raise ValueError and excessive nesting raises RecursionError.
    /// Python exceptions propagate without rolling back preceding output.
    ///
    /// :param value: Text, a dump operation, an iterable of supported values,
    ///     or an object to convert with str().
    fn dump(slf: &Bound<'_, Self>, value: &Bound<'_, PyAny>) -> PyResult<()> {
        dispatch::dispatch(slf, value, &mut HashSet::new())
    }

    /// Append text without inserting line breaks within it.
    ///
    /// Wraps before the text if needed; oversized text overflows intact.
    /// Existing line endings are preserved, not escaped.
    ///
    /// :param text: Text to emit as one unit. Empty text emits nothing.
    /// :raises OSError: If text exceeds 65,535 Unicode characters, before output.
    /// :returns: None on success; writer errors propagate as exceptions.
    fn no_break(&mut self, text: &str) -> PyResult<()> {
        self.inner.no_break(text).map_err(PyErr::from)
    }

    /// Finish the dump, emit pending text, and flush the Python stream.
    ///
    /// The stream remains open. After successful completion, repeated calls do
    /// nothing and no further text may be added.
    fn dump_all(&mut self) -> PyResult<()> {
        self.inner.dump_all().map_err(PyErr::from)
    }
}
