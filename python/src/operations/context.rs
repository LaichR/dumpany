use dumpany_rs::core::Indentation;
use pyo3::class::gc::{PyTraverseError, PyVisit};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
/// A dumper operation that groups content into an independently aligned text block.
///
/// A context controls the left alignment of its content independently of the
/// dumper's normal indentation. An optional prefix appears before the content;
/// subsequent lines align with the content rather than repeating the prefix.
///
/// A named context can also receive prologue and epilogue content, which appears
/// before and after its main content regardless of when those operations are
/// dumped. Contexts can be nested recursively; prologue and epilogue content can
/// be added only while the outermost context remains open.
///
/// The context's content is dumped recursively.
/// Closing restores the enclosing alignment and ends the line unless new_line=False.
/// Normal indentation is left unchanged unless increment or decrement is supplied;
/// the requested change takes effect after the context's final line.
#[pyclass(name = "Context", module = "dumpany")]
pub struct PyContext {
    pub(crate) prefix: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) content: Option<Py<PyAny>>,
    pub(crate) indentation: Option<Indentation>,
    pub(crate) new_line: bool,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyContext {
    /// Create a new context.
    ///
    /// :param content: Any value accepted by Dumper.dump(). Consumed only when
    ///     the context is dumped.
    /// :param prefix: Optional text emitted once before the content. Subsequent
    ///     lines align after this prefix rather than repeating it.
    /// :param name: Optional name used by Prologue and Epilogue to target this
    ///     context. Names must be unique within the outermost context until it
    ///     is rendered.
    /// :param increment: Number of normal indentation levels to add on closing.
    ///     Defaults to None (no increment).
    /// :param decrement: Number of normal indentation levels to remove on closing,
    ///     stopping at zero. Defaults to None (no decrement).
    /// :param new_line: End the line on closing. Defaults to True. Supplying
    ///     increment or decrement always ends the line, including a zero count.
    ///
    /// Counts must be integers from 0 to 255. Supplying both increment and
    /// decrement raises ValueError, even when either count is zero. Indentation
    /// changes made inside the context are not automatically undone on closing.
    #[new]
    #[pyo3(signature = (content, *, prefix=None, name=None, increment=None, decrement=None, new_line=true))]
    fn new(
        content: Py<PyAny>,
        prefix: Option<String>,
        name: Option<String>,
        increment: Option<u8>,
        decrement: Option<u8>,
        new_line: bool,
    ) -> PyResult<Self> {
        Ok(Self {
            prefix,
            name,
            content: Some(content),
            indentation: super::indentation(increment, decrement)?,
            new_line,
        })
    }
}

#[pymethods]
impl PyContext {
    /// Expose the Python content reference to the garbage collector.
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(content) = &self.content {
            visit.call(content)?;
        }
        Ok(())
    }

    /// Break unreachable cycles passing through the context's content.
    fn __clear__(&mut self) {
        self.content = None;
    }
}
