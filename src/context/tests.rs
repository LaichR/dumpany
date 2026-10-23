use super::*;
use crate::core::Text;
use crate::recorder::Recorder;

fn collect(context: &mut Context<'_, '_>, text: &mut String) -> Result<()> {
    while let Some(event) = context.next_event()? {
        match event {
            Operation::Text(value) => text.push_str(value.0),
            Operation::OpenContext(mut child) => collect(&mut child, text)?,
            _ => {}
        }
    }
    Ok(())
}

fn recording() -> Result<Recorder> {
    let mut recorder = Recorder::new(None, Some("root"))?;
    recorder.text("before")?;
    recorder.open_context(Some("- "), Some("child"))?;
    recorder.text("inside")?;
    recorder.open_context(None, Some("grandchild"))?;
    recorder.text("nested")?;
    recorder.close_context()?;
    recorder.text("tail")?;
    recorder.close_context()?;
    recorder.text("after")?;
    recorder.close_context()?;
    Ok(recorder)
}

#[test]
fn recursively_consumes_contexts() -> Result<()> {
    let recorder = recording()?;
    let mut cursor = Cursor::new(recorder.encoded_content());
    let mut root = Context::from_encoding(&mut cursor)?;
    assert_eq!(root.name, Some("root"));
    let mut text = String::new();
    collect(&mut root, &mut text)?;
    assert_eq!(text, "beforeinsidenestedtailafter");
    drop(root);
    cursor.ensure_exhausted()?;
    Ok(())
}

#[test]
fn dropped_child_skips_nested_remainder() -> Result<()> {
    let recorder = recording()?;
    let mut cursor = Cursor::new(recorder.encoded_content());
    let mut root = Context::from_encoding(&mut cursor)?;
    assert!(matches!(
        root.next_event()?,
        Some(Operation::Text(Text("before")))
    ));
    match root.next_event()? {
        Some(Operation::OpenContext(mut child)) => {
            assert_eq!(child.prefix, Some("- "));
            assert!(matches!(
                child.next_event()?,
                Some(Operation::Text(Text("inside")))
            ));
        }
        _ => panic!("expected child"),
    }
    assert!(matches!(
        root.next_event()?,
        Some(Operation::Text(Text("after")))
    ));
    assert!(root.next_event()?.is_none());
    drop(root);
    cursor.ensure_exhausted()?;
    Ok(())
}

#[test]
fn reports_errors_during_drop_to_parent() -> Result<()> {
    let mut recorder = Recorder::new(None, None)?;
    recorder.open_context(None, None)?;
    recorder.text("unclosed")?;
    let mut cursor = Cursor::new(recorder.encoded_content());
    let mut root = Context::from_encoding(&mut cursor)?;
    match root.next_event()? {
        Some(Operation::OpenContext(child)) => drop(child),
        _ => panic!("expected child"),
    }
    assert_eq!(
        root.next_event().unwrap_err().kind(),
        ErrorKind::UnexpectedEof
    );
    assert_eq!(
        root.next_event().unwrap_err().kind(),
        ErrorKind::UnexpectedEof
    );
    Ok(())
}

#[test]
fn rejects_trailing_content() -> Result<()> {
    let mut recorder = Recorder::new(None, None)?;
    recorder.close_context()?;
    recorder.text("trailing")?;
    let mut cursor = Cursor::new(recorder.encoded_content());
    let mut root = Context::from_encoding(&mut cursor)?;
    assert!(root.next_event()?.is_none());
    drop(root);
    assert_eq!(
        cursor.ensure_exhausted().unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    assert_eq!(
        cursor.ensure_exhausted().unwrap_err().kind(),
        ErrorKind::InvalidData
    );
    Ok(())
}

#[test]
fn retains_errors_from_direct_decoding() -> Result<()> {
    let mut recorder = Recorder::new(None, None)?;
    recorder.text("unclosed")?;
    let mut cursor = Cursor::new(recorder.encoded_content());
    let mut root = Context::from_encoding(&mut cursor)?;
    assert!(matches!(
        root.next_event()?,
        Some(Operation::Text(Text("unclosed")))
    ));
    let first = root.next_event().unwrap_err();
    let repeated = root.next_event().unwrap_err();
    assert_eq!(first.kind(), ErrorKind::UnexpectedEof);
    assert_eq!(repeated.kind(), first.kind());
    assert_eq!(repeated.to_string(), first.to_string());
    Ok(())
}
