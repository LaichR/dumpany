//! Context metadata and scoped formatting.
use std::io::{Error, ErrorKind, Result, Write};

use crate::core::Dumper;
use crate::operations::{FromBytes, Operation, ToBytes, put_optional_text};
use bytes::{BufMut, BytesMut};

/// An aligned context with an optional prefix and name.
#[derive(Debug)]
pub struct Context<'data> {
    pub prefix: Option<&'data str>,
    pub name: Option<&'data str>,
}

impl<'data> Context<'data> {
    /// Run a closure inside this context, closing it even if the closure fails.
    /// If both fail, the closure's error takes precedence. Panics are not handled.
    /// The closure must leave any contexts it opens balanced.
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

    /// Context metadata for recording. Only name and prefix are encoded.
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
