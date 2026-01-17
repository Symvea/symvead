use crate::protocol::error::ProtocolError;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use crate::utils::crc::crc32;
use tracing::error;

/// Fixed-size frame header (12 bytes)
#[derive(Debug, Clone)]
pub struct FrameHeader {
    pub frame_type: u8,
    pub flags: u8,
    pub header_len: u16,
    pub payload_len: u32,
    pub checksum: u32,
}

impl FrameHeader {
    pub const SIZE: usize = 12;

    pub fn decode(buf: &[u8]) -> Result<Self, ProtocolError> {
        if buf.len() < Self::SIZE {
            error!("Frame header too short: {} bytes", buf.len());
            return Err(ProtocolError::Truncated);
        }

        let payload_len = u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]);

        let header = Self {
            frame_type: buf[0],
            flags: buf[1],
            header_len: u16::from_be_bytes([buf[2], buf[3]]),
            payload_len,
            checksum: u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]),
        };
        

        
        Ok(header)
    }

    pub fn encode(&self) -> [u8; Self::SIZE] {
        let mut buf = [0u8; Self::SIZE];
        buf[0] = self.frame_type;
        buf[1] = self.flags;
        buf[2..4].copy_from_slice(&self.header_len.to_be_bytes());
        buf[4..8].copy_from_slice(&self.payload_len.to_be_bytes());
        buf[8..12].copy_from_slice(&self.checksum.to_be_bytes());
        

        
        buf
    }
}

/// Frame types
#[derive(Debug)]
pub enum Frame {
    Upload { key: String, data: Vec<u8>, user_id: Option<String> },
    Download { key: String },
    Verify { key: String },
    Ack { key: String, original_size: u64, compressed_size: u64 },
    Data { key: String, data: Vec<u8> },
    Verified { key: String, hash_match: bool },
    NotFound { key: String },
    FreezeDictionary,
    Close,
    // Chunked upload frames
    ChunkStart { key: String, total_size: u64, chunk_count: u32, user_id: Option<String> },
    ChunkData { key: String, chunk_index: u32, data: Vec<u8> },
    ChunkEnd { key: String },
}

pub async fn read_frame(stream: &mut TcpStream) -> anyhow::Result<Frame> {

    
    // Read header
    let mut header_buf = [0u8; FrameHeader::SIZE];
    stream.read_exact(&mut header_buf).await?;
    

    
    let header = FrameHeader::decode(&header_buf)?;
    

    
    // Read payload
    let mut payload = vec![0u8; header.payload_len as usize];
    if header.payload_len > 0 {
        stream.read_exact(&mut payload).await?;

    }
    
    // Verify checksum
    let computed_checksum = crc32(&payload);

    
    if computed_checksum != header.checksum {
        error!("Checksum mismatch: expected={:x}, computed={:x}", header.checksum, computed_checksum);
        return Err(anyhow::anyhow!("Checksum mismatch"));
    }
    
    // Parse frame based on type

    
    match header.frame_type {
        1 => { // Upload

            if payload.len() < 4 {
                error!("Upload frame payload too short: {} bytes", payload.len());
                return Err(anyhow::anyhow!("Upload frame payload too short"));
            }
            
            let key_len = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;

            
            if payload.len() < 4 + key_len {
                error!("Upload frame payload too short for key: {} bytes needed, {} available", 4 + key_len, payload.len());
                return Err(anyhow::anyhow!("Upload frame payload too short for key"));
            }
            
            let key = String::from_utf8(payload[4..4+key_len].to_vec())?;
            let data = payload[4+key_len..].to_vec();
            

            Ok(Frame::Upload { key, data, user_id: None })
        },
        2 => { // Download
            let key = String::from_utf8(payload)?;
            Ok(Frame::Download { key })
        },
        8 => { // Verify
            let key = String::from_utf8(payload)?;
            Ok(Frame::Verify { key })
        },
        3 => {

            Ok(Frame::FreezeDictionary)
        },
        4 => {

            Ok(Frame::Close)
        },
        0x10 => { // ChunkStart

            if payload.len() < 12 {
                return Err(anyhow::anyhow!("ChunkStart frame payload too short"));
            }
            let key_len = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
            if payload.len() < 12 + key_len {
                return Err(anyhow::anyhow!("ChunkStart frame payload too short for key"));
            }
            let key = String::from_utf8(payload[4..4+key_len].to_vec())?;
            let total_size = u64::from_be_bytes([
                payload[4+key_len], payload[5+key_len], payload[6+key_len], payload[7+key_len],
                payload[8+key_len], payload[9+key_len], payload[10+key_len], payload[11+key_len]
            ]);
            let chunk_count = u32::from_be_bytes([
                payload[12+key_len], payload[13+key_len], payload[14+key_len], payload[15+key_len]
            ]);
            Ok(Frame::ChunkStart { key, total_size, chunk_count, user_id: None })
        },
        0x11 => { // ChunkData

            if payload.len() < 8 {
                return Err(anyhow::anyhow!("ChunkData frame payload too short"));
            }
            let key_len = u32::from_be_bytes([payload[0], payload[1], payload[2], payload[3]]) as usize;
            if payload.len() < 8 + key_len {
                return Err(anyhow::anyhow!("ChunkData frame payload too short for key"));
            }
            let key = String::from_utf8(payload[4..4+key_len].to_vec())?;
            let chunk_index = u32::from_be_bytes([
                payload[4+key_len], payload[5+key_len], payload[6+key_len], payload[7+key_len]
            ]);
            let data = payload[8+key_len..].to_vec();
            Ok(Frame::ChunkData { key, chunk_index, data })
        },
        0x12 => { // ChunkEnd

            let key = String::from_utf8(payload)?;
            Ok(Frame::ChunkEnd { key })
        },
        _ => {
            error!("Unknown frame type: {}", header.frame_type);
            Err(anyhow::anyhow!("Unknown frame type: {}", header.frame_type))
        }
    }
}

pub async fn write_frame(stream: &mut TcpStream, frame: Frame) -> anyhow::Result<()> {
    let (frame_type, payload) = match frame {
        Frame::Upload { key, data, .. } => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&(key.len() as u32).to_be_bytes());
            payload.extend_from_slice(key.as_bytes());
            payload.extend_from_slice(&data);

            (1u8, payload)
        },
        Frame::Download { key } => {

            (2u8, key.into_bytes())
        },
        Frame::Verify { key } => {

            (8u8, key.into_bytes())
        },
        Frame::Ack { key, original_size, compressed_size } => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&(key.len() as u32).to_be_bytes());
            payload.extend_from_slice(key.as_bytes());
            payload.extend_from_slice(&original_size.to_be_bytes());
            payload.extend_from_slice(&compressed_size.to_be_bytes());

            (5u8, payload)
        },
        Frame::Data { key, data } => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&(key.len() as u32).to_be_bytes());
            payload.extend_from_slice(key.as_bytes());
            payload.extend_from_slice(&data);

            (6u8, payload)
        },
        Frame::NotFound { key } => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&(key.len() as u32).to_be_bytes());
            payload.extend_from_slice(key.as_bytes());

            (7u8, payload)
        },
        Frame::Verified { key, hash_match } => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&(key.len() as u32).to_be_bytes());
            payload.extend_from_slice(key.as_bytes());
            payload.push(if hash_match { 1 } else { 0 });

            (9u8, payload)
        },
        Frame::FreezeDictionary => {

            (3u8, Vec::new())
        },
        Frame::Close => {

            (4u8, Vec::new())
        },
        Frame::ChunkStart { key, total_size, chunk_count, .. } => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&(key.len() as u32).to_be_bytes());
            payload.extend_from_slice(key.as_bytes());
            payload.extend_from_slice(&total_size.to_be_bytes());
            payload.extend_from_slice(&chunk_count.to_be_bytes());

            (0x10u8, payload)
        },
        Frame::ChunkData { key, chunk_index, data } => {
            let mut payload = Vec::new();
            payload.extend_from_slice(&(key.len() as u32).to_be_bytes());
            payload.extend_from_slice(key.as_bytes());
            payload.extend_from_slice(&chunk_index.to_be_bytes());
            payload.extend_from_slice(&data);

            (0x11u8, payload)
        },
        Frame::ChunkEnd { key } => {

            (0x12u8, key.into_bytes())
        },
    };
    
    let checksum = crc32(&payload);
    let header = FrameHeader {
        frame_type,
        flags: 0,
        header_len: FrameHeader::SIZE as u16,
        payload_len: payload.len() as u32,
        checksum,
    };
    

    
    stream.write_all(&header.encode()).await?;
    if !payload.is_empty() {
        stream.write_all(&payload).await?;
    }
    

    Ok(())
}
