//! Encodes operations into a byte buffer. Lifecycle and routing belong to the caller.
use bytes::BytesMut;
use std::fmt::Display;
use std::io::Result;

use crate::context::Context;
use crate::core::{Indentation, NewLine, Text};
use crate::operations::{CloseContext, ToBytes};

#[derive(Default)]
pub struct Recorder {
    encoded_content: BytesMut,
    context_depth: usize,
}

impl Recorder {
    /// Start recording by encoding the root context immediately.
    pub fn new(prefix: Option<&'static str>, name: Option<&'static str>) -> Result<Self> {
        let mut recorder = Self::default();
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

    /// Transfer the buffer without copying.
    pub fn into_encoded_content(self) -> BytesMut {
        self.encoded_content
    }
}

#[cfg(test)]
mod tests;
