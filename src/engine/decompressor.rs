use crate::storage::dictionary::Dictionary;
use crate::engine::huffman::{HuffmanTable, HuffmanNode};
use std::collections::HashMap;

fn rebuild_tree(encode_table: &HashMap<u32, Vec<bool>>) -> HuffmanNode {
    let mut root = HuffmanNode {
        freq: 0,
        token: None,
        left: None,
        right: None,
    };
    
    for (&token, code) in encode_table {
        let mut current = &mut root;
        
        for &bit in code {
            if bit {
                if current.right.is_none() {
                    current.right = Some(Box::new(HuffmanNode {
                        freq: 0,
                        token: None,
                        left: None,
                        right: None,
                    }));
                }
                current = current.right.as_mut().unwrap();
            } else {
                if current.left.is_none() {
                    current.left = Some(Box::new(HuffmanNode {
                        freq: 0,
                        token: None,
                        left: None,
                        right: None,
                    }));
                }
                current = current.left.as_mut().unwrap();
            }
        }
        
        current.token = Some(token);
    }
    
    root
}

pub fn decompress(
    data: &[u8],
    dict: &Dictionary,
) -> Vec<u8> {
    if data.len() < 4 {
        return Vec::new();
    }
    
    let mut offset = 0;
    
    // Read Huffman table size
    let table_size = u32::from_be_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
    ]) as usize;
    offset += 4;
    
    // Reconstruct Huffman table
    let mut encode_table = HashMap::new();
    
    for _ in 0..table_size {
        if offset + 6 > data.len() {
            return Vec::new();
        }
        
        let token = u32::from_be_bytes([
            data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
        ]);
        offset += 4;
        
        let code_len = data[offset] as usize;
        offset += 1;
        
        let code_bytes_len = data[offset] as usize;
        offset += 1;
        
        if offset + code_bytes_len > data.len() {
            return Vec::new();
        }
        
        let code_bytes = &data[offset..offset + code_bytes_len];
        offset += code_bytes_len;
        
        // Reconstruct code bits
        let mut code = Vec::new();
        let mut bits_read = 0;
        
        for &byte in code_bytes {
            for bit_pos in 0..8 {
                if bits_read >= code_len {
                    break;
                }
                let bit = (byte >> (7 - bit_pos)) & 1 == 1;
                code.push(bit);
                bits_read += 1;
            }
            if bits_read >= code_len {
                break;
            }
        }
        
        encode_table.insert(token, code);
    }
    
    // Read compressed data size
    if offset + 4 > data.len() {
        return Vec::new();
    }
    
    let compressed_size = u32::from_be_bytes([
        data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
    ]) as usize;
    offset += 4;
    
    if offset + compressed_size > data.len() {
        return Vec::new();
    }
    
    let compressed_data = &data[offset..offset + compressed_size];
    
    // Create Huffman table and decode
    let mut huffman_table = HuffmanTable {
        encode_table: encode_table.clone(),
        decode_tree: None,
    };
    
    // Rebuild decode tree from encode table
    huffman_table.decode_tree = Some(Box::new(rebuild_tree(&encode_table)));
    
    let tokens = huffman_table.decode(compressed_data);
    
    // Convert tokens back to bytes - optimized for large files
    let mut out = Vec::with_capacity(tokens.len() * 4); // Pre-allocate
    for &token in &tokens {
        if let Some(bytes) = dict.decode.get(&token) {
            out.extend_from_slice(bytes);
        } else if token <= 255 {
            out.push(token as u8);
        }
    }
    
    out
}
