# Configuration Surface

## Build-time configurations

`Cargo.toml` defines `default = []` and no optional features.
`c_src/CMakeLists.txt` defines no options, compile definitions, or conditional
sources. Therefore the complete feature combination set is:

| # | Cargo feature set | C configuration | [ ] |
|---|-------------------|-----------------|-----|
| B1 | `{}` (`--no-default-features`) | default, no CMake options | [x] |

## Runtime configurations

The only public library entry point is `process_buffer`. All lower-level
functions in `lib.c` are `static`, so they are exercised through this exported
pipeline. For rows 1-32, each mask is tested with many fixed-seed randomized
buffers and a cross-product sample of: lengths `1..=256`; bytes including
`0`/`255`, runs, duplicates, and arbitrary data; `param1` negative, zero,
`1`, boundary values through `255`, and values above `255`; and `param2` zero
and nonzero.

Flags run in this fixed order: rotate (`R`, `0x01`), compact (`C`, `0x02`),
deduplicate (`D`, `0x04`), interleave (`I`, `0x08`), reverse segments (`S`,
`0x10`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `process_buffer` | mask `0x00`: none; randomized full parameter/input sample | [x] |
| 2 | `process_buffer` | mask `0x01`: R; randomized full parameter/input sample | [x] |
| 3 | `process_buffer` | mask `0x02`: C; randomized full parameter/input sample | [x] |
| 4 | `process_buffer` | mask `0x03`: R+C; randomized full parameter/input sample | [x] |
| 5 | `process_buffer` | mask `0x04`: D; randomized full parameter/input sample | [x] |
| 6 | `process_buffer` | mask `0x05`: R+D; randomized full parameter/input sample | [x] |
| 7 | `process_buffer` | mask `0x06`: C+D; randomized full parameter/input sample | [x] |
| 8 | `process_buffer` | mask `0x07`: R+C+D; randomized full parameter/input sample | [x] |
| 9 | `process_buffer` | mask `0x08`: I; randomized full parameter/input sample | [x] |
| 10 | `process_buffer` | mask `0x09`: R+I; randomized full parameter/input sample | [x] |
| 11 | `process_buffer` | mask `0x0a`: C+I; randomized full parameter/input sample | [x] |
| 12 | `process_buffer` | mask `0x0b`: R+C+I; randomized full parameter/input sample | [x] |
| 13 | `process_buffer` | mask `0x0c`: D+I; randomized full parameter/input sample | [x] |
| 14 | `process_buffer` | mask `0x0d`: R+D+I; randomized full parameter/input sample | [x] |
| 15 | `process_buffer` | mask `0x0e`: C+D+I; randomized full parameter/input sample | [x] |
| 16 | `process_buffer` | mask `0x0f`: R+C+D+I; randomized full parameter/input sample | [x] |
| 17 | `process_buffer` | mask `0x10`: S; randomized full parameter/input sample | [x] |
| 18 | `process_buffer` | mask `0x11`: R+S; randomized full parameter/input sample | [x] |
| 19 | `process_buffer` | mask `0x12`: C+S; randomized full parameter/input sample | [x] |
| 20 | `process_buffer` | mask `0x13`: R+C+S; randomized full parameter/input sample | [x] |
| 21 | `process_buffer` | mask `0x14`: D+S; randomized full parameter/input sample | [x] |
| 22 | `process_buffer` | mask `0x15`: R+D+S; randomized full parameter/input sample | [x] |
| 23 | `process_buffer` | mask `0x16`: C+D+S; randomized full parameter/input sample | [x] |
| 24 | `process_buffer` | mask `0x17`: R+C+D+S; randomized full parameter/input sample | [x] |
| 25 | `process_buffer` | mask `0x18`: I+S; randomized full parameter/input sample | [x] |
| 26 | `process_buffer` | mask `0x19`: R+I+S; randomized full parameter/input sample | [x] |
| 27 | `process_buffer` | mask `0x1a`: C+I+S; randomized full parameter/input sample | [x] |
| 28 | `process_buffer` | mask `0x1b`: R+C+I+S; randomized full parameter/input sample | [x] |
| 29 | `process_buffer` | mask `0x1c`: D+I+S; randomized full parameter/input sample | [x] |
| 30 | `process_buffer` | mask `0x1d`: R+D+I+S; randomized full parameter/input sample | [x] |
| 31 | `process_buffer` | mask `0x1e`: C+D+I+S; randomized full parameter/input sample | [x] |
| 32 | `process_buffer` | mask `0x1f`: R+C+D+I+S; randomized full parameter/input sample | [x] |
| 33 | `process_buffer` | unknown flag bits `0xffffffe0`, alone and combined with `0x1f`, are ignored | [x] |
| 34 | `process_buffer` | R: `length == 1` reaches rotate's `len <= 1` guard | [x] |
| 35 | `process_buffer` | R: `param1 % length == 0` skips rotation | [x] |
| 36 | `process_buffer` | R: negative offset is normalized by adding `length` | [x] |
| 37 | `process_buffer` | R: normalized offset `< length / 2` uses prefix/move/restore path | [x] |
| 38 | `process_buffer` | R: normalized offset `>= length / 2` uses right-side path | [x] |
| 39 | `process_buffer` | C: `param1 <= 0` selects default threshold `3` | [x] |
| 40 | `process_buffer` | C: `param1 == 1`; singleton runs expand to value/count pairs | [x] |
| 41 | `process_buffer` | C: valid threshold `2..=255`, run length below threshold | [x] |
| 42 | `process_buffer` | C: run length exactly threshold | [x] |
| 43 | `process_buffer` | C: run length above threshold with trailing bytes moved | [x] |
| 44 | `process_buffer` | C: `param1 > 255` selects default threshold `3` | [x] |
| 45 | `process_buffer` | C: run length `256` is capped to count `255` | [x] |
| 46 | `process_buffer` | D: `length == 1` reaches deduplicate's early return | [x] |
| 47 | `process_buffer` | D: `param2 == 0`, seen-table/swap path, duplicates and unique bytes | [x] |
| 48 | `process_buffer` | D: `param2 != 0`, order-preserving search/move path | [x] |
| 49 | `process_buffer` | I: effective length `< 2` skips interleave | [x] |
| 50 | `process_buffer` | I: even length, `half <= 256`, temporary-buffer path | [x] |
| 51 | `process_buffer` | I: odd length, `half <= 256`, trailing-byte path | [x] |
| 52 | `process_buffer` | I: even length `514`, `half > 256`, in-place path | [x] |
| 53 | `process_buffer` | I: odd length `515`, `half > 256`, in-place path | [x] |
| 54 | `process_buffer` | S: effective length `< 4` skips reverse-segments dispatch | [x] |
| 55 | `process_buffer` | S: `param1 <= 0` selects segment size `4` | [x] |
| 56 | `process_buffer` | S: `param1 == 1` reaches segment-size `<= 1` guard | [x] |
| 57 | `process_buffer` | S: positive segment size greater than effective length is skipped | [x] |
| 58 | `process_buffer` | S: complete segments with remainder `0` | [x] |
| 59 | `process_buffer` | S: complete segments with remainder `1` | [x] |
| 60 | `process_buffer` | S: complete segments with remainder greater than `1` | [x] |
| 61 | `process_buffer` | C+I+S: compaction shrinks effective length below later guards | [x] |
| 62 | `process_buffer` | R+C+S: one `param1` simultaneously controls offset, threshold, and segment size | [x] |
| 63 | `process_buffer` | all operations: byte boundaries `0`/`255`, runs, duplicates, odd/even lengths | [x] |
| 64 | `process_buffer` | R: length `700`, small-path offset `300`; offset `>= 256` uses multiple 256-byte chunks | [x] |
