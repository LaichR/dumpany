//! Single-pass context traversal over a completed recorder buffer.
use std::io::{Error, ErrorKind, Result};

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
mod tests {
    use super::*;
    use crate::core::Text;
    use crate::recorder::Recorder;

    fn collect(context: &mut Context<'_, '_>, text: &mut String) -> Result<()> {
        while let Some(event) = context.next_event()? {
            match event {
                Operation::Text(value) => text.push_str(value.0),
                Operation::OpenContext(mut child) => collect(&mut child, text)?,
                _ => {}
            }
        }
        Ok(())
    }

    fn recording() -> Result<Recorder> {
        let mut recorder = Recorder::new(None, Some("root"))?;
        recorder.text("before")?;
        recorder.open_context(Some("- "), Some("child"))?;
        recorder.text("inside")?;
        recorder.open_context(None, Some("grandchild"))?;
        recorder.text("nested")?;
        recorder.close_context()?;
        recorder.text("tail")?;
        recorder.close_context()?;
        recorder.text("after")?;
        recorder.close_context()?;
        Ok(recorder)
    }

    #[test]
    fn recursively_consumes_contexts() -> Result<()> {
        let recorder = recording()?;
        let mut cursor = Cursor::new(recorder.encoded_content());
        let mut root = Context::from_encoding(&mut cursor)?;
        assert_eq!(root.name, Some("root"));
        let mut text = String::new();
        collect(&mut root, &mut text)?;
        assert_eq!(text, "beforeinsidenestedtailafter");
        drop(root);
        cursor.ensure_exhausted()?;
        Ok(())
    }

    #[test]
    fn dropped_child_skips_nested_remainder() -> Result<()> {
        let recorder = recording()?;
        let mut cursor = Cursor::new(recorder.encoded_content());
        let mut root = Context::from_encoding(&mut cursor)?;
        assert!(matches!(
            root.next_event()?,
            Some(Operation::Text(Text("before")))
        ));
        match root.next_event()? {
            Some(Operation::OpenContext(mut child)) => {
                assert_eq!(child.prefix, Some("- "));
                assert!(matches!(
                    child.next_event()?,
                    Some(Operation::Text(Text("inside")))
                ));
            }
            _ => panic!("expected child"),
        }
        assert!(matches!(
            root.next_event()?,
            Some(Operation::Text(Text("after")))
        ));
        assert!(root.next_event()?.is_none());
        drop(root);
        cursor.ensure_exhausted()?;
        Ok(())
    }

    #[test]
    fn reports_errors_during_drop_to_parent() -> Result<()> {
        let mut recorder = Recorder::new(None, None)?;
        recorder.open_context(None, None)?;
        recorder.text("unclosed")?;
        let mut cursor = Cursor::new(recorder.encoded_content());
        let mut root = Context::from_encoding(&mut cursor)?;
        match root.next_event()? {
            Some(Operation::OpenContext(child)) => drop(child),
            _ => panic!("expected child"),
        }
        assert_eq!(
            root.next_event().unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        assert_eq!(
            root.next_event().unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        Ok(())
    }

    #[test]
    fn rejects_trailing_content() -> Result<()> {
        let mut recorder = Recorder::new(None, None)?;
        recorder.close_context()?;
        recorder.text("trailing")?;
        let mut cursor = Cursor::new(recorder.encoded_content());
        let mut root = Context::from_encoding(&mut cursor)?;
        assert!(root.next_event()?.is_none());
        drop(root);
        assert_eq!(
            cursor.ensure_exhausted().unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        assert_eq!(
            cursor.ensure_exhausted().unwrap_err().kind(),
            ErrorKind::InvalidData
        );
        Ok(())
    }

    #[test]
    fn retains_errors_from_direct_decoding() -> Result<()> {
        let mut recorder = Recorder::new(None, None)?;
        recorder.text("unclosed")?;
        let mut cursor = Cursor::new(recorder.encoded_content());
        let mut root = Context::from_encoding(&mut cursor)?;
        assert!(matches!(
            root.next_event()?,
            Some(Operation::Text(Text("unclosed")))
        ));
        let first = root.next_event().unwrap_err();
        let repeated = root.next_event().unwrap_err();
        assert_eq!(first.kind(), ErrorKind::UnexpectedEof);
        assert_eq!(repeated.kind(), first.kind());
        assert_eq!(repeated.to_string(), first.to_string());
        Ok(())
    }
}
