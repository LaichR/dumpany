use pyo3::prelude::*;
use std::io::{self, Write};

/// Adapts a Python text stream to the byte-oriented writer used by the Dumper.
///
/// Each write must contain complete UTF-8, as the Dumper's writes do. The stream
/// must implement `write(str) -> int` (a character count) and `flush()`. It is
/// kept alive by this adapter but is never closed or implicitly flushed on drop.
/// Python exceptions are retained as the source of an I/O error.
pub struct PythonWriter {
    stream: Py<PyAny>,
}

impl PythonWriter {
    pub fn new(stream: Py<PyAny>) -> Self {
        Self { stream }
    }
}

impl Write for PythonWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = std::str::from_utf8(buf)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if text.is_empty() {
            return Ok(0);
        }
        Python::attach(|py| {
            let written = self
                .stream
                .bind(py)
                .call_method1("write", (text,))
                .map_err(io::Error::other)?
                .extract::<usize>()
                .map_err(io::Error::other)?;

            // Python reports characters, whereas Write reports bytes. In
            // particular, a short write must leave the next call on a UTF-8 boundary.
            let mut chars = text.chars();
            let mut bytes = 0;
            for _ in 0..written {
                let ch = chars.next().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Python stream.write() returned more characters than supplied",
                    )
                })?;
                bytes += ch.len_utf8();
            }
            Ok(bytes)
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Python::attach(|py| {
            self.stream
                .bind(py)
                .call_method0("flush")
                .map(|_| ())
                .map_err(io::Error::other)
        })
    }
}

#[cfg(test)]
mod tests;
