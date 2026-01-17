use std::collections::HashMap;
use crate::engine::symbols::Symbol;

pub fn plan_symbols(
    data: &[u8],
    max_len: usize,
) -> Vec<Symbol> {
    let mut freq = HashMap::<Vec<u8>, usize>::new();
    
    // Sample data for large files to avoid O(n²) complexity
    let sample_data = if data.len() > 1024 * 1024 {
        let sample_size = (data.len() / 20).min(256 * 1024); // 5% sample, max 256KB
        &data[..sample_size]
    } else {
        data
    };
    
    // Limit max_len for performance
    let effective_max_len = max_len.min(16);

    for len in 2..=effective_max_len {
        for i in 0..sample_data.len().saturating_sub(len) {
            freq.entry(sample_data[i..i+len].to_vec())
                .and_modify(|v| *v += 1)
                .or_insert(1);
        }
    }

    let mut symbols = Vec::new();
    let mut token = 256u32;

    for (bytes, count) in freq {
        let gain = (count as isize * bytes.len() as isize)
            - (count as isize * 2);

        if gain > 0 {
            symbols.push(Symbol::new(bytes, token, gain));
            token += 1;
        }
    }

    symbols.sort_by(|a, b| b.gain.cmp(&a.gain));
    symbols.truncate(1000); // Limit symbol count
    symbols
}
