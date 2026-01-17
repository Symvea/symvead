use crate::engine::symbols::Symbol;

pub fn tokenize(
    input: &[u8],
    symbols: &[Symbol],
) -> Vec<u32> {
    let mut out = Vec::new();
    let mut i = 0;

    while i < input.len() {
        let mut best_match = None;
        let mut best_len = 0;

        // Find the longest matching symbol
        for s in symbols {
            if input[i..].starts_with(&s.bytes) && s.bytes.len() > best_len {
                best_match = Some(s.token);
                best_len = s.bytes.len();
            }
        }

        if let Some(token) = best_match {
            out.push(token);
            i += best_len;
        } else {
            out.push(input[i] as u32);
            i += 1;
        }
    }

    out
}
