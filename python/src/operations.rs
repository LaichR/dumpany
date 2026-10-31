mod context;
mod join;
mod new_line;
mod prologue_epilogue;

pub(crate) use context::PyContext;
pub(crate) use join::Join;
pub(crate) use new_line::NewLine;
pub(crate) use prologue_epilogue::{Epilogue, Prologue};

fn indentation(
    increment: Option<u8>,
    decrement: Option<u8>,
) -> pyo3::PyResult<Option<dumpany_rs::core::Indentation>> {
    use dumpany_rs::core::Indentation;
    match (increment, decrement) {
        (Some(_), Some(_)) => Err(pyo3::exceptions::PyValueError::new_err(
            "specify either increment or decrement, not both",
        )),
        (Some(count), None) => Ok(Some(Indentation::Increment(count))),
        (None, Some(count)) => Ok(Some(Indentation::Decrement(count))),
        (None, None) => Ok(None),
    }
}
