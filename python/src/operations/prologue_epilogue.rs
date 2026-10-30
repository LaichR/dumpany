use pyo3::class::gc::{PyTraverseError, PyVisit};
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

/// Shared content and Python reference management for target operations.
pub(crate) struct TargetContent {
    name: String,
    content: Option<Py<PyAny>>,
}

impl TargetContent {
    fn new(name: String, content: Py<PyAny>) -> Self {
        Self {
            name,
            content: Some(content),
        }
    }

    /// Copy references before dispatch so no operation borrow spans Python callbacks.
    pub(crate) fn snapshot(&self, py: Python<'_>) -> PyResult<(String, Py<PyAny>)> {
        let content = self
            .content
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("target operation has been cleared"))?;
        Ok((self.name.clone(), content.clone_ref(py)))
    }

    fn traverse(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        if let Some(content) = &self.content {
            visit.call(content)?;
        }
        Ok(())
    }

    fn clear(&mut self) {
        self.content = None;
    }
}

#[gen_stub_pyclass]
/// A dumper operation that adds content before the body of a named context.
///
/// The content appears before the context's main content, even if the prologue
/// is dumped after that content. Repeated prologues append in dumping order.
/// The named context must already exist, and the outermost context containing
/// it must remain open.
#[pyclass(module = "dumpany")]
pub struct Prologue {
    pub(crate) inner: TargetContent,
}

#[gen_stub_pymethods]
#[pymethods]
impl Prologue {
    /// Create a prologue for the context identified by name.
    ///
    /// Content is consumed only when the prologue is dumped. It accepts the same
    /// values and operations as Dumper.dump(), except that it must not contain
    /// another Prologue or Epilogue, even indirectly.
    ///
    /// :param name: Name of an existing context to prepend content to.
    /// :param content: Text or dump operations to place before that context's
    ///     main content. Consumed only when this operation is dumped.
    #[new]
    #[pyo3(signature = (name, content))]
    fn new(name: String, content: Py<PyAny>) -> Self {
        Self {
            inner: TargetContent::new(name, content),
        }
    }
}

#[pymethods]
impl Prologue {
    /// Expose content references so Python can detect unreachable cycles.
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        self.inner.traverse(visit)
    }

    /// Break unreachable cycles passing through this operation.
    fn __clear__(&mut self) {
        self.inner.clear();
    }
}

#[gen_stub_pyclass]
/// A dumper operation that adds content after the body of a named context.
///
/// The content appears after the context's main content, even if the epilogue
/// is dumped before that content. Repeated epilogues append in dumping order.
/// The named context must already exist, and the outermost context containing
/// it must remain open.
#[pyclass(module = "dumpany")]
pub struct Epilogue {
    pub(crate) inner: TargetContent,
}

#[gen_stub_pymethods]
#[pymethods]
impl Epilogue {
    /// Create an epilogue for the context identified by name.
    ///
    /// Content is consumed only when the epilogue is dumped. It accepts the same
    /// values and operations as Dumper.dump(), except that it must not contain
    /// another Prologue or Epilogue, even indirectly.
    ///
    /// :param name: Name of an existing context to append content to.
    /// :param content: Text or dump operations to place after that context's
    ///     main content. Consumed only when this operation is dumped.
    #[new]
    #[pyo3(signature = (name, content))]
    fn new(name: String, content: Py<PyAny>) -> Self {
        Self {
            inner: TargetContent::new(name, content),
        }
    }
}

#[pymethods]
impl Epilogue {
    /// Expose content references so Python can detect unreachable cycles.
    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        self.inner.traverse(visit)
    }

    /// Break unreachable cycles passing through this operation.
    fn __clear__(&mut self) {
        self.inner.clear();
    }
}
