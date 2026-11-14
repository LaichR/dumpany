//! Produce structured text with wrapping, indentation, and aligned contexts.
//!
//! Start with [`core::Dumper`] and [`core::DumperConfig`]. Compose output with
//! [`core::Dump`] operations, iterators, or tuple lists. Named contexts allow
//! prologues and epilogues to be supplied separately from their bodies.
//!
//! Finish output explicitly with [`core::Dumper::dump_all`] or
//! [`core::Dumper::into_inner`].
//!
//! # Examples
//!
//! ```
//! use dumpany::context::Context;
//! use dumpany::core::{Dumper, DumperConfig};
//!
//! let mut out = Dumper::new(Vec::new(), DumperConfig::default())?;
//! Context::new(Some("- "), None).with(&mut out, |out| out.text("A list item"))?;
//! let bytes = out.into_inner().map_err(|error| error.into_parts().0)?;
//! assert_eq!(bytes, b"- A list item\n");
//! # Ok::<(), std::io::Error>(())
//! ```

pub mod context;
pub mod core;
pub mod dump_tuple;
mod operations;
pub mod recorder;
