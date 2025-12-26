use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct MagicBytesHeuristic;

impl Heuristic for MagicBytesHeuristic {
    fn name(&self) -> &'static str {
        "magic-bytes"
    }

    fn apply(&self, input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        for hit in &feats.magic_hits {
            let mut conf = if hit.strong { 0.97 } else { 0.65 };
            if input.offset != 0 && hit.strong {
                // Strong relative to the window, not necessarily file start.
                conf = 0.85;
            }

            out.push(
                Suggestion::new(hit.name, conf)
                    .reason(format!("magic match at +0x{:X} in window", hit.at)),
            );
        }
    }
}
