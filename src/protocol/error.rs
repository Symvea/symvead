use std::fmt;

#[derive(Debug)]
pub enum ProtocolError {
    InvalidMagic,
    UnsupportedVersion(u16),
    FrameTooLarge(usize),
    InvalidHeader,
    UnexpectedFrameType(u8),
    Truncated,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProtocolError::InvalidMagic =>
                write!(f, "invalid protocol magic"),
            ProtocolError::UnsupportedVersion(v) =>
                write!(f, "unsupported protocol version {}", v),
            ProtocolError::FrameTooLarge(size) =>
                write!(f, "frame too large: {}", size),
            ProtocolError::InvalidHeader =>
                write!(f, "invalid frame header"),
            ProtocolError::UnexpectedFrameType(t) =>
                write!(f, "unexpected frame type {}", t),
            ProtocolError::Truncated =>
                write!(f, "truncated frame"),
        }
    }
}

impl std::error::Error for ProtocolError {}
