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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{DumperConfig, NewLine, Text};
    use std::{cell::RefCell, io::Error, rc::Rc};
    use tuple_list::tuple_list;

    #[test]
    fn empty_tuple_emits_nothing() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
        tuple_list!().dump_head_tail(&mut dumper)?;
        dumper.tuple(tuple_list!())?;
        assert!(writer.borrow().is_empty());
        Ok(())
    }

    #[test]
    fn single_element_is_dumped() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
        dumper.tuple(tuple_list!(Text("hello")))?;
        dumper.new_line(None)?;
        assert_eq!(&*writer.borrow(), b"hello\n");
        Ok(())
    }

    #[test]
    fn heterogeneous_elements_are_dumped_in_order() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
        dumper.tuple(tuple_list!(
            Text("answer: "),
            Text(42),
            NewLine(None),
            Text("done"),
        ))?;
        dumper.new_line(None)?;
        assert_eq!(&*writer.borrow(), b"answer: 42\ndone\n");
        Ok(())
    }

    #[test]
    fn tuple_list_can_be_dumped_repeatedly() -> Result<()> {
        let writer = Rc::new(RefCell::new(Vec::new()));
        let mut dumper = Dumper::new(writer.clone(), DumperConfig::default());
        let items = tuple_list!(Text("hello"), NewLine(None));
        items.dump_head_tail(&mut dumper)?;
        items.dump_head_tail(&mut dumper)?;
        assert_eq!(&*writer.borrow(), b"hello\nhello\n");
        Ok(())
    }

    struct Step {
        name: &'static str,
        calls: Rc<RefCell<Vec<&'static str>>>,
        fail: bool,
    }

    impl Dump for Step {
        fn dump<W: Write>(&self, _: &mut Dumper<W>) -> Result<()> {
            self.calls.borrow_mut().push(self.name);
            if self.fail {
                Err(Error::other(self.name))
            } else {
                Ok(())
            }
        }
    }

    #[test]
    fn errors_in_head_or_tail_stop_remaining_elements() {
        for failing_index in [0, 1] {
            let writer = Rc::new(RefCell::new(Vec::new()));
            let mut dumper = Dumper::new(writer, DumperConfig::default());
            let calls = Rc::new(RefCell::new(Vec::new()));
            let step = |name, index| Step {
                name,
                calls: calls.clone(),
                fail: index == failing_index,
            };
            let error = dumper
                .tuple(tuple_list!(
                    step("first", 0),
                    step("second", 1),
                    step("third", 2),
                ))
                .unwrap_err();
            let expected = &["first", "second"][..=failing_index];
            assert_eq!(&*calls.borrow(), expected);
            assert_eq!(error.kind(), std::io::ErrorKind::Other);
            assert_eq!(error.to_string(), expected[failing_index]);
        }
    }
}
