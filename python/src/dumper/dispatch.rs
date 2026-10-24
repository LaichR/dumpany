use super::PyDumper;
use crate::operations::NewLine;
use dumpany::core::Indentation;
use pyo3::exceptions::{PyRecursionError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyByteArray, PyBytes, PyMemoryView, PyString};
use std::collections::HashSet;

const MAX_NESTING: usize = 100;

pub(super) fn dispatch(
    dumper: &Bound<'_, PyDumper>,
    value: &Bound<'_, PyAny>,
    active: &mut HashSet<usize>,
) -> PyResult<()> {
    if value.is(value.py().Ellipsis()) {
        return dump_newline(dumper, None);
    }
    if let Ok(operation) = value.cast::<NewLine>() {
        return dump_newline(dumper, operation.borrow().indentation);
    }
    if is_atomic(value) {
        return dump_text(dumper, value);
    }
    let ty = value.get_type();
    if ty.hasattr("__iter__")? || ty.hasattr("__getitem__")? {
        return dump_iterable(dumper, value, active);
    }
    dump_text(dumper, value)
}

fn is_atomic(value: &Bound<'_, PyAny>) -> bool {
    value.is_instance_of::<PyString>()
        || value.is_instance_of::<PyBytes>()
        || value.is_instance_of::<PyByteArray>()
        || value.is_instance_of::<PyMemoryView>()
}

fn dump_newline(dumper: &Bound<'_, PyDumper>, indentation: Option<Indentation>) -> PyResult<()> {
    dumper
        .try_borrow_mut()?
        .inner
        .new_line(indentation)
        .map_err(PyErr::from)
}

fn dump_iterable(
    dumper: &Bound<'_, PyDumper>,
    value: &Bound<'_, PyAny>,
    active: &mut HashSet<usize>,
) -> PyResult<()> {
    let identity = value.as_ptr() as usize;
    if active.contains(&identity) {
        return Err(PyValueError::new_err(
            "cycle detected while dumping iterable",
        ));
    }
    if active.len() >= MAX_NESTING {
        return Err(PyRecursionError::new_err(
            "maximum dump nesting depth exceeded",
        ));
    }
    active.insert(identity);
    // Do not borrow the dumper while advancing user-defined Python iterators.
    let result = (|| {
        for item in value.try_iter()? {
            dispatch(dumper, &item?, active)?;
        }
        Ok(())
    })();
    active.remove(&identity);
    result
}

fn dump_text(dumper: &Bound<'_, PyDumper>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    // str() may execute Python callbacks; convert before borrowing the dumper.
    let text = value.str()?.to_str()?.to_owned();
    dumper
        .try_borrow_mut()?
        .inner
        .text(text)
        .map_err(PyErr::from)
}
