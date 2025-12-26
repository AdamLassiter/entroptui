use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct TextHeuristic;

impl Heuristic for TextHeuristic {
    fn name(&self) -> &'static str {
        "text"
    }

    fn apply(&self, input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        let h = input.entropy_mean_bpb.clamp(0.0, 8.0);

        // "Text-ish" usually: printable high, entropy midrange, chi-square high.
        if feats.printable_ratio > 0.90 && feats.zero_ratio < 0.02 {
            let mut conf = 0.55 + (feats.printable_ratio - 0.90).clamp(0.0, 0.10) as f32 * 3.0;
            if (3.0..=6.2).contains(&h) {
                conf += 0.15;
            }
            if feats.utf8_valid {
                out.push(
                    Suggestion::new("UTF-8 text", conf.min(0.95))
                        .reason(format!("printable={:.1}%", feats.printable_ratio * 100.0))
                        .reason(format!("entropy≈{:.2} bits/byte", h))
                        .reason(format!("zeros≈{:.2}%", feats.zero_ratio * 100.0)),
                );
            } else {
                out.push(
                    Suggestion::new("ASCII-ish text", conf.min(0.85))
                        .reason(format!("printable={:.1}%", feats.printable_ratio * 100.0))
                        .reason(format!("entropy≈{:.2} bits/byte", h)),
                );
            }
        }
        if feats.printable_ratio > 0.80 && feats.zero_ratio < 0.20 {
            let conf = 0.50 + (feats.printable_ratio - 0.80).clamp(0.0, 0.20) as f32 * 1.5;
            out.push(
                Suggestion::new("Short text", conf.min(0.75))
                    .reason(format!("printable={:.1}%", feats.printable_ratio * 100.0))
                    .reason(format!("entropy≈{:.2} bits/byte", h)),
            );
        }
    }
}
