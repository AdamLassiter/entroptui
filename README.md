# entroptui

A fast terminal UI for exploring large files by sliding a window across the bytes and visualizing entropy. Useful for spotting compressed/encrypted regions, text, padding, and container structure without loading the whole file into memory.

## Features

- **Streaming windowed reader**: seeks + reads only the current window from disk (handles very large files).
- **Entropy chart view**: per-bin entropy plot over the current window.
- **Hilbert map view**: entropy mapped onto a Hilbert curve "image" (heatmap).
- **Hex view**: hex dump of the start of the current window.
- **Heuristic suggestions**: ranked guesses (with reasons) based on entropy + quick statistics (magic bytes, text/base64/hex detection, compressed vs encrypted-ish hints).
- Extensible suggestion system via a `Heuristic` trait.

## Installation

Requires Rust (stable).

```bash
git clone <repo-url>
cd entroptui
cargo build --release
```

## Usage

```bash
cargo run --release -- path/to/file.bin
```

Optional args:

```bash
cargo run --release -- path/to/file.bin --window 1048576 --offset 0
```

- `--window`: initial window size in bytes (default: 1 MiB)
- `--offset`: initial offset in bytes (default: 0)

## Keyboard shortcuts

Navigation:

- `←` / `→`: scroll by **one visible hex page**
- `PageUp` / `PageDown`: scroll by **one full analysis window**
- `Home` / `End`: jump to start / end

Analysis parameters:

- `+` or `=`: zoom in (halve window size)
- `-`: zoom out (double window size)
- `b`: cycle entropy bucket size (1 → 2 → 4 bytes per symbol)

Views:

- `v`: toggle view (Chart → Hilbert → Hex)

Exit:

- `q` / `Esc`: quit
- `Ctrl+C`: quit

## What the views mean

- **Chart**: shows entropy (normalized 0..1) across the current window (binned to screen width).
- **Hilbert**: maps chunks of the current window onto a Hilbert curve grid; colors represent entropy (blue → cyan → green → yellow → red).
- **Hex**: shows a hex dump of the start of the current window.

## Suggestions

The "Suggestions" panel produces multiple candidates with confidence and *reasons*, using purely heuristic rules, including:

- magic byte signatures (PNG, ZIP, gzip, zstd, ELF, PDF, etc.)
- printable/UTF-8 detection
- base64-ish and hex-ish detection
- entropy + byte histogram uniformity to differentiate "compressed" vs "encrypted/random-ish"

In **Hex** view mode, feature extraction is based on the bytes currently visible in the hex viewer (the "hex window"), to keep the suggestions relevant to what you’re looking at.

## Notes / Limitations

- Entropy and heuristics are hints, not proof. Many formats mix regions (headers + compressed payloads).
- Very small windows can make entropy estimates noisy; zoom out for stability.
- Hilbert computation is multithreaded (rayon), but terminal rendering still has a cost for very large grids.

## Extending

Suggestions are modular. Add a new heuristic by implementing:

- `suggest::Heuristic::apply(&self, input, feats, out)`

and registering it in `SuggestEngine::default()`.

Potential future additions:

- byte histogram panel
- per-bin chi-square / runs tests
- file signature database
- region tagging/export (e.g., mark offsets of "high entropy" segments)
