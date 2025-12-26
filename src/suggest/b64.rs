use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct Base64Heuristic;

impl Heuristic for Base64Heuristic {
    fn name(&self) -> &'static str {
        "base64"
    }

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