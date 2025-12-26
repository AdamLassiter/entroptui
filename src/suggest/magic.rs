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

#[derive(Clone, Debug)]
pub struct MagicHit {
    pub name: &'static str,
    pub at: usize,
    pub strong: bool, // at==0
}

pub fn scan_magic(sample: &[u8]) -> Vec<MagicHit> {
    // We treat hits at the beginning as "strong".
    let mut hits = Vec::new();

    let magics: &[(&'static str, &'static [u8])] = &[
        ("PNG", b"\x89PNG\r\n\x1a\n"),
        ("ZIP (local file header)", b"PK\x03\x04"),
        ("ZIP (central directory)", b"PK\x01\x02"),
        ("GZIP", b"\x1F\x8B"),
        ("Zstandard", b"\x28\xB5\x2F\xFD"),
        ("XZ", b"\xFD\x37\x7A\x58\x5A\x00"),
        ("Bzip2", b"BZh"),
        ("PDF", b"%PDF-"),
        ("ELF", b"\x7FELF"),
        ("PE/COFF (MZ)", b"MZ"),
        ("UTF-8 BOM", b"\xEF\xBB\xBF"),
        ("UTF-16 LE BOM", b"\xFF\xFE"),
        ("UTF-16 BE BOM", b"\xFE\xFF"),
    ];

    for (name, sig) in magics {
        if let Some(at) = find_subslice(sample, sig) {
            hits.push(MagicHit {
                name,
                at,
                strong: at == 0,
            });
        }
    }

    // Mach-O (several common magics)
    let macho: &[(&'static str, [u8; 4])] = &[
        ("Mach-O (FE ED FA CE)", [0xFE, 0xED, 0xFA, 0xCE]),
        ("Mach-O (FE ED FA CF)", [0xFE, 0xED, 0xFA, 0xCF]),
        ("Mach-O (CE FA ED FE)", [0xCE, 0xFA, 0xED, 0xFE]),
        ("Mach-O (CF FA ED FE)", [0xCF, 0xFA, 0xED, 0xFE]),
    ];
    for (name, sig) in macho {
        if let Some(at) = find_subslice(sample, sig) {
            hits.push(MagicHit {
                name,
                at,
                strong: at == 0,
            });
        }
    }

    // zlib/deflate stream header is common: 0x78 0x01/0x5E/0x9C/0xDA
    if sample.len() >= 2 && sample[0] == 0x78 {
        let b1 = sample[1];
        if matches!(b1, 0x01 | 0x5E | 0x9C | 0xDA) {
            hits.push(MagicHit {
                name: "zlib/deflate (0x78 ?? header)",
                at: 0,
                strong: true,
            });
        }
    }

    hits
}

fn find_subslice(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    if hay.len() < needle.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}
