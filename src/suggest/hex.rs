use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct HexHeuristic;

impl Heuristic for HexHeuristic {
    fn name(&self) -> &'static str {
        "hex"
    }

    fn apply(&self, _input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        if feats.hex_ratio > 0.96 && feats.printable_ratio > 0.96 && feats.hex_digits_even {
            let conf =
                (0.60 + ((feats.hex_ratio - 0.96) / 0.04).clamp(0.0, 1.0) as f32 * 0.35).min(0.95);

            out.push(
                Suggestion::new("Hex text (encoded binary)", conf)
                    .reason(format!("hex-ish={:.1}%", feats.hex_ratio * 100.0))
                    .reason("even number of hex digits in sample".to_string()),
            );
        }
    }
}