use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use crate::storage::PersistentStorage;

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternAnalytics {
    pub pattern_frequency: HashMap<String, u64>,
    pub temporal_stability: HashMap<String, u64>, // days since first seen
    pub coverage_analysis: HashMap<String, f64>,  // % of files explained
}

impl PatternAnalytics {
    pub fn analyze_corpus(storage: &PersistentStorage) -> Result<Self, Box<dyn std::error::Error>> {
        let mut pattern_frequency = HashMap::new();
        let mut temporal_stability = HashMap::new();
        let mut coverage_analysis = HashMap::new();
        
        let total_files = storage.count_files()?;
        let current_time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        
        // Analyze each symbol
        for symbol_hash in storage.list_symbols()? {
            let usage = storage.get_symbol_usage(&symbol_hash)?;
            let symbol = storage.load_symbol(&symbol_hash)?;
            
            // Pattern frequency
            pattern_frequency.insert(symbol_hash.clone(), usage.total_occurrences);
            
            // Temporal stability (days since first seen)
            let days_old = (current_time - symbol.first_seen) / 86400;
            temporal_stability.insert(symbol_hash.clone(), days_old);
            
            // Coverage analysis (% of files that contain this pattern)
            let coverage = (usage.objects.len() as f64 / total_files as f64) * 100.0;
            coverage_analysis.insert(symbol_hash, coverage);
        }
        
        Ok(Self {
            pattern_frequency,
            temporal_stability,
            coverage_analysis,
        })
    }
    
    pub fn get_insights(&self) -> Vec<String> {
        let mut insights = Vec::new();
        
        // Find most frequent patterns
        if let Some((_pattern, count)) = self.pattern_frequency.iter().max_by_key(|(_, &v)| v) {
            insights.push(format!("This pattern appears in {} documents", count));
        }
        
        // Find oldest stable patterns
        if let Some((_pattern, days)) = self.temporal_stability.iter().max_by_key(|(_, &v)| v) {
            if *days > 365 {
                let years = days / 365;
                insights.push(format!("This hasn't changed in {} years", years));
            }
        }
        
        // Find high-coverage patterns
        if let Some((_pattern, coverage)) = self.coverage_analysis.iter()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap()) {
            insights.push(format!("This structure explains {:.0}% of files", coverage));
        }
        
        insights
    }
}