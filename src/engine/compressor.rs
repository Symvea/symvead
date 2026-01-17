use crate::engine::{
    tokenizer::tokenize,
    planner::plan_symbols,
    huffman::HuffmanTable,
    symbols::Symbol,
};
use crate::storage::{
    dictionary::Dictionary,
    symbols::SymbolStore,
    metadata::{SymbolInfo, TokenBreakdown},
};

pub fn compress(
    input: &[u8],
    dict: &mut Dictionary,
    symbol_store: &SymbolStore,
    object_key: &str,
) -> (Vec<u8>, Vec<SymbolInfo>, f64, TokenBreakdown) {
    let mut symbol_infos = Vec::new();
    
    if !dict.frozen {
        let symbols = if input.len() > 1024 * 1024 {
            let sample_size = (input.len() / 10).min(64 * 1024);
            plan_symbols(&input[..sample_size], 16)
        } else {
            plan_symbols(input, 32)
        };
        
        for s in symbols {
            let symbol = Symbol::new(s.bytes.clone(), s.token, s.gain);
            
            // Store symbol globally
            symbol_store.store_symbol(&symbol.hash, &symbol.bytes).ok();
            symbol_store.add_usage(&symbol.hash, object_key, symbol.bytes.len() as u64, 1).ok();
            
            // Track for metadata
            symbol_infos.push(SymbolInfo {
                hash: symbol.hash.clone(),
                bytes: symbol.bytes.len() as u64,
            });
            
            dict.encode.insert(symbol.bytes.clone(), symbol.token);
            dict.decode.insert(symbol.token, symbol.bytes);
        }
    } else {
        // Dictionary is frozen, track existing symbols with actual usage counts
        let tokens = tokenize(input, &dict.encode.iter().map(|(b, t)| Symbol::new(b.clone(), *t, 0)).collect::<Vec<_>>());
        
        // Count actual symbol usage in this file
        let mut symbol_counts = std::collections::HashMap::new();
        for &token in &tokens {
            if let Some(bytes) = dict.decode.get(&token) {
                let symbol = Symbol::new(bytes.clone(), token, 0);
                *symbol_counts.entry(symbol.hash.clone()).or_insert(0) += 1;
                
                if !symbol_infos.iter().any(|s| s.hash == symbol.hash) {
                    symbol_infos.push(SymbolInfo {
                        hash: symbol.hash.clone(),
                        bytes: symbol.bytes.len() as u64,
                    });
                }
            }
        }
        
        // Update usage with actual counts
        for (hash, count) in symbol_counts {
            if let Some(info) = symbol_infos.iter().find(|s| s.hash == hash) {
                symbol_store.add_usage(&hash, object_key, info.bytes, count).ok();
            }
        }
    }

    let symbols = dict.encode.iter().map(|(b, t)| {
        Symbol::new(b.clone(), *t, 0)
    }).collect::<Vec<_>>();

    let tokens = tokenize(input, &symbols);
    
    // Calculate explained bytes from actual token usage
    let mut explained_bytes = 0u64;
    let mut literal_bytes = 0u64;
    
    for &token in &tokens {
        if let Some(bytes) = dict.decode.get(&token) {
            explained_bytes += bytes.len() as u64;
        } else {
            literal_bytes += 1; // Raw byte
        }
    }
    
    let huffman_table = HuffmanTable::build(&tokens);
    let huffman_data = huffman_table.encode(&tokens);
    
    let mut output = Vec::new();
    output.extend_from_slice(&(huffman_table.encode_table.len() as u32).to_be_bytes());
    
    for (&token, code) in &huffman_table.encode_table {
        output.extend_from_slice(&token.to_be_bytes());
        output.push(code.len() as u8);
        
        let mut code_bytes = Vec::new();
        let mut current_byte = 0u8;
        let mut bit_count = 0;
        
        for &bit in code {
            if bit {
                current_byte |= 1 << (7 - bit_count);
            }
            bit_count += 1;
            
            if bit_count == 8 {
                code_bytes.push(current_byte);
                current_byte = 0;
                bit_count = 0;
            }
        }
        
        if bit_count > 0 {
            code_bytes.push(current_byte);
        }
        
        output.push(code_bytes.len() as u8);
        output.extend(code_bytes);
    }
    
    output.extend_from_slice(&(huffman_data.len() as u32).to_be_bytes());
    output.extend(huffman_data);
    
    let explained_ratio = if input.len() > 0 {
        explained_bytes as f64 / input.len() as f64
    } else {
        0.0
    };
    
    let token_breakdown = TokenBreakdown {
        symbol_bytes: explained_bytes,
        literal_bytes,
        literal_reason: "Below promotion threshold".to_string(),
    };
    
    (output, symbol_infos, explained_ratio, token_breakdown)
}
