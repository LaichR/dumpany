//! Single-pass context traversal over a completed recorder buffer.
use std::io::{Error, ErrorKind, Result, Write};

use crate::core::Dumper;
use crate::operations::{FromBytes, Operation, ToBytes, put_optional_text};
use bytes::{BufMut, BytesMut};

#[derive(Debug)]
pub(crate) struct Cursor<'a> {
    remaining: &'a [u8],
    error: Option<Error>,
}

#[allow(dead_code)]
impl<'a> Cursor<'a> {
    pub(crate) fn new(encoded_content: &'a [u8]) -> Self {
        Self {
            remaining: encoded_content,
            error: None,
        }
    }

    /// Called by the replay owner after the root context has been consumed.
    pub(crate) fn ensure_exhausted(&mut self) -> Result<()> {
        if self.error.is_none() && !self.remaining.is_empty() {
            self.error = Some(Error::new(
                ErrorKind::InvalidData,
                "content follows root context",
            ));
        }
        match &self.error {
            Some(error) => Err(Error::new(error.kind(), error.to_string())),
            None => Ok(()),
        }
    }
}

/// A live context. A returned child exclusively borrows the traversal cursor
/// until it is consumed or dropped. The encoded buffer is never copied.
#[derive(Debug)]
pub struct Context<'parent, 'data> {
    pub prefix: Option<&'data str>,
    pub name: Option<&'data str>,
    cursor: Option<&'parent mut Cursor<'data>>,
}

impl Context<'static, 'static> {
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
        let closed = dumper.close_context();
        match result {
            Ok(value) => closed.map(|()| value),
            Err(error) => Err(error),
        }
    }

    /// Context metadata for recording. Only name and prefix are encoded.
    pub fn new(prefix: Option<&'static str>, name: Option<&'static str>) -> Self {
        Self {
            prefix,
            name,
            cursor: None,
        }
    }
}

impl<'parent, 'data: 'parent> Context<'parent, 'data> {
    /// Create a new context from encoded buffer; unlike 'new' name and suffix
    /// do not have static lifetime
    pub(crate) fn from_metadata(prefix: Option<&'data str>, name: Option<&'data str>) -> Self {
        Self {
            prefix,
            name,
            cursor: None,
        }
    }
}

impl<'parent, 'data: 'parent> FromBytes<'data> for Context<'parent, 'data> {
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

impl ToBytes for Context<'_, '_> {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()> {
        out.put_u8(crate::operations::OPEN_CONTEXT);
        put_optional_text(out, self.prefix);
        put_optional_text(out, self.name);
        Ok(())
    }
}

impl<'parent, 'data: 'parent> Context<'parent, 'data> {
    /// Borrow a caller-owned replay cursor and consume the opening record.
    #[allow(dead_code)] // Retained for traversal of standalone recorder buffers.
    pub(crate) fn from_encoding(cursor: &'parent mut Cursor<'data>) -> Result<Self> {
        if let Some(error) = &cursor.error {
            return Err(Error::new(error.kind(), error.to_string()));
        }
        let mut context = match Context::from_bytes(&mut cursor.remaining) {
            Ok(context) => context,
            Err(error) => {
                let reported = Error::new(error.kind(), error.to_string());
                cursor.error = Some(error);
                return Err(reported);
            }
        };
        context.cursor = Some(cursor);
        Ok(context)
    }
}

impl<'data> Context<'_, 'data> {
    /// Closing records terminate this context and are not yielded.
    /// Dropping a returned child skips its unconsumed body before resuming here.
    /// Traversal errors remain stored and are reported on every subsequent call.
    /// The caller must stop calling after this returns `Ok(None)`.
    pub fn next_event(&mut self) -> Result<Option<Operation<'_, 'data>>> {
        let cursor = self
            .cursor
            .as_deref_mut()
            .expect("No traversal cursor available");
        if let Some(error) = &cursor.error {
            return Err(Error::new(error.kind(), error.to_string()));
        }
        let operation = match Operation::from_bytes(&mut cursor.remaining) {
            Ok(operation) => operation,
            Err(error) => {
                let reported = Error::new(error.kind(), error.to_string());
                cursor.error = Some(error);
                return Err(reported);
            }
        };
        Ok(match operation {
            Operation::Text(value) => Some(Operation::Text(value)),
            Operation::NewLine(value) => Some(Operation::NewLine(value)),
            Operation::Indentation(value) => Some(Operation::Indentation(value)),
            Operation::OpenContext(mut context) => {
                context.cursor = Some(
                    self.cursor
                        .as_deref_mut()
                        .expect("No traversal cursor available"),
                );
                Some(Operation::OpenContext(context))
            }
            Operation::CloseContext(_) => {
                self.cursor = None;
                None
            }
        })
    }
}

impl Drop for Context<'_, '_> {
    fn drop(&mut self) {
        let Some(cursor) = self.cursor.take() else {
            return;
        };
        if cursor.error.is_some() {
            return;
        }
        let mut depth = 1;
        while depth > 0 {
            match Operation::from_bytes(&mut cursor.remaining) {
                Ok(Operation::OpenContext(_)) => depth += 1,
                Ok(Operation::CloseContext(_)) => depth -= 1,
                Ok(_) => {}
                Err(error) => {
                    cursor.error = Some(error);
                    return;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
