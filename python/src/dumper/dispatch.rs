use super::PyDumper;
use crate::operations::{Join, NewLine};
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
    if value.is_instance_of::<Join>() {
        return with_active(value, active, |active| dump_join(dumper, value, active));
    }
    if is_atomic(value) {
        return dump_text(dumper, value);
    }
    let ty = value.get_type();
    if ty.hasattr("__iter__")? || ty.hasattr("__getitem__")? {
        return with_active(value, active, |active| dump_iterable(dumper, value, active));
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

fn with_active(
    value: &Bound<'_, PyAny>,
    active: &mut HashSet<usize>,
    body: impl FnOnce(&mut HashSet<usize>) -> PyResult<()>,
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
    let result = body(active);
    active.remove(&identity);
    result
}

fn dump_iterable(
    dumper: &Bound<'_, PyDumper>,
    value: &Bound<'_, PyAny>,
    active: &mut HashSet<usize>,
) -> PyResult<()> {
    // Do not borrow the dumper while advancing user-defined Python iterators.
    for item in value.try_iter()? {
        dispatch(dumper, &item?, active)?;
    }
    Ok(())
}

fn dump_join(
    dumper: &Bound<'_, PyDumper>,
    value: &Bound<'_, PyAny>,
    active: &mut HashSet<usize>,
) -> PyResult<()> {
    let py = value.py();
    // Release the operation borrow before executing Python callbacks.
    let (delimiter, iterable) = {
        let operation = value.cast::<Join>()?.borrow();
        let (delimiter, iterable) = operation
            .contents
            .as_ref()
            .ok_or_else(|| PyValueError::new_err("Join has been cleared"))?;
        (delimiter.clone_ref(py), iterable.clone_ref(py))
    };
    with_active(iterable.bind(py), active, |active| {
        let mut items = iterable.bind(py).try_iter()?;
        if let Some(first) = items.next() {
            dispatch(dumper, &first?, active)?;
            for item in items {
                let item = item?;
                dispatch(dumper, delimiter.bind(py), active)?;
                dispatch(dumper, &item, active)?;
            }
        }
        Ok(())
    })
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
