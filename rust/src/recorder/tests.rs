use super::*;
use crate::operations::{FromBytes, Operation};

#[test]
fn records_root_and_basic_operations() -> Result<()> {
    let mut recorder = Recorder::new(Some("- "), Some("root"))?;
    assert_eq!(recorder.context_depth(), 1);
    recorder.text("hello λ")?;
    recorder.inc_indent(1)?;
    recorder.text(42)?;
    recorder.new_line(Some(Indentation::Decrement(1)))?;
    recorder.dec_indent(1)?;
    recorder.new_line(None)?;
    recorder.close_context()?;
    assert_eq!(recorder.context_depth(), 0);

    let buffer = recorder.into_encoded_content();
    let mut input = buffer.as_ref();
    match Operation::from_bytes(&mut input)? {
        Operation::OpenContext(context) => {
            assert_eq!((context.prefix, context.name), (Some("- "), Some("root")));
        }
        _ => panic!("expected root context"),
    }
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("hello λ"));
    assert_eq!(
        Indentation::from_bytes(&mut input)?,
        Indentation::Increment(1)
    );
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("42"));
    assert_eq!(
        NewLine::from_bytes(&mut input)?,
        NewLine(Some(Indentation::Decrement(1)))
    );
    assert_eq!(
        Indentation::from_bytes(&mut input)?,
        Indentation::Decrement(1)
    );
    assert_eq!(NewLine::from_bytes(&mut input)?, NewLine(None));
    CloseContext::from_bytes(&mut input)?;
    assert!(input.is_empty());
    Ok(())
}

#[test]
fn records_nested_contexts_inline() -> Result<()> {
    let mut recorder = Recorder::new(None, Some("root"))?;
    recorder.text("before")?;
    recorder.open_context(Some("* "), Some("child"))?;
    assert_eq!(recorder.context_depth(), 2);
    recorder.text("inside")?;
    recorder.close_context()?;
    assert_eq!(recorder.context_depth(), 1);
    recorder.text("after")?;
    recorder.close_context()?;
    assert_eq!(recorder.context_depth(), 0);

    let mut input = recorder.encoded_content();
    Context::from_bytes(&mut input)?;
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("before"));
    let context = Context::from_bytes(&mut input)?;
    assert_eq!((context.prefix, context.name), (Some("* "), Some("child")));
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("inside"));
    CloseContext::from_bytes(&mut input)?;
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("after"));
    CloseContext::from_bytes(&mut input)?;
    assert!(input.is_empty());
    Ok(())
}

#[test]
fn records_operations_without_an_enclosing_context() -> Result<()> {
    let mut recorder = Recorder::default();
    recorder.text("section")?;
    recorder.new_line(None)?;
    let mut input = recorder.encoded_content();
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("section"));
    assert_eq!(NewLine::from_bytes(&mut input)?, NewLine(None));
    assert!(input.is_empty());
    Ok(())
}

#[test]
fn records_indentation_counts_as_single_operations() -> Result<()> {
    for count in [0, 1, 2, 255] {
        let mut recorder = Recorder::default();
        recorder.inc_indent(count)?;
        recorder.dec_indent(count)?;
        let mut input = recorder.encoded_content();
        assert_eq!(input.len(), 6);
        assert_eq!(
            Indentation::from_bytes(&mut input)?,
            Indentation::Increment(count)
        );
        assert_eq!(
            Indentation::from_bytes(&mut input)?,
            Indentation::Decrement(count)
        );
        assert!(input.is_empty());
    }
    Ok(())
}
