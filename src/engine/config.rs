#[derive(Debug, Clone)]
pub struct EngineConfig {
    pub max_symbol_len: usize,
    pub min_gain_bytes: isize,
    pub allow_user_dict: bool,
    pub allow_global_dict: bool,
}

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            max_symbol_len: 64,
            min_gain_bytes: 2,
            allow_user_dict: true,
            allow_global_dict: true,
        }
    }
}
