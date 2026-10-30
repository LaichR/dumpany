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
#[pyclass(name = "Context", module = "dumpany")]
pub struct PyContext {
    pub(crate) prefix: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) content: Option<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl PyContext {
    /// Create a new context.
    ///
    /// prefix is emitted once and establishes continuation-line alignment.
    /// name optionally registers a target for Prologue and Epilogue; names must
    /// be unique within the outermost context until it is rendered.
    #[new]
    #[pyo3(signature = (content, *, prefix=None, name=None))]
    fn new(content: Py<PyAny>, prefix: Option<String>, name: Option<String>) -> Self {
        Self {
            prefix,
            name,
            content: Some(content),
        }
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
