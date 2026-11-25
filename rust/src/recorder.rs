//! Internal recording support for the formatter.
use bytes::BytesMut;
use std::fmt::Display;
use std::io::Result;

use crate::context::Context;
use crate::core::{BreakPolicy, Indentation, NewLine, NoBreak, Text};
use crate::operations::{CloseContext, ToBytes};

/// Track when a nested operation must emit the shared recorder.
#[derive(Default)]
pub(crate) struct RecordingState {
    depth: usize,
    owns_recorder: bool,
}

impl RecordingState {
    /// Enter a scope, claiming the recorder only if none already exists.
    pub fn begin(&mut self, recorder_exists: bool) {
        if !self.is_active() {
            self.owns_recorder = !recorder_exists;
        }
        self.depth += 1;
    }

    /// Leave a scope and report whether its recorder should now be emitted.
    /// Unmatched endings are left for replay to reject.
    pub fn end(&mut self) -> bool {
        if !self.is_active() {
            return false;
        }
        self.depth -= 1;
        self.depth == 0 && std::mem::take(&mut self.owns_recorder)
    }

    pub fn is_active(&self) -> bool {
        self.depth > 0
    }
}

/// Buffer body, section, or region operations for later replay.
#[derive(Default)]
pub(crate) struct Recorder {
    /// Operations in output order, including inline join and region boundaries.
    encoded_content: BytesMut,
    /// Number of currently open contexts in this recording.
    context_depth: usize,
}

// Recording only captures operations; all layout decisions belong to replay.
impl Recorder {
    pub fn begin_join(&mut self, breaks: BreakPolicy) {
        crate::operations::put_join(&mut self.encoded_content, breaks);
    }
    pub fn join_part(&mut self) {
        crate::operations::put_join_part(&mut self.encoded_content);
    }
    pub fn end_join(&mut self) {
        crate::operations::put_join_end(&mut self.encoded_content);
    }
    /// Return whether this recording has a context available to close.
    pub fn can_close_context(&self) -> bool {
        self.context_depth > 0
    }

    pub fn begin_no_break_region(&mut self) {
        crate::operations::put_no_break_region(&mut self.encoded_content);
    }

    pub fn end_no_break_region(&mut self) {
        crate::operations::put_end_no_break_region(&mut self.encoded_content);
    }

    /// Record indivisible `text`; oversized text returns InvalidInput.
    pub fn no_break<T: AsRef<str>>(&mut self, text: T) -> Result<()> {
        NoBreak(text.as_ref()).to_bytes(&mut self.encoded_content)
    }

    /// Format and record `text` once, returning any formatting error.
    pub fn text<T: Display>(&mut self, text: T) -> Result<()> {
        Text(text).to_bytes(&mut self.encoded_content)
    }

    /// Record a break with optional `change_indentation` for following lines.
    /// Returns Ok after encoding the operation.
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

    /// Record a context with optional `prefix` text and section-target `name`.
    /// Returns Ok after encoding the opening operation.
    pub fn open_context(&mut self, prefix: Option<&str>, name: Option<&str>) -> Result<()> {
        Context::new(prefix, name).to_bytes(&mut self.encoded_content)?;
        self.context_depth += 1;
        Ok(())
    }

    /// Record a context close, ending its line when `new_line` is true.
    /// Returns Ok after encoding. The caller must first check can_close_context.
    pub fn close_context(&mut self, new_line: bool) -> Result<()> {
        CloseContext(new_line).to_bytes(&mut self.encoded_content)?;
        self.context_depth -= 1;
        Ok(())
    }

    /// Return the number of contexts still open in this recording.
    pub fn context_depth(&self) -> usize {
        self.context_depth
    }

    /// Borrow the encoded operations.
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
