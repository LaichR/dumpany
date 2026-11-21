//! Internal recording support for the formatter.
use bytes::BytesMut;
use std::fmt::Display;
use std::io::Result;

use crate::context::Context;
use crate::core::{Indentation, NewLine, NoBreak, Text};
use crate::operations::{CloseContext, ToBytes};

#[derive(Default)]
pub(crate) struct Recorder {
    encoded_content: BytesMut,
    context_depth: usize,
}

impl Recorder {
    pub fn no_break<T: AsRef<str>>(&mut self, text: T) -> Result<()> {
        NoBreak(text).to_bytes(&mut self.encoded_content)
    }

    pub fn text<T: Display>(&mut self, text: T) -> Result<()> {
        Text(text).to_bytes(&mut self.encoded_content)
    }

    pub fn new_line(&mut self, change_indentation: Option<Indentation>) -> Result<()> {
        NewLine(change_indentation).to_bytes(&mut self.encoded_content)
    }

    /// Record an increase of count indentation levels as one operation.
    pub fn inc_indent(&mut self, count: u8) -> Result<()> {
        Indentation::Increment(count).to_bytes(&mut self.encoded_content)
    }

    /// Record a decrease of count indentation levels as one operation.
    pub fn dec_indent(&mut self, count: u8) -> Result<()> {
        Indentation::Decrement(count).to_bytes(&mut self.encoded_content)
    }

    pub fn open_context(&mut self, prefix: Option<&str>, name: Option<&str>) -> Result<()> {
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
}

#[cfg(test)]
impl Recorder {
    /// Start recording by encoding the root context immediately.
    pub fn new(prefix: Option<&str>, name: Option<&str>) -> Result<Self> {
        let mut recorder = Self::default();
        recorder.open_context(prefix, name)?;
        Ok(recorder)
    }

    /// Transfer the buffer without copying.
    pub fn into_encoded_content(self) -> BytesMut {
        self.encoded_content
    }
}

#[cfg(test)]
mod tests;
