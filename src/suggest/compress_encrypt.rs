use crate::suggest::{Features, Heuristic, SuggestInput, Suggestion};

pub struct CompressedVsEncryptedHeuristic;

impl Heuristic for CompressedVsEncryptedHeuristic {
    fn name(&self) -> &'static str {
        "compressed-vs-encrypted"
    }

    fn apply(&self, input: &SuggestInput, feats: &Features, out: &mut Vec<Suggestion>) {
        let h = input.entropy_mean_bpb.clamp(0.0, 8.0);

        // Look for explicit compression magics.
        let has_compress_magic = feats.magic_hits.iter().any(|m| {
            matches!(
                m.name,
                "ZIP (local file header)"
                    | "ZIP (central directory)"
                    | "ZIP (end of central dir)"
                    | "7-Zip"
                    | "RAR (v1.5+)"
                    | "RAR (v5+)"
                    | "CAB"
                    | "ar archive"
                    | "GZIP"
                    | "Zstandard"
                    | "XZ"
                    | "Bzip2"
                    | "LZ4 frame"
                    | "LZIP"
                    | "Unix compress (.Z)"
                    | "LZMA (.lzma)"
            )
        });

        // "Near-uniform" at byte-level: chi-square ~ 255 (df=255) is typical.
        // We use wide thresholds; this is a heuristic, not a test.
        let chi = feats.chi_square_256;
        let byte_uniformish = (120.0..=600.0).contains(&chi);

        if h >= 7.2 {
            if has_compress_magic {
                let conf = if h >= 7.6 { 0.92 } else { 0.80 };
                out.push(
                    Suggestion::new("Compressed data", conf)
                        .reason(format!("entropy≈{:.2} bits/byte", h))
                        .reason("compression/container magic present in window".to_string()),
                );
            } else if byte_uniformish && input.entropy_std_bpb < 0.6 {
                let conf = if h >= 7.85 { 0.88 } else { 0.75 };
                out.push(
                    Suggestion::new("Encrypted or strongly-compressed/random data", conf)
                        .reason(format!("entropy≈{:.2} bits/byte", h))
                        .reason(format!("chi-square(256)≈{:.0} (near-uniform-ish)", chi)),
                );
            } else {
                out.push(
                    Suggestion::new("High-entropy binary blob", 0.65)
                        .reason(format!("entropy≈{:.2} bits/byte", h))
                        .reason(format!("chi-square(256)≈{:.0}", chi)),
                );
            }
        }
    }
}
