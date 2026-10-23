use crate::core::{Dump, Dumper};
use std::io::{Result, Write};

pub trait DumpHeadTail {
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
