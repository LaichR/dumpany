//! Compact operation records. Lengths are little-endian u64 byte counts.
//! Decoding consumes one record and borrows text from the input buffer.
use bytes::{BufMut, BytesMut};
use std::fmt::{self, Display, Write as FmtWrite};
use std::io::{Error, ErrorKind, Result, Write};

use crate::context::Context;
use crate::core::{Dump, Dumper, Indentation, NewLine, Text};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct CloseContext;

impl Dump for Context<'_, 'static> {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.open_context(self.prefix, self.name)
    }
}

impl Dump for CloseContext {
    fn dump<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        dumper.close_context()
    }
}

/// Append a complete operation record without allocating an intermediate buffer.
pub trait ToBytes {
    fn to_bytes(&self, out: &mut BytesMut) -> Result<()>;
}

/// Decode one operation of the expected type.
pub trait FromBytes<'a>: Sized {
    fn from_bytes(input: &mut &'a [u8]) -> Result<Self>;
}

#[derive(Debug)]
pub enum Operation<'parent, 'data> {
    Text(Text<&'data str>),
    NewLine(NewLine),
    Indentation(Indentation),
    OpenContext(Context<'parent, 'data>),
    CloseContext(CloseContext),
}

const TEXT: u8 = 0;
const NEWLINE: u8 = 1;
const INDENTATION: u8 = 2;
pub(crate) const OPEN_CONTEXT: u8 = 3;
const CLOSE_CONTEXT: u8 = 4;

fn invalid(message: &str) -> Error {
    Error::new(ErrorKind::InvalidData, message)
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
        Ok(())
    }
}

impl<'parent, 'a: 'parent> FromBytes<'a> for Operation<'parent, 'a> {
    /// Decode one record, leaving input unchanged on error. Opening records
    /// contain metadata only; Context::next_event attaches the live cursor.
    fn from_bytes(input: &mut &'a [u8]) -> Result<Self> {
        let mut remaining = *input;
        let operation = match byte(&mut remaining)? {
            TEXT => Self::Text(Text(text(&mut remaining)?)),
            NEWLINE => Self::NewLine(NewLine(match byte(&mut remaining)? {
                0 => None,
                1 => Some(indent(&mut remaining)?),
                _ => return Err(invalid("invalid optional indentation tag")),
            })),
            INDENTATION => Self::Indentation(indent(&mut remaining)?),
            OPEN_CONTEXT => Self::OpenContext(Context::from_metadata(
                optional_text(&mut remaining)?,
                optional_text(&mut remaining)?,
            )),
            CLOSE_CONTEXT => Self::CloseContext(CloseContext),
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
decode_primitive!(NewLine, NewLine);
decode_primitive!(Indentation, Indentation);
decode_primitive!(CloseContext, CloseContext);

#[cfg(test)]
mod tests;
