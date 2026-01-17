/// Growable reusable byte buffer
#[derive(Debug)]
pub struct ByteBuffer {
    buf: Vec<u8>,
}

impl ByteBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            buf: Vec::with_capacity(capacity),
        }
    }

    pub fn clear(&mut self) {
        self.buf.clear();
    }

    pub fn extend(&mut self, data: &[u8]) {
        self.buf.extend_from_slice(data);
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.buf
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
}
