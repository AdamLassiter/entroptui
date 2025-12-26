use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct Base64Heuristic;

impl Heuristic for Base64Heuristic {
    fn apply(&self, _input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        if feats.base64_ratio > 0.97 && feats.printable_ratio > 0.97 {
            let mut conf =
                0.60 + ((feats.base64_ratio - 0.97) / 0.03).clamp(0.0, 1.0) as f32 * 0.30;
            if feats.base64_padding_ratio > 0.002 {
                conf += 0.05;
            }

            out.push(
                Suggestion::new("Base64 text (encoded binary)", conf.min(0.95))
                    .reason(format!("base64-ish={:.1}%", feats.base64_ratio * 100.0))
                    .reason(format!(
                        "padding '='≈{:.2}%",
                        feats.base64_padding_ratio * 100.0
                    )),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn triggers_on_base64_like_text() {
        let feats = Features {
            base64_ratio: 0.99,
            printable_ratio: 0.98,
            base64_padding_ratio: 0.01,
            ..Default::default()
        };
        let mut out = Vec::new();
        Base64Heuristic.apply(
            &SuggestInput {
                data: b"",
                offset: 0,
                entropy_mean_bpb: 0.0,
                entropy_std_bpb: 0.0,
                entropy_bins_norm: None,
            },
            &feats,
            &mut out,
        );
        assert_eq!(out[0].label, "Base64 text (encoded binary)");
        assert!(out[0].confidence > 0.6);
    }

    #[test]
    fn base64_heuristic_does_not_trigger_below_threshold() {
        let feats = Features {
            base64_ratio: 0.90,
            printable_ratio: 0.95,
            ..Default::default()
        };
        let mut out = Vec::new();
        let input = SuggestInput {
            data: &[],
            offset: 0,
            entropy_mean_bpb: 0.0,
            entropy_std_bpb: 0.0,
            entropy_bins_norm: None,
        };
        Base64Heuristic.apply(&input, &feats, &mut out);
        assert!(out.is_empty(), "should not emit suggestion below threshold");
    }
}
