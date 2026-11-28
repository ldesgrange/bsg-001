//! Framing and encoding rules shared by every BSG-001 command.

use std::error::Error;
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolError {
    UnrelatedReply,
    TruncatedText,
    TextNotAscii,
    TextTooLong,
    TruncatedNumber,
    TruncatedReading,
    TruncatedLoggingState,
    UnknownLoggingDuration,
    TruncatedRecords,
    UnknownStatus,
}

impl Display for ProtocolError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnrelatedReply => "The reply does not answer the query.",
            Self::TruncatedText => "The content stops before the text does.",
            Self::TextNotAscii => "Text must contain only ASCII characters.",
            Self::TextTooLong => "Text must fit in 255 characters.",
            Self::TruncatedNumber => "The content stops before the number does.",
            Self::TruncatedReading => "The content stops before the reading does.",
            Self::TruncatedLoggingState => "The content stops before the logging state does.",
            Self::UnknownLoggingDuration => "The logging duration is neither off nor from 1 to 31 days.",
            Self::TruncatedRecords => "The packets stop before the records do.",
            Self::UnknownStatus => "The status is neither measuring nor stopped.",
        };
        formatter.write_str(message)
    }
}

impl Error for ProtocolError {}

/// The operations a BSG-001 understands, each identified by the first byte of a frame.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Opcode {
    StartStop,
    SetClock,
    QueryLoggingState,
    SetName,
    QueryName,
    QueryReading,
    QueryRecords,
}

impl Opcode {
    const ALL: [Self; 7] = [
        Self::StartStop,
        Self::SetClock,
        Self::QueryLoggingState,
        Self::SetName,
        Self::QueryName,
        Self::QueryReading,
        Self::QueryRecords,
    ];

    pub const fn byte(self) -> u8 {
        match self {
            Self::StartStop => 0x10,
            Self::SetClock => 0x20,
            Self::QueryLoggingState => 0x21,
            Self::SetName => 0x30,
            Self::QueryName => 0x31,
            Self::QueryReading => 0xa0,
            Self::QueryRecords => 0xa1,
        }
    }

    /// The operation a byte identifies, if any.
    pub fn from_byte(byte: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|opcode| opcode.byte() == byte)
    }
}

/// Frame a command; a command without a payload is a query, written as its opcode alone.
pub fn command_frame(opcode: Opcode, payload: &[u8]) -> Vec<u8> {
    let mut frame = vec![opcode.byte()];
    if !payload.is_empty() {
        frame.push(u8::try_from(payload.len()).expect("a command payload fits in one frame"));
        frame.extend_from_slice(payload);
    }
    frame
}

/// Return what follows the opcode and length byte of the reply to a query.
pub fn reply_content(query: Opcode, reply: &[u8]) -> Result<&[u8], ProtocolError> {
    match reply {
        [opcode, _length, content @ ..] if *opcode == query.byte() => Ok(content),
        _ => Err(ProtocolError::UnrelatedReply),
    }
}

/// Encode text as a count byte followed by its ASCII characters.
pub fn encode_text(text: &str) -> Result<Vec<u8>, ProtocolError> {
    if !text.is_ascii() {
        return Err(ProtocolError::TextNotAscii);
    }
    let mut bytes = vec![u8::try_from(text.len()).map_err(|_| ProtocolError::TextTooLong)?];
    bytes.extend_from_slice(text.as_bytes());
    Ok(bytes)
}

/// Read counted ASCII text from the start of some content, ignoring what follows.
pub fn decode_text(content: &[u8]) -> Result<String, ProtocolError> {
    let (count, rest) = content.split_first().ok_or(ProtocolError::TruncatedText)?;
    let characters = rest.get(..usize::from(*count)).ok_or(ProtocolError::TruncatedText)?;
    if !characters.is_ascii() {
        return Err(ProtocolError::TextNotAscii);
    }
    Ok(characters.iter().copied().map(char::from).collect())
}

/// Read a two-byte number, least significant byte first, from the start of some content.
pub fn decode_number(content: &[u8]) -> Result<u16, ProtocolError> {
    match content {
        [low, high, ..] => Ok(u16::from_le_bytes([*low, *high])),
        _ => Err(ProtocolError::TruncatedNumber),
    }
}
