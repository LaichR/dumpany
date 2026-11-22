//! Internal recording support for the formatter.
use bytes::BytesMut;
use std::fmt::Display;
use std::io::Result;

use crate::context::Context;
use crate::core::{Indentation, NewLine, NoBreak, Text};
use crate::operations::{CloseContext, ToBytes};
use crate::operations::{FromBytes, Operation};
use std::collections::HashMap;

/// One contribution to a first-line width, kept in output order.
enum WidthPart {
    /// Already-counted Unicode characters in recorded text.
    Characters(usize),
    /// Index of a separately recorded region contributing to this first line.
    Region(usize),
    /// A section whose content may be supplied after the region is recorded.
    Section {
        /// Context name used to look up the section at replay time.
        name: String,
        /// Select the prologue when true, otherwise the epilogue.
        prologue: bool,
    },
}

/// Accumulate width until the first explicit break, deferring named sections.
#[derive(Default)]
struct FirstLine {
    /// Text widths and section references in their eventual output order.
    parts: Vec<WidthPart>,
    /// A known break was recorded; later content cannot affect this first line.
    ended: bool,
}

// Recording adds contributions; replay resolves sections that may have arrived later.
impl FirstLine {
    /// Count `text` up to its first `new_line` delimiter; ignore later lines.
    fn text(&mut self, text: &str, new_line: &str) {
        if self.ended {
            return;
        }
        let (first, ends) = match text.split_once(new_line) {
            Some((first, _)) => (first, true),
            None => (text, false),
        };
        let width = first.chars().count();
        if let Some(WidthPart::Characters(previous)) = self.parts.last_mut() {
            *previous += width;
        } else if width != 0 {
            self.parts.push(WidthPart::Characters(width));
        }
        self.ended = ends;
    }

    /// Add an optional context `name`; `prologue` selects its leading or trailing section.
    fn section(&mut self, name: Option<&str>, prologue: bool) {
        if !self.ended
            && let Some(name) = name
        {
            self.parts.push(WidthPart::Section {
                name: name.to_owned(),
                prologue,
            });
        }
    }

    /// Resolve `prologues` and `epilogues`, returning (character count, line ended).
    /// A break in a referenced section also stops counting the enclosing first line.
    fn resolve(
        &self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
        regions: &[Recorder],
    ) -> (usize, bool) {
        let mut width = 0;
        for part in &self.parts {
            match part {
                WidthPart::Characters(count) => width += count,
                WidthPart::Region(index) => {
                    let region = &regions[*index];
                    let (count, ended) =
                        region
                            .first_line
                            .resolve(prologues, epilogues, &region.regions);
                    width += count;
                    if ended {
                        return (width, true);
                    }
                }
                WidthPart::Section { name, prologue } => {
                    let sections = if *prologue { prologues } else { epilogues };
                    if let Some(section) = sections.get(name) {
                        let (count, ended) =
                            section
                                .first_line
                                .resolve(prologues, epilogues, &section.regions);
                        width += count;
                        if ended {
                            return (width, true);
                        }
                    }
                }
            }
        }
        (width, self.ended)
    }
}

/// Buffer operations for later replay and retain widths for no-break regions.
pub(crate) struct Recorder {
    /// Operations in output order; region markers refer to slots in `regions`.
    encoded_content: BytesMut,
    /// Number of currently open contexts in this recording.
    context_depth: usize,
    /// Names of open contexts, including None for unnamed contexts.
    context_names: Vec<Option<String>>,
    /// Completed region recordings, attached in output order and retained until replay.
    regions: Vec<Recorder>,
    /// First-line metadata for the entire recording, also used by named sections.
    first_line: FirstLine,
    /// Configured delimiter used to recognize explicit breaks in text.
    new_line: String,
}

// Test and internal convenience default: recognize LF as the line delimiter.
impl Default for Recorder {
    fn default() -> Self {
        Self::with_new_line("\n")
    }
}

// Each recording tracks its own width, whether it is a body, section, or region.
impl Recorder {
    /// Create an empty recording using the nonempty `new_line` delimiter.
    pub fn with_new_line(new_line: &str) -> Self {
        Self {
            encoded_content: BytesMut::new(),
            context_depth: 0,
            context_names: Vec::new(),
            regions: Vec::new(),
            first_line: FirstLine::default(),
            new_line: new_line.to_owned(),
        }
    }

    /// Return retained metadata indexed by the encoded region markers.
    pub fn regions(&self) -> &[Recorder] {
        &self.regions
    }

    /// Return whether this recording has a context available to close.
    pub fn can_close_context(&self) -> bool {
        self.context_depth > 0
    }

    /// Attach a completed `region`; its content and width remain owned by this recorder.
    pub fn no_break_region(&mut self, region: Recorder) {
        let index = self.regions.len();
        crate::operations::put_no_break_region(&mut self.encoded_content, index);
        if !self.first_line.ended {
            self.first_line.parts.push(WidthPart::Region(index));
        }
        self.regions.push(region);
    }

    /// Return the first-line character count using current `prologues` and
    /// `epilogues`; missing named sections contribute nothing.
    pub fn first_line_width(
        &self,
        prologues: &HashMap<String, Recorder>,
        epilogues: &HashMap<String, Recorder>,
    ) -> usize {
        self.first_line
            .resolve(prologues, epilogues, &self.regions)
            .0
    }
    /// Record indivisible `text` and its width; oversized text returns InvalidInput.
    pub fn no_break<T: AsRef<str>>(&mut self, text: T) -> Result<()> {
        NoBreak(text.as_ref()).to_bytes(&mut self.encoded_content)?;
        self.first_line.text(text.as_ref(), &self.new_line);
        Ok(())
    }

    /// Format `text` once and record its width; formatting errors are returned.
    pub fn text<T: Display>(&mut self, text: T) -> Result<()> {
        let start = self.encoded_content.len();
        Text(text).to_bytes(&mut self.encoded_content)?;
        // Count the already-formatted text; never invoke Display a second time.
        if let Operation::Text(text) = Operation::from_bytes(&mut &self.encoded_content[start..])? {
            self.first_line.text(text.0, &self.new_line);
        }
        Ok(())
    }

    /// Record an explicit break and optional `change_indentation` for following lines.
    /// Returns Ok after ending first-line width accumulation.
    pub fn new_line(&mut self, change_indentation: Option<Indentation>) -> Result<()> {
        NewLine(change_indentation).to_bytes(&mut self.encoded_content)?;
        self.first_line.ended = true;
        Ok(())
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
    /// Returns Ok after adding the prefix width and a deferred prologue reference.
    pub fn open_context(&mut self, prefix: Option<&str>, name: Option<&str>) -> Result<()> {
        Context::new(prefix, name).to_bytes(&mut self.encoded_content)?;
        self.context_depth += 1;
        self.first_line.text(prefix.unwrap_or(""), &self.new_line);
        self.first_line.section(name, true);
        self.context_names.push(name.map(str::to_owned));
        Ok(())
    }

    /// Record a context close; `new_line` also ends first-line width accumulation.
    /// Returns Ok after adding the epilogue reference. The caller must first check
    /// can_close_context so the close does not cross a region boundary.
    pub fn close_context(&mut self, new_line: bool) -> Result<()> {
        CloseContext(new_line).to_bytes(&mut self.encoded_content)?;
        self.context_depth -= 1;
        let name = self.context_names.pop().flatten();
        self.first_line.section(name.as_deref(), false);
        if new_line {
            self.first_line.ended = true;
        }
        Ok(())
    }

    /// Return the number of contexts still open in this recording.
    pub fn context_depth(&self) -> usize {
        self.context_depth
    }

    /// Borrow the encoded operations; region metadata remains in this recorder.
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
