use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct LowEntropyHeuristic;

impl Heuristic for LowEntropyHeuristic {
    fn apply(&self, input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        let h = input.entropy_mean_bpb.clamp(0.0, 8.0);
        if h < 1.2 || feats.zero_ratio > 0.35 || feats.top1_ratio > 0.35 {
            let mut conf = 0.65;
            if feats.zero_ratio > 0.70 {
                conf = 0.92;
            } else if h < 0.6 {
                conf = 0.85;
            }

            out.push(
                Suggestion::new("Low-entropy / padding / repeated patterns", conf)
                    .reason(format!("entropy≈{:.2} bits/byte", h))
                    .reason(format!("0x00={:.1}%", feats.zero_ratio * 100.0))
                    .reason(format!("top-byte={:.1}%", feats.top1_ratio * 100.0)),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_on_low_entropy_or_zeros() {
        let feats = Features {
            zero_ratio: 0.8,
            top1_ratio: 0.8,
            ..Default::default()
        };
        let mut out = Vec::new();
        let input = SuggestInput {
            data: &[],
            offset: 0,
            entropy_mean_bpb: 0.4,
            entropy_std_bpb: 0.1,
            entropy_bins_norm: None,
        };
        LowEntropyHeuristic.apply(&input, &feats, &mut out);
        assert!(!out.is_empty());
        assert!(out[0].label.contains("Low-entropy"));
    }
}
