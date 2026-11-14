//! Define aligned contexts and produce their content through closures.
use std::io::{Error, ErrorKind, Result, Write};

use crate::core::Dumper;
use crate::operations::{FromBytes, Operation, ToBytes, put_optional_text};
use bytes::{BufMut, BytesMut};

/// An aligned context with an optional prefix and name.
///
/// Use [`Self::with`] to produce the context's content. Continuation lines align
/// with the content after the prefix.
#[derive(Debug)]
pub struct Context<'data> {
    /// Text placed before the context's content.
    pub prefix: Option<&'data str>,
    /// Identifier for selecting the context's prologue or epilogue.
    pub name: Option<&'data str>,
}

impl<'data> Context<'data> {
    /// Produce this context's content through a closure.
    ///
    /// Closes the context after the closure returns, including on error, but not
    /// if it panics. The closure must close any contexts it opens.
    ///
    /// # Arguments
    ///
    /// * `dumper` — Destination for the formatted output.
    /// * `body` — Closure that produces the context's content.
    ///
    /// # Returns
    ///
    /// Returns the closure's value on success, or an error from opening the
    /// context, the closure, or closing the context. The closure's error takes
    /// precedence over a closing error.
    ///
    /// # Examples
    ///
    /// ```
    /// use dumpany::context::Context;
    /// use dumpany::core::{Dumper, DumperConfig};
    ///
    /// let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
    /// Context::new(Some("- "), None).with(&mut out, |out| {
    ///     out.text("first")?;
    ///     out.new_line(None)?;
    ///     out.text("second")
    /// })?;
    /// let bytes = out.into_inner().map_err(|error| error.into_parts().0)?;
    /// assert_eq!(bytes, b"- first\n  second\n");
    /// # Ok::<(), std::io::Error>(())
    /// ```
    pub fn with<W: Write, R>(
        &self,
        dumper: &mut Dumper<W>,
        body: impl FnOnce(&mut Dumper<W>) -> Result<R>,
    ) -> Result<R> {
        dumper.open_context(self.prefix, self.name)?;
        let result = body(dumper);
        let closed = dumper.close_context(None);
        match result {
            Ok(value) => closed.map(|()| value),
            Err(error) => Err(error),
        }
    }

    /// Describe a context to use with [`Self::with`].
    ///
    /// # Arguments
    ///
    /// * `prefix` — Optional text before the content; must fit on the current line.
    /// * `name` — Optional identifier for selecting a prologue or epilogue. Must
    ///   be unique until the last open main context closes.
    ///
    /// # Returns
    ///
    /// Returns a context description without opening it in a dumper.
    pub fn new(prefix: Option<&'data str>, name: Option<&'data str>) -> Self {
        Self { prefix, name }
    }
}

impl<'data> FromBytes<'data> for Context<'data> {
    fn from_bytes(input: &mut &'data [u8]) -> Result<Self> {
        let mut remaining = *input;
        match Operation::from_bytes(&mut remaining)? {
            Operation::OpenContext(context) => {
                *input = remaining;
                Ok(context)
            }
            _ => Err(Error::new(
                ErrorKind::InvalidData,
                "expected context opening",
            )),
        }
    }
}

impl ToBytes for Context<'_> {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()> {
        out.put_u8(crate::operations::OPEN_CONTEXT);
        put_optional_text(out, self.prefix);
        put_optional_text(out, self.name);
        Ok(())
    }
}
