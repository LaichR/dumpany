use super::*;

#[test]
fn round_trip_operations() -> Result<()> {
    let mut buffer = BytesMut::new();
    Text("hello λ\n  world").to_bytes(&mut buffer)?;
    Text(42).to_bytes(&mut buffer)?;
    NewLine(None).to_bytes(&mut buffer)?;
    NewLine(Some(Indentation::Decrement(2))).to_bytes(&mut buffer)?;
    Indentation::Increment(3).to_bytes(&mut buffer)?;
    Context::new(Some("- "), Some("items")).to_bytes(&mut buffer)?;
    Context::new(None, None).to_bytes(&mut buffer)?;
    CloseContext.to_bytes(&mut buffer)?;

    let mut input = buffer.as_ref();
    let decoded = Text::<&str>::from_bytes(&mut input)?;
    assert_eq!(decoded.0, "hello λ\n  world");
    assert_eq!(decoded.0.as_ptr(), buffer[9..].as_ptr());
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("42"));
    assert_eq!(NewLine::from_bytes(&mut input)?, NewLine(None));
    assert_eq!(
        NewLine::from_bytes(&mut input)?,
        NewLine(Some(Indentation::Decrement(2)))
    );
    assert_eq!(
        Indentation::from_bytes(&mut input)?,
        Indentation::Increment(3)
    );
    let context = Context::from_bytes(&mut input)?;
    assert_eq!((context.prefix, context.name), (Some("- "), Some("items")));
    let context = Context::from_bytes(&mut input)?;
    assert_eq!((context.prefix, context.name), (None, None));
    assert_eq!(CloseContext::from_bytes(&mut input)?, CloseContext);
    assert!(input.is_empty());
    Ok(())
}

#[test]
fn rejects_truncated_and_invalid_records() -> Result<()> {
    let mut buffer = BytesMut::new();
    Text("hello").to_bytes(&mut buffer)?;
    for len in 0..buffer.len() {
        let original = &buffer[..len];
        let mut input = original;
        assert_eq!(
            Operation::from_bytes(&mut input).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        assert_eq!(input, original);
    }
    buffer[9] = 255;
    assert_eq!(
        Operation::from_bytes(&mut buffer.as_ref())
            .unwrap_err()
            .kind(),
        ErrorKind::InvalidData
    );
    for record in [
        &[255][..],
        &[NEWLINE, 2],
        &[INDENTATION, 2, 1],
        &[OPEN_CONTEXT, 2],
    ] {
        assert_eq!(
            Operation::from_bytes(&mut &record[..]).unwrap_err().kind(),
            ErrorKind::InvalidData
        );
    }
    let mut input = &[CLOSE_CONTEXT][..];
    assert!(NewLine::from_bytes(&mut input).is_err());
    assert_eq!(input, &[CLOSE_CONTEXT]);
    Ok(())
}
