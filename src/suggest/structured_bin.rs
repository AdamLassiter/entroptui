use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct StructuredBinaryHeuristic;

impl Heuristic for StructuredBinaryHeuristic {
    fn name(&self) -> &'static str {
        "structured-binary"
    }

    fn apply(&self, input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        let h = input.entropy_mean_bpb.clamp(0.0, 8.0);

        // A rough "structured" signal: entropy low-mid, but not dominated by zeros,
        // and non-uniform byte histogram.
        if (1.2..=5.0).contains(&h)
            && feats.zero_ratio < 0.20
            && feats.printable_ratio < 0.85
            && feats.chi_square_256 > 1200.0
        {
            let conf = 0.62 + ((5.0 - h) / 3.8).clamp(0.0, 1.0) as f32 * 0.18;
            out.push(
                Suggestion::new(
                    "Structured binary (fields/tables/serialized)",
                    conf.min(0.85),
                )
                .reason(format!("entropy≈{:.2} bits/byte", h))
                .reason(format!(
                    "chi-square(256)≈{:.0} (non-uniform)",
                    feats.chi_square_256
                ))
                .reason(format!(
                    "adjacent equal≈{:.1}%",
                    feats.adjacent_equal_ratio * 100.0
                )),
            );
        }
    }
}
