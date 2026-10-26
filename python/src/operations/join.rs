use pyo3::class::gc::{PyTraverseError, PyVisit};
use pyo3::prelude::*;

/// Lazily dump iterable items with a delimiter between them.
#[pyclass(module = "dumpany")]
pub struct Join {
    pub(crate) contents: Option<(Py<PyAny>, Py<PyAny>)>,
}

#[pymethods]
impl Join {
    #[new]
    #[pyo3(signature = (delimiter, iterable))]
    fn new(delimiter: Py<PyAny>, iterable: Py<PyAny>) -> Self {
        Self {
            contents: Some((delimiter, iterable)),
        }
    }
    /// used for garbage collection
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some((delimiter, iterable)) = &self.contents {
            visit.call(delimiter)?;
            visit.call(iterable)?;
        }
        Ok(())
    }
    /// collect python objects
    fn __clear__(&mut self) {
        self.contents = None;
    }
}
