use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct MixedRegionsHeuristic;

impl Heuristic for MixedRegionsHeuristic {
    fn apply(&self, input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        let range_norm = (feats.entropy_max_norm - feats.entropy_min_norm).clamp(0.0, 1.0);
        if input.entropy_std_bpb > 1.2 || range_norm > 0.55 {
            let conf = (0.60 + (range_norm as f32 * 0.30)).min(0.90);
            out.push(
                Suggestion::new("Mixed regions / container-like structure", conf)
                    .reason(format!(
                        "entropy stddev≈{:.2} bits/byte",
                        input.entropy_std_bpb
                    ))
                    .reason(format!("bin range≈{:.2} (normalized)", range_norm)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_high_entropy_variance() {
        let feats = Features {
            entropy_min_norm: 0.1,
            entropy_max_norm: 0.8,
            ..Default::default()
        };
        let mut out = Vec::new();
        let input = SuggestInput {
            data: &[],
            offset: 0,
            entropy_mean_bpb: 4.0,
            entropy_std_bpb: 1.4,
            entropy_bins_norm: None,
        };
        MixedRegionsHeuristic.apply(&input, &feats, &mut out);
        assert!(!out.is_empty());
    }
}
