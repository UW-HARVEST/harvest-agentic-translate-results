# Configuration-surface table

Mechanically derived from the public header, the two exported entry points,
the `valid_1`/`valid_2`/`valid_3`/`valid_4` predicates, the `replacement`
branch, invalid-byte position/count, and the `REPLACEMENT_INC == 4096`
growth branch in `../c_src/src/lib.c`.

There are no Cargo features in `Cargo.toml`; the build configurations to verify
are the default build and the equivalent `--no-default-features` build.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `w_utf8_drop` | empty C string; loop terminates immediately | [x] |
| 2 | `w_utf8_drop` | one ASCII byte (`0xxxxxxx`) | [x] |
| 3 | `w_utf8_drop` | many ASCII bytes | [x] |
| 4 | `w_utf8_drop` | valid 2-byte sequence, including lead boundaries `C2` and `DF` | [x] |
| 5 | `w_utf8_drop` | valid 3-byte `E0` sequence with second byte at/above `A0` | [x] |
| 6 | `w_utf8_drop` | valid 3-byte `ED` sequence with second byte below `A0` | [x] |
| 7 | `w_utf8_drop` | valid generic 3-byte sequence (`E1..EC`, `EE..EF`) | [x] |
| 8 | `w_utf8_drop` | valid 4-byte `F0` sequence with second byte at/above `90` | [x] |
| 9 | `w_utf8_drop` | valid 4-byte `F4` sequence with second byte at/below `8F` | [x] |
| 10 | `w_utf8_drop` | valid generic 4-byte sequence (`F1..F3`) | [x] |
| 11 | `w_utf8_drop` | mixed one-/two-/three-/four-byte valid sequences, one and many elements | [x] |
| 12 | `w_utf8_drop` | isolated continuation byte (`80..BF`) at offset zero | [x] |
| 13 | `w_utf8_drop` | forbidden overlong 2-byte lead (`C0` or `C1`) | [x] |
| 14 | `w_utf8_drop` | `C2..DF` lead followed by a non-continuation byte | [x] |
| 15 | `w_utf8_drop` | 3-byte lead with first or second continuation malformed | [x] |
| 16 | `w_utf8_drop` | `E0` followed by byte below `A0` (overlong boundary) | [x] |
| 17 | `w_utf8_drop` | `ED` followed by byte at/above `A0` (surrogate boundary) | [x] |
| 18 | `w_utf8_drop` | 4-byte lead with any continuation malformed | [x] |
| 19 | `w_utf8_drop` | `F0` followed by byte below `90` (overlong boundary) | [x] |
| 20 | `w_utf8_drop` | `F4` followed by byte above `8F` (above Unicode maximum) | [x] |
| 21 | `w_utf8_drop` | invalid lead `F5..FF` | [x] |
| 22 | `w_utf8_drop` | invalid byte after a randomized valid prefix; returned pointer offset is compared | [x] |
| 23 | `w_utf8_filter` | entirely valid empty input; `replacement` is not consulted | [x] |
| 24 | `w_utf8_filter` | entirely valid ASCII input, one and many bytes; `replacement` is not consulted | [x] |
| 25 | `w_utf8_filter` | entirely valid 2-byte sequences, including `C2`/`DF` boundaries | [x] |
| 26 | `w_utf8_filter` | entirely valid 3-byte sequences, including `E0`/`ED` special boundaries | [x] |
| 27 | `w_utf8_filter` | entirely valid 4-byte sequences, including `F0`/`F4` special boundaries | [x] |
| 28 | `w_utf8_filter` | entirely valid mixed-width input | [x] |
| 29 | `w_utf8_filter` | one invalid byte at start, `replacement == false` | [x] |
| 30 | `w_utf8_filter` | one invalid byte at start, `replacement == true` | [x] |
| 31 | `w_utf8_filter` | one invalid byte after a valid prefix, `replacement == false` | [x] |
| 32 | `w_utf8_filter` | one invalid byte after a valid prefix, `replacement == true` | [x] |
| 33 | `w_utf8_filter` | invalid byte followed by valid 1-byte data, both replacement modes | [x] |
| 34 | `w_utf8_filter` | invalid byte followed by valid 2-byte data, both replacement modes | [x] |
| 35 | `w_utf8_filter` | invalid byte followed by valid 3-byte data, both replacement modes | [x] |
| 36 | `w_utf8_filter` | invalid byte followed by valid 4-byte data, both replacement modes | [x] |
| 37 | `w_utf8_filter` | malformed 2-byte classes (`C0/C1`, bad continuation), both replacement modes | [x] |
| 38 | `w_utf8_filter` | malformed 3-byte continuations, both replacement modes | [x] |
| 39 | `w_utf8_filter` | `E0` overlong and `ED` surrogate boundary violations, both replacement modes | [x] |
| 40 | `w_utf8_filter` | malformed 4-byte continuations, both replacement modes | [x] |
| 41 | `w_utf8_filter` | `F0` overlong, `F4` above-max, and `F5..FF` leads, both replacement modes | [x] |
| 42 | `w_utf8_filter` | many invalid bytes, `replacement == false`; all invalid bytes are dropped | [x] |
| 43 | `w_utf8_filter` | `1..1365` invalid bytes, `replacement == true`; one growth allocation suffices | [x] |
| 44 | `w_utf8_filter` | at least `1366` invalid bytes, `replacement == true`; `repl < 3` triggers another 4096-byte growth | [x] |
| 45 | `w_utf8_filter` | randomized mixed valid/invalid bytes and positions, `replacement == false` | [x] |
| 46 | `w_utf8_filter` | randomized mixed valid/invalid bytes and positions, `replacement == true` | [x] |

