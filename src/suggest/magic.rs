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
    let mut hits = Vec::new();

    let mut push = |name: &'static str, at: usize| {
        hits.push(MagicHit {
            name,
            at,
            strong: at == 0,
        });
    };

    // Exact byte signatures we scan anywhere in the sample.
    // (If you find this too chatty, restrict to at==0 for most entries.)
    let magics: &[(&'static str, &'static [u8])] = &[
        // --- Images ---
        ("PNG", b"\x89PNG\r\n\x1a\n"),
        ("JPEG", b"\xFF\xD8\xFF"),
        ("GIF87a", b"GIF87a"),
        ("GIF89a", b"GIF89a"),
        ("BMP", b"BM"),
        ("TIFF (LE)", b"II*\x00"),
        ("TIFF (BE)", b"MM\x00*"),
        ("ICO", b"\x00\x00\x01\x00"),
        ("CUR", b"\x00\x00\x02\x00"),
        ("Photoshop PSD", b"8BPS"),
        ("OpenEXR", b"\x76\x2F\x31\x01"),

        // --- Archives / packaging ---
        ("ZIP (local file header)", b"PK\x03\x04"),
        ("ZIP (central directory)", b"PK\x01\x02"),
        ("ZIP (end of central dir)", b"PK\x05\x06"),
        ("7-Zip", b"7z\xBC\xAF\x27\x1C"),
        ("RAR (v1.5+)", b"Rar!\x1A\x07\x00"),
        ("RAR (v5+)", b"Rar!\x1A\x07\x01\x00"),
        ("CAB", b"MSCF"),
        ("ar archive", b"!<arch>\n"),

        // --- Compression ---
        ("GZIP", b"\x1F\x8B"),
        ("Zstandard", b"\x28\xB5\x2F\xFD"),
        ("XZ", b"\xFD\x37\x7A\x58\x5A\x00"),
        ("Bzip2", b"BZh"),
        ("LZ4 frame", b"\x04\x22\x4D\x18"),
        ("LZIP", b"LZIP"),
        ("Unix compress (.Z)", b"\x1F\x9D"),
        ("LZMA (.lzma)", b"\x5D\x00\x00\x80\x00"),

        // --- Documents / structured data ---
        ("PDF", b"%PDF-"),
        ("PostScript", b"%!PS-Adobe-"),
        ("SQLite 3", b"SQLite format 3\x00"),
        ("Apache Parquet", b"PAR1"),
        ("Apache ORC", b"ORC"),
        ("bplist (binary plist)", b"bplist00"),
        ("Microsoft OLE2/CFBF", b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1"),

        // --- Executables / bytecode ---
        ("ELF", b"\x7FELF"),
        ("PE/COFF (MZ)", b"MZ"),
        ("Wasm", b"\x00asm"),
        ("Java class", b"\xCA\xFE\xBA\xBE"),
        ("Dalvik DEX", b"dex\n035\0"),
        ("Dalvik DEX (036)", b"dex\n036\0"),

        // --- Fonts ---
        ("TrueType font (TTF)", b"\x00\x01\x00\x00"),
        ("OpenType font (OTF)", b"OTTO"),
        ("WOFF", b"wOFF"),
        ("WOFF2", b"wOF2"),

        // --- Media ---
        ("Ogg", b"OggS"),
        ("FLAC", b"fLaC"),
        ("MIDI", b"MThd"),
        ("Matroska/WebM (EBML)", b"\x1A\x45\xDF\xA3"),
        ("MP3 (ID3 tag)", b"ID3"),

        // --- PCAP ---
        ("pcap (LE)", b"\xD4\xC3\xB2\xA1"),
        ("pcap (BE)", b"\xA1\xB2\xC3\xD4"),
        ("pcapng", b"\x0A\x0D\x0D\x0A"),

        // --- Text encodings / wrappers ---
        ("UTF-8 BOM", b"\xEF\xBB\xBF"),
        ("UTF-16 LE BOM", b"\xFF\xFE"),
        ("UTF-16 BE BOM", b"\xFE\xFF"),
        ("OpenSSL salted", b"Salted__"),
        ("PEM (BEGIN ...)", b"-----BEGIN "),
        ("XML (declaration)", b"<?xml"),
        ("Shebang script", b"#!"),
    ];

    for (name, sig) in magics {
        if let Some(at) = find_subslice(sample, sig) {
            push(name, at);
        }
    }

    // Mach-O (common magics)
    let macho: &[(&'static str, [u8; 4])] = &[
        ("Mach-O (FE ED FA CE)", [0xFE, 0xED, 0xFA, 0xCE]),
        ("Mach-O (FE ED FA CF)", [0xFE, 0xED, 0xFA, 0xCF]),
        ("Mach-O (CE FA ED FE)", [0xCE, 0xFA, 0xED, 0xFE]),
        ("Mach-O (CF FA ED FE)", [0xCF, 0xFA, 0xED, 0xFE]),
    ];
    for (name, sig) in macho {
        if let Some(at) = find_subslice(sample, &sig) {
            push(name, at);
        }
    }

    // zlib/deflate stream header is common: 0x78 0x01/0x5E/0x9C/0xDA
    if sample.len() >= 2 && sample[0] == 0x78 {
        let b1 = sample[1];
        if matches!(b1, 0x01 | 0x5E | 0x9C | 0xDA) {
            push("zlib/deflate (0x78 ?? header)", 0);
        }
    }

    // RIFF-based formats: "RIFF" + 4-byte size + type at offset 8
    if sample.len() >= 12 && &sample[0..4] == b"RIFF" {
        let kind = &sample[8..12];
        let name = match kind {
            b"WAVE" => "WAV (RIFF/WAVE)",
            b"AVI " => "AVI (RIFF/AVI)",
            b"WEBP" => "WebP (RIFF/WEBP)",
            _ => "RIFF container",
        };
        push(name, 0);
    }

    // MP4/QuickTime family usually has: [size:4] "ftyp" at offset 4
    if sample.len() >= 12 && &sample[4..8] == b"ftyp" {
        push("ISO BMFF / MP4 (ftyp)", 4);
    }

    // TAR: "ustar" at offset 257 (classic ustar)
    if sample.len() >= 262 && &sample[257..262] == b"ustar" {
        push("TAR (ustar)", 257);
    }

    // MPEG-TS: sync byte 0x47 every 188 bytes (check first 2 packets)
    if sample.len() > 376 && sample[0] == 0x47 && sample[188] == 0x47 && sample[376] == 0x47 {
        push("MPEG-TS (188-byte packets)", 0);
    }

    // EXT2/3/4 superblock magic 0xEF53 at offset 1024+56 = 1080
    if sample.len() >= 1082 {
        let lo = sample[1080];
        let hi = sample[1081];
        if lo == 0x53 && hi == 0xEF {
            push("ext2/3/4 filesystem (EF53)", 1080);
        }
    }

    // MBR signature 0x55AA at offset 510
    if sample.len() >= 512 && sample[510] == 0x55 && sample[511] == 0xAA {
        push("MBR boot sector (0x55AA)", 510);
    }

    // GPT header: "EFI PART" at LBA1, offset 512
    if sample.len() >= 520 && &sample[512..520] == b"EFI PART" {
        push("GPT header (EFI PART)", 512);
    }

    // ISO9660 primary volume descriptor identifier "CD001" at 0x8001
    if sample.len() >= 0x8006 && &sample[0x8001..0x8006] == b"CD001" {
        push("ISO9660 (CD001)", 0x8001);
    }

    // Safer JSON “magic”: after optional ASCII whitespace, starts with { or [
    // and the first chunk is mostly printable.
    {
        let mut i = 0usize;
        while i < sample.len() && matches!(sample[i], b' ' | b'\t' | b'\r' | b'\n') {
            i += 1;
        }
        if i < sample.len() && (sample[i] == b'{' || sample[i] == b'[') {
            let look = &sample[i..sample.len().min(i + 64)];
            let printable = look
                .iter()
                .filter(|&&b| (0x20..=0x7E).contains(&b) || matches!(b, b' ' | b'\t' | b'\r' | b'\n'))
                .count();
            if printable as f64 / look.len().max(1) as f64 > 0.95 {
                push("JSON (text, heuristic)", i);
            }
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
