//! Compose heterogeneous dumpable values as nested tuple lists.
//!
//! Pass `(head, tail)` pairs ending in `()` to [`Dumper::tuple`].

use crate::core::{Dump, Dumper};
use std::io::{Result, Write};

/// Enables a nested tuple list to emit its values in order without separators.
///
/// Each head implements [`Dump`]; the tail is another list or `()`.
pub trait DumpHeadTail {
    /// Emit the values in this tuple list.
    ///
    /// # Arguments
    ///
    /// * `dumper` — Destination for the formatted output.
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success, or the first error from dumping a value.
    /// Remaining values are not dumped after an error.
    fn dump_head_tail<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()>;
}

impl DumpHeadTail for () {
    fn dump_head_tail<W: Write>(&self, _: &mut Dumper<W>) -> Result<()> {
        Ok(())
    }
}

impl<Head: Dump, Tail: DumpHeadTail> DumpHeadTail for (Head, Tail) {
    fn dump_head_tail<W: Write>(&self, dumper: &mut Dumper<W>) -> Result<()> {
        let (head, tail) = self;
        head.dump(dumper)?;
        tail.dump_head_tail(dumper)
    }
}
