use bsg_001::domain::protocol::{Opcode, ProtocolError, command_frame, decode_number, decode_text, encode_text, reply_content};

#[test]
fn each_operation_has_its_opcode() {
    for (opcode, byte) in [
        (Opcode::StartStop, 0x10),
        (Opcode::SetClock, 0x20),
        (Opcode::QueryLoggingState, 0x21),
        (Opcode::SetName, 0x30),
        (Opcode::QueryName, 0x31),
        (Opcode::QueryReading, 0xa0),
        (Opcode::QueryRecords, 0xa1),
    ] {
        assert_eq!(opcode.byte(), byte);
        assert_eq!(Opcode::from_byte(byte), Some(opcode));
    }
    assert_eq!(Opcode::from_byte(0x99), None);
}

#[test]
fn a_query_is_its_opcode_alone() {
    assert_eq!(command_frame(Opcode::QueryLoggingState, &[]), [0x21]);
}

#[test]
fn a_command_with_a_payload_is_followed_by_its_length() {
    assert_eq!(command_frame(Opcode::StartStop, &[1, 10]), [0x10, 2, 1, 10]);
}

#[test]
fn the_content_of_a_reply_follows_its_opcode_and_length() {
    assert_eq!(reply_content(Opcode::QueryLoggingState, &[0x21, 3, 7, 8, 9]), Ok(&[7, 8, 9][..]));
}

#[test]
fn a_reply_to_another_query_or_without_a_length_is_unrelated() {
    assert_eq!(reply_content(Opcode::QueryLoggingState, &[0xa0, 1, 7]), Err(ProtocolError::UnrelatedReply));
    assert_eq!(reply_content(Opcode::QueryLoggingState, &[0x21]), Err(ProtocolError::UnrelatedReply));
}

#[test]
fn text_is_encoded_as_a_count_and_its_ascii_characters() {
    assert_eq!(encode_text("AB12"), Ok(vec![4, b'A', b'B', b'1', b'2']));
    assert_eq!(encode_text("é"), Err(ProtocolError::TextNotAscii));
    assert_eq!(encode_text(&"A".repeat(256)), Err(ProtocolError::TextTooLong));
}

#[test]
fn counted_text_is_decoded_ignoring_what_follows() {
    assert_eq!(decode_text(&[2, b'A', b'B', b'X']), Ok("AB".to_owned()));
    assert_eq!(decode_text(&[3, b'A', b'B']), Err(ProtocolError::TruncatedText));
    assert_eq!(decode_text(&[]), Err(ProtocolError::TruncatedText));
    assert_eq!(decode_text(&[1, 0xe9]), Err(ProtocolError::TextNotAscii));
}

#[test]
fn a_number_is_two_bytes_least_significant_first() {
    assert_eq!(decode_number(&[0x34, 0x12, 0xff]), Ok(0x1234));
    assert_eq!(decode_number(&[0x34]), Err(ProtocolError::TruncatedNumber));
}
