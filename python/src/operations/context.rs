use pyo3::class::gc::{PyTraverseError, PyVisit};
use pyo3::prelude::*;

/// Align a text block independently of normal indentation, with an optional prefix.
/// Continuation lines align after the prefix rather than repeating it.
#[pyclass(name = "Context", module = "dumpany")]
pub struct PyContext {
    pub(crate) prefix: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) content: Option<Py<PyAny>>,
}

#[pymethods]
impl PyContext {
    #[new]
    #[pyo3(signature = (content, *, prefix=None, name=None))]
    fn new(content: Py<PyAny>, prefix: Option<String>, name: Option<String>) -> Self {
        Self {
            prefix,
            name,
            content: Some(content),
        }
    }

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
