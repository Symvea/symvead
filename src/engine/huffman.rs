use std::collections::{HashMap, BinaryHeap};
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct HuffmanNode {
    pub freq: usize,
    pub token: Option<u32>,
    pub left: Option<Box<HuffmanNode>>,
    pub right: Option<Box<HuffmanNode>>,
}

impl PartialEq for HuffmanNode {
    fn eq(&self, other: &Self) -> bool {
        self.freq == other.freq
    }
}

impl Eq for HuffmanNode {}

impl PartialOrd for HuffmanNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HuffmanNode {
    fn cmp(&self, other: &Self) -> Ordering {
        other.freq.cmp(&self.freq) // Reverse for min-heap
    }
}

pub struct HuffmanTable {
    pub encode_table: HashMap<u32, Vec<bool>>,
    pub decode_tree: Option<Box<HuffmanNode>>,
}

impl HuffmanTable {
    pub fn build(tokens: &[u32]) -> Self {
        let mut freq_map = HashMap::new();
        for &token in tokens {
            *freq_map.entry(token).or_insert(0) += 1;
        }

        if freq_map.len() <= 1 {
            // Special case: only one unique token
            let mut encode_table = HashMap::new();
            if let Some(&token) = freq_map.keys().next() {
                encode_table.insert(token, vec![false]); // Single bit
            }
            return Self {
                encode_table,
                decode_tree: None,
            };
        }

        let mut heap = BinaryHeap::new();
        for (token, freq) in freq_map {
            heap.push(HuffmanNode {
                freq,
                token: Some(token),
                left: None,
                right: None,
            });
        }

        // Build Huffman tree
        while heap.len() > 1 {
            let right = heap.pop().unwrap();
            let left = heap.pop().unwrap();
            
            heap.push(HuffmanNode {
                freq: left.freq + right.freq,
                token: None,
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            });
        }

        let root = heap.pop().unwrap();
        let mut encode_table = HashMap::new();
        
        fn build_codes(node: &HuffmanNode, code: Vec<bool>, table: &mut HashMap<u32, Vec<bool>>) {
            if let Some(token) = node.token {
                table.insert(token, if code.is_empty() { vec![false] } else { code });
            } else {
                if let Some(ref left) = node.left {
                    let mut left_code = code.clone();
                    left_code.push(false);
                    build_codes(left, left_code, table);
                }
                if let Some(ref right) = node.right {
                    let mut right_code = code.clone();
                    right_code.push(true);
                    build_codes(right, right_code, table);
                }
            }
        }

        build_codes(&root, Vec::new(), &mut encode_table);

        Self {
            encode_table,
            decode_tree: Some(Box::new(root)),
        }
    }

    pub fn encode(&self, tokens: &[u32]) -> Vec<u8> {
        let mut bits = Vec::new();
        
        for &token in tokens {
            if let Some(code) = self.encode_table.get(&token) {
                bits.extend(code);
            }
        }

        // Pack bits into bytes
        let mut bytes = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;

        for bit in bits {
            if bit {
                current_byte |= 1 << (7 - bit_count);
            }
            bit_count += 1;

            if bit_count == 8 {
                bytes.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }

        // Handle remaining bits
        if bit_count > 0 {
            bytes.push(current_byte);
        }

        // Prepend bit count for last byte
        let mut result = vec![bit_count as u8];
        result.extend(bytes);
        result
    }

    pub fn decode(&self, data: &[u8]) -> Vec<u32> {
        if data.is_empty() {
            return Vec::new();
        }

        let last_byte_bits = data[0] as usize;
        let bytes = &data[1..];
        
        if bytes.is_empty() {
            return Vec::new();
        }

        let Some(ref root) = self.decode_tree else {
            // Single token case - optimized
            if let Some((&token, _)) = self.encode_table.iter().next() {
                let total_bits = if last_byte_bits > 0 {
                    (bytes.len() - 1) * 8 + last_byte_bits
                } else {
                    bytes.len() * 8
                };
                return vec![token; total_bits];
            }
            return Vec::new();
        };

        let mut tokens = Vec::new();
        let mut current_node = root.as_ref();
        
        // Pre-calculate total bits to avoid repeated calculations
        let total_bytes = bytes.len();
        
        for (i, &byte) in bytes.iter().enumerate() {
            let bits_in_byte = if i == total_bytes - 1 && last_byte_bits > 0 {
                last_byte_bits
            } else {
                8
            };

            // Process all bits in this byte at once
            for bit_pos in 0..bits_in_byte {
                let bit = (byte >> (7 - bit_pos)) & 1 == 1;
                
                current_node = if bit {
                    current_node.right.as_ref().unwrap()
                } else {
                    current_node.left.as_ref().unwrap()
                };

                if let Some(token) = current_node.token {
                    tokens.push(token);
                    current_node = root.as_ref();
                }
            }
        }

        tokens
    }
}