//! Formatting operations and conversion between operations and bytes.
//!
//! Use [`ToBytes`] to encode operations and [`FromBytes`] to decode them.
use bytes::{BufMut, BytesMut};
use std::fmt::{self, Display, Write as FmtWrite};
use std::io::{Error, ErrorKind, Result, Write};

use crate::context::Context;
use crate::core::{Dump, Dumper, Indentation, NewLine, NoBreak, Text};

/// Close the current context without changing indentation as a [`Dump`] operation.
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub(crate) struct CloseContext(pub bool);

impl Dump for Context<'static> {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.open_context(self.prefix, self.name)
    }
}

impl Dump for CloseContext {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.close_context(None, self.0)
    }
}

/// Enables an operation to be encoded as bytes.
pub(crate) trait ToBytes {
    /// Append this operation to an encoded sequence.
    ///
    /// # Arguments
    ///
    /// * `out` — Buffer receiving the operation after its existing contents.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or an error if encoding fails, such as when
    /// a text value's [`Display`] implementation fails.
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()>;
}

/// Enables an operation to be decoded from bytes.
pub(crate) trait FromBytes<'a>: Sized {
    /// Decode the next operation of the expected type.
    ///
    /// # Arguments
    ///
    /// * `input` — Encoded sequence. Advances past the operation on success and
    ///   remains unchanged on error.
    ///
    /// # Returns
    ///
    /// Returns the decoded operation, borrowing text from the input where needed.
    /// Returns [`ErrorKind::UnexpectedEof`] for incomplete input or
    /// [`ErrorKind::InvalidData`] for invalid data or an unexpected operation type.
    fn from_bytes(input: &mut &'a [u8]) -> Result<Self>;
}

/// Represent any supported formatting operation decoded from bytes.
///
/// Text and context descriptions borrow their strings from the input.
#[derive(Debug)]
pub(crate) enum Operation<'data> {
    /// Begin an inline join; alternating item/delimiter parts end at JoinEnd.
    Join(crate::core::BreakPolicy),
    JoinPart,
    JoinEnd,
    /// Render encoded operations without automatic line breaks.
    NoBreakRegion,
    EndNoBreakRegion,
    /// Append indivisible text.
    NoBreak {
        text: &'data str,
        length: u16,
    },
    /// Append the supplied text.
    Text(Text<&'data str>),
    /// Insert a line break and optionally change indentation.
    NewLine(NewLine),
    /// Change indentation without inserting a line break.
    Indentation(Indentation),
    /// Open a context with the supplied prefix and name.
    OpenContext(Context<'data>),
    /// Close the current context without changing indentation.
    CloseContext(CloseContext),
}

const TEXT: u8 = 0;
const NEWLINE: u8 = 1;
const INDENTATION: u8 = 2;
pub(crate) const OPEN_CONTEXT: u8 = 3;
const CLOSE_CONTEXT: u8 = 4;
const NO_BREAK: u8 = 5;
const NO_BREAK_REGION: u8 = 6;
const JOIN: u8 = 7;
const JOIN_PART: u8 = 8;
const JOIN_END: u8 = 9;
const END_NO_BREAK_REGION: u8 = 10;

pub(crate) fn put_join(out: &mut BytesMut, policy: crate::core::BreakPolicy) {
    out.put_u8(JOIN);
    out.put_u8(match policy {
        crate::core::BreakPolicy::Inherit => 0,
        crate::core::BreakPolicy::Always => 1,
        crate::core::BreakPolicy::AsNeeded => 2,
    });
}

pub(crate) fn put_join_part(out: &mut BytesMut) {
    out.put_u8(JOIN_PART);
}
pub(crate) fn put_join_end(out: &mut BytesMut) {
    out.put_u8(JOIN_END);
}

pub(crate) fn put_no_break_region(out: &mut BytesMut) {
    out.put_u8(NO_BREAK_REGION);
}

pub(crate) fn put_end_no_break_region(out: &mut BytesMut) {
    out.put_u8(END_NO_BREAK_REGION);
}

fn invalid(message: &str) -> Error {
    Error::new(ErrorKind::InvalidData, message)
}

/// Borrow the next inline part, skipping boundaries belonging to nested joins.
pub(crate) fn join_part<'a>(input: &mut &'a [u8]) -> Result<(&'a [u8], bool)> {
    let start = *input;
    let mut depth = 0usize;
    loop {
        let before = input.len();
        match Operation::from_bytes(input)? {
            Operation::Join(_) => depth += 1,
            Operation::JoinEnd if depth > 0 => depth -= 1,
            Operation::JoinEnd => return Ok((&start[..start.len() - before], true)),
            Operation::JoinPart if depth == 0 => {
                return Ok((&start[..start.len() - before], false));
            }
            _ => {}
        }
    }
}

fn take<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8]> {
    if input.len() < len {
        return Err(Error::new(ErrorKind::UnexpectedEof, "truncated operation"));
    }
    let (value, rest) = input.split_at(len);
    *input = rest;
    Ok(value)
}

fn byte(input: &mut &[u8]) -> Result<u8> {
    Ok(take(input, 1)?[0])
}

fn text<'a>(input: &mut &'a [u8]) -> Result<&'a str> {
    let len = u64::from_le_bytes(take(input, 8)?.try_into().unwrap());
    let len = usize::try_from(len).map_err(|_| invalid("text length exceeds usize"))?;
    std::str::from_utf8(take(input, len)?).map_err(|_| invalid("invalid UTF-8"))
}

fn optional_text<'a>(input: &mut &'a [u8]) -> Result<Option<&'a str>> {
    match byte(input)? {
        0 => Ok(None),
        1 => Ok(Some(text(input)?)),
        _ => Err(invalid("invalid optional text tag")),
    }
}

fn put_text(out: &mut BytesMut, value: &str) {
    out.put_u64_le(value.len() as u64);
    out.extend_from_slice(value.as_bytes());
}

pub(crate) fn put_optional_text(out: &mut BytesMut, value: Option<&str>) {
    out.put_u8(u8::from(value.is_some()));
    if let Some(value) = value {
        put_text(out, value);
    }
}

fn put_indent(out: &mut BytesMut, value: Indentation) {
    match value {
        Indentation::Increment(amount) => {
            out.put_u8(0);
            out.put_u8(amount);
        }
        Indentation::Decrement(amount) => {
            out.put_u8(1);
            out.put_u8(amount);
        }
    }
}

fn indent(input: &mut &[u8]) -> Result<Indentation> {
    let direction = byte(input)?;
    let amount = byte(input)?;
    match direction {
        0 => Ok(Indentation::Increment(amount)),
        1 => Ok(Indentation::Decrement(amount)),
        _ => Err(invalid("invalid indentation direction")),
    }
}

struct TextWriter<'a>(&'a mut BytesMut);
impl fmt::Write for TextWriter<'_> {
    fn write_str(&mut self, value: &str) -> fmt::Result {
        self.0.extend_from_slice(value.as_bytes());
        Ok(())
    }
}

impl<T: Display> ToBytes for Text<T> {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()> {
        let start = out.len();
        out.put_u8(TEXT);
        out.put_u64_le(0);
        if write!(TextWriter(out), "{}", self.0).is_err() {
            out.truncate(start);
            return Err(invalid("text formatting failed"));
        }
        let len = (out.len() - start - 9) as u64;
        out[start + 1..start + 9].copy_from_slice(&len.to_le_bytes());
        Ok(())
    }
}

impl ToBytes for NewLine {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()> {
        out.put_u8(NEWLINE);
        out.put_u8(u8::from(self.0.is_some()));
        if let Some(value) = self.0 {
            put_indent(out, value);
        }
        Ok(())
    }
}

impl<T: AsRef<str>> ToBytes for NoBreak<T> {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()> {
        let text = self.0.as_ref();
        let length = crate::core::no_break_length(text)?;
        out.put_u8(NO_BREAK);
        out.put_u16_le(length);
        put_text(out, text);
        Ok(())
    }
}

impl ToBytes for Indentation {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()> {
        out.put_u8(INDENTATION);
        put_indent(out, *self);
        Ok(())
    }
}

impl ToBytes for CloseContext {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()> {
        out.put_u8(CLOSE_CONTEXT);
        out.put_u8(u8::from(self.0));
        Ok(())
    }
}

impl<'a> FromBytes<'a> for Operation<'a> {
    /// Decode one record, leaving input unchanged on error.
    fn from_bytes(input: &mut &'a [u8]) -> Result<Self> {
        let mut remaining = *input;
        let operation = match byte(&mut remaining)? {
            JOIN => Self::Join(match byte(&mut remaining)? {
                0 => crate::core::BreakPolicy::Inherit,
                1 => crate::core::BreakPolicy::Always,
                2 => crate::core::BreakPolicy::AsNeeded,
                _ => return Err(invalid("invalid join policy")),
            }),
            JOIN_PART => Self::JoinPart,
            JOIN_END => Self::JoinEnd,
            NO_BREAK_REGION => Self::NoBreakRegion,
            END_NO_BREAK_REGION => Self::EndNoBreakRegion,
            TEXT => Self::Text(Text(text(&mut remaining)?)),
            NO_BREAK => {
                let length = u16::from_le_bytes(take(&mut remaining, 2)?.try_into().unwrap());
                Self::NoBreak {
                    text: text(&mut remaining)?,
                    length,
                }
            }
            NEWLINE => Self::NewLine(NewLine(match byte(&mut remaining)? {
                0 => None,
                1 => Some(indent(&mut remaining)?),
                _ => return Err(invalid("invalid optional indentation tag")),
            })),
            INDENTATION => Self::Indentation(indent(&mut remaining)?),
            OPEN_CONTEXT => Self::OpenContext(Context::new(
                optional_text(&mut remaining)?,
                optional_text(&mut remaining)?,
            )),
            CLOSE_CONTEXT => Self::CloseContext(CloseContext(match byte(&mut remaining)? {
                0 => false,
                1 => true,
                _ => return Err(invalid("invalid close-context newline flag")),
            })),
            _ => return Err(invalid("unknown operation tag")),
        };
        *input = remaining;
        Ok(operation)
    }
}

macro_rules! decode_primitive {
    ($type:ty, $variant:ident) => {
        impl<'a> FromBytes<'a> for $type {
            fn from_bytes(input: &mut &'a [u8]) -> Result<Self> {
                let mut remaining = *input;
                match Operation::from_bytes(&mut remaining)? {
                    Operation::$variant(value) => {
                        *input = remaining;
                        Ok(value)
                    }
                    _ => Err(invalid("unexpected operation type")),
                }
            }
        }
    };
}

decode_primitive!(Text<&'a str>, Text);
impl<'a> FromBytes<'a> for NoBreak<&'a str> {
    fn from_bytes(input: &mut &'a [u8]) -> Result<Self> {
        let mut remaining = *input;
        match Operation::from_bytes(&mut remaining)? {
            Operation::NoBreak { text, .. } => {
                *input = remaining;
                Ok(NoBreak(text))
            }
            _ => Err(invalid("unexpected operation type")),
        }
    }
}
decode_primitive!(NewLine, NewLine);
decode_primitive!(Indentation, Indentation);
decode_primitive!(CloseContext, CloseContext);

#[cfg(test)]
mod tests;
