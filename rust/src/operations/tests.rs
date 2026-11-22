use super::*;

#[test]
fn no_break_region_preserves_metadata_index_and_rejects_truncation() -> Result<()> {
    let mut buffer = BytesMut::new();
    put_no_break_region(&mut buffer, 42);
    let mut input = buffer.as_ref();
    match Operation::from_bytes(&mut input)? {
        Operation::NoBreakRegion(decoded) => assert_eq!(decoded, 42),
        _ => panic!("expected NoBreakRegion"),
    }
    assert!(input.is_empty());
    for end in 0..buffer.len() {
        let original = &buffer[..end];
        let mut input = original;
        assert_eq!(
            Operation::from_bytes(&mut input).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        assert_eq!(input, original);
    }
    Ok(())
}

#[test]
fn oversized_no_break_leaves_recording_unchanged() {
    let mut buffer = BytesMut::from(&b"existing"[..]);
    let error = NoBreak("a".repeat(65536))
        .to_bytes(&mut buffer)
        .unwrap_err();
    assert_eq!(error.kind(), ErrorKind::InvalidInput);
    assert_eq!(buffer.as_ref(), b"existing");
}

#[test]
fn no_break_records_character_length_separately_from_byte_length() -> Result<()> {
    let mut buffer = BytesMut::new();
    NoBreak(String::from("é 界")).to_bytes(&mut buffer)?;
    assert_eq!(buffer[0], NO_BREAK);
    assert_eq!(u16::from_le_bytes(buffer[1..3].try_into().unwrap()), 3);
    assert_eq!(u64::from_le_bytes(buffer[3..11].try_into().unwrap()), 6);
    let mut input = buffer.as_ref();
    match Operation::from_bytes(&mut input)? {
        Operation::NoBreak { text, length } => {
            assert_eq!(text, "é 界");
            assert_eq!(length, 3);
        }
        _ => panic!("expected NoBreak"),
    }
    assert!(input.is_empty());
    for end in 0..buffer.len() {
        let original = &buffer[..end];
        let mut input = original;
        assert_eq!(
            Operation::from_bytes(&mut input).unwrap_err().kind(),
            ErrorKind::UnexpectedEof
        );
        assert_eq!(input, original);
    }
    Ok(())
}

#[test]
fn round_trip_operations() -> Result<()> {
    let mut buffer = BytesMut::new();
    Text("hello λ\n  world").to_bytes(&mut buffer)?;
    Text(42).to_bytes(&mut buffer)?;
    NoBreak("a λ literal").to_bytes(&mut buffer)?;
    NewLine(None).to_bytes(&mut buffer)?;
    NewLine(Some(Indentation::Decrement(2))).to_bytes(&mut buffer)?;
    Indentation::Increment(3).to_bytes(&mut buffer)?;
    Context::new(Some("- "), Some("items")).to_bytes(&mut buffer)?;
    Context::new(None, None).to_bytes(&mut buffer)?;
    CloseContext(true).to_bytes(&mut buffer)?;

    let mut input = buffer.as_ref();
    let decoded = Text::<&str>::from_bytes(&mut input)?;
    assert_eq!(decoded.0, "hello λ\n  world");
    assert_eq!(decoded.0.as_ptr(), buffer[9..].as_ptr());
    assert_eq!(Text::<&str>::from_bytes(&mut input)?, Text("42"));
    assert_eq!(
        NoBreak::<&str>::from_bytes(&mut input)?,
        NoBreak("a λ literal")
    );
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
    assert_eq!(CloseContext::from_bytes(&mut input)?, CloseContext(true));
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
