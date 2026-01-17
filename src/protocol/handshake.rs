use crate::protocol::{SYMVEA_MAGIC, PROTOCOL_VERSION};
use crate::protocol::error::ProtocolError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::{debug, error, trace};

#[derive(Debug, Clone)]
pub struct Handshake {
    pub version: u16,
    pub flags: u16,
    pub capabilities: u32,
}

impl Handshake {
    pub const WIRE_SIZE: usize = 4 + 2 + 2 + 4;

    pub fn decode(buf: &[u8]) -> Result<Self, ProtocolError> {
        trace!("Decoding handshake from {} bytes: {:?}", buf.len(), buf);
        
        if buf.len() < Self::WIRE_SIZE {
            error!("Handshake too short: {} bytes, expected {}", buf.len(), Self::WIRE_SIZE);
            return Err(ProtocolError::Truncated);
        }

        if &buf[0..4] != SYMVEA_MAGIC {
            error!("Invalid magic bytes: {:?}, expected {:?}", &buf[0..4], SYMVEA_MAGIC);
            return Err(ProtocolError::InvalidMagic);
        }

        let version = u16::from_be_bytes([buf[4], buf[5]]);
        if version != PROTOCOL_VERSION {
            error!("Unsupported version: {}, expected {}", version, PROTOCOL_VERSION);
            return Err(ProtocolError::UnsupportedVersion(version));
        }

        let flags = u16::from_be_bytes([buf[6], buf[7]]);
        let capabilities = u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]);

        let handshake = Self {
            version,
            flags,
            capabilities,
        };
        
        debug!("Decoded handshake: version={}, flags={}, capabilities={}", 
               handshake.version, handshake.flags, handshake.capabilities);

        Ok(handshake)
    }

    pub fn encode(&self) -> [u8; Self::WIRE_SIZE] {
        let mut buf = [0u8; Self::WIRE_SIZE];
        buf[0..4].copy_from_slice(&SYMVEA_MAGIC);
        buf[4..6].copy_from_slice(&self.version.to_be_bytes());
        buf[6..8].copy_from_slice(&self.flags.to_be_bytes());
        buf[8..12].copy_from_slice(&self.capabilities.to_be_bytes());
        
        debug!("Encoded handshake: version={}, flags={}, capabilities={}", 
               self.version, self.flags, self.capabilities);
        trace!("Handshake bytes: {:?}", buf);
        
        buf
    }
}

pub async fn read_handshake(stream: &mut TcpStream) -> anyhow::Result<Handshake> {
    debug!("Reading handshake ({} bytes)", Handshake::WIRE_SIZE);
    
    let mut buf = [0u8; Handshake::WIRE_SIZE];
    stream.read_exact(&mut buf).await?;
    
    trace!("Raw handshake bytes: {:?}", buf);
    
    let handshake = Handshake::decode(&buf)?;
    debug!("Handshake read successfully");
    
    Ok(handshake)
}

pub async fn write_handshake(stream: &mut TcpStream) -> anyhow::Result<()> {
    debug!("Writing handshake");
    
    let handshake = Handshake {
        version: PROTOCOL_VERSION,
        flags: 0,
        capabilities: 0,
    };
    
    let encoded = handshake.encode();
    stream.write_all(&encoded).await?;
    
    debug!("Handshake written successfully");
    Ok(())
}
