use pyo3::class::gc::{PyTraverseError, PyVisit};
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

#[gen_stub_pyclass]
/// Dump composed content without automatic line breaks inside it.
///
/// Starts at the current position without inserting a break before the region.
/// Oversized lines overflow intact. Regions nest and honor explicit
/// newlines, including context closing breaks. Contexts must close inside the
/// region; target switching and finalization inside a region are not allowed.
/// An error while producing a region permanently fails the dumper. Pending
/// output is discarded; bytes already written cannot be undone.
#[pyclass(module = "dumpany")]
pub struct NoBreakRegion {
    pub(crate) content: Option<Py<PyAny>>,
}

#[gen_stub_pymethods]
#[pymethods]
impl NoBreakRegion {
    /// Create a no-break region.
    ///
    /// :param content: Any value accepted by Dumper.dump(), including iterators.
    ///     Consumed only when dumped; iterators and callbacks run once.
    #[new]
    fn new(content: Py<PyAny>) -> Self {
        Self {
            content: Some(content),
        }
    }
}

#[pymethods]
impl NoBreakRegion {
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(content) = &self.content {
            visit.call(content)?;
        }
        Ok(())
    }

    fn __clear__(&mut self) {
        self.content = None;
    }
}
