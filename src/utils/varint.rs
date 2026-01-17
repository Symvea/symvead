/// Encode u64 into varint (LEB128-style)
pub fn encode_varint(mut value: u64, out: &mut Vec<u8>) {
    while value >= 0x80 {
        out.push((value as u8) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

/// Decode varint from bytes
pub fn decode_varint(buf: &[u8]) -> Option<(u64, usize)> {
    let mut result = 0u64;
    let mut shift = 0;

    for (i, &byte) in buf.iter().enumerate() {
        let value = (byte & 0x7F) as u64;
        result |= value << shift;

        if byte & 0x80 == 0 {
            return Some((result, i + 1));
        }

        shift += 7;
        if shift > 63 {
            return None;
        }
    }
    None
}
