//! Records one root context and its nested operations into a single byte buffer.
//! The caller owns lifecycle validation: close only open contexts and stop
//! recording when the root closes.
use bytes::BytesMut;
use std::fmt::Display;
use std::io::Result;

use crate::context::Context;
use crate::core::{Indentation, NewLine, Text};
use crate::operations::{CloseContext, ToBytes};

pub struct Recorder {
    encoded_content: BytesMut,
    context_depth: usize,
}

impl Recorder {
    /// Start recording by encoding the root context immediately.
    pub fn new(prefix: Option<&'static str>, name: Option<&'static str>) -> Result<Self> {
        let mut recorder = Self {
            encoded_content: BytesMut::new(),
            context_depth: 0,
        };
        recorder.open_context(prefix, name)?;
        Ok(recorder)
    }

    pub fn text<T: Display>(&mut self, text: T) -> Result<()> {
        Text(text).to_bytes(&mut self.encoded_content)
    }

    pub fn new_line(&mut self, change_indentation: Option<Indentation>) -> Result<()> {
        NewLine(change_indentation).to_bytes(&mut self.encoded_content)
    }

    pub fn inc_indent(&mut self) -> Result<()> {
        Indentation::Increment(1).to_bytes(&mut self.encoded_content)
    }

    pub fn dec_indent(&mut self) -> Result<()> {
        Indentation::Decrement(1).to_bytes(&mut self.encoded_content)
    }

    pub fn open_context(
        &mut self,
        prefix: Option<&'static str>,
        name: Option<&'static str>,
    ) -> Result<()> {
        Context::new(prefix, name).to_bytes(&mut self.encoded_content)?;
        self.context_depth += 1;
        Ok(())
    }

    /// The caller must ensure a context is open before closing it.
    pub fn close_context(&mut self) -> Result<()> {
        CloseContext.to_bytes(&mut self.encoded_content)?;
        self.context_depth -= 1;
        Ok(())
    }

    pub fn context_depth(&self) -> usize {
        self.context_depth
    }

    pub fn encoded_content(&self) -> &[u8] {
        &self.encoded_content
    }

    /// Transfer the buffer without copying. The caller ensures the root is closed.
    pub fn into_encoded_content(self) -> BytesMut {
        self.encoded_content
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::operations::{FromBytes, Operation};

    #[test]
    fn records_root_and_basic_operations() -> Result<()> {
        let mut recorder = Recorder::new(Some("- "), Some("root"))?;
        assert_eq!(recorder.context_depth(), 1);
        recorder.text("hello λ")?;
        recorder.inc_indent()?;
        recorder.text(42)?;
        recorder.new_line(Some(Indentation::Decrement(1)))?;
        recorder.dec_indent()?;
        recorder.new_line(None)?;
        recorder.close_context()?;
        assert_eq!(recorder.context_depth(), 0);

        let buffer = recorder.into_encoded_content();
        let mut input = buffer.as_ref();
        match Operation::from_bytes(&mut input)? {
            Operation::OpenContext(context) => {
                assert_eq!((context.prefix, context.name), (Some("- "), Some("root")));
            }
            _ => panic!("expected root context"),
        }
        assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("hello λ"));
        assert_eq!(
            Indentation::from_bytes(&mut input)?,
            Indentation::Increment(1)
        );
        assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("42"));
        assert_eq!(
            NewLine::from_bytes(&mut input)?,
            NewLine(Some(Indentation::Decrement(1)))
        );
        assert_eq!(
            Indentation::from_bytes(&mut input)?,
            Indentation::Decrement(1)
        );
        assert_eq!(NewLine::from_bytes(&mut input)?, NewLine(None));
        CloseContext::from_bytes(&mut input)?;
        assert!(input.is_empty());
        Ok(())
    }

    #[test]
    fn records_nested_contexts_inline() -> Result<()> {
        let mut recorder = Recorder::new(None, Some("root"))?;
        recorder.text("before")?;
        recorder.open_context(Some("* "), Some("child"))?;
        assert_eq!(recorder.context_depth(), 2);
        recorder.text("inside")?;
        recorder.close_context()?;
        assert_eq!(recorder.context_depth(), 1);
        recorder.text("after")?;
        recorder.close_context()?;
        assert_eq!(recorder.context_depth(), 0);

        let mut input = recorder.encoded_content();
        Context::from_bytes(&mut input)?;
        assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("before"));
        let context = Context::from_bytes(&mut input)?;
        assert_eq!((context.prefix, context.name), (Some("* "), Some("child")));
        assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("inside"));
        CloseContext::from_bytes(&mut input)?;
        assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("after"));
        CloseContext::from_bytes(&mut input)?;
        assert!(input.is_empty());
        Ok(())
    }
}
