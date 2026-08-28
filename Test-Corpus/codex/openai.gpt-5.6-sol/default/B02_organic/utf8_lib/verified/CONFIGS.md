# Configuration surface

The runtime option is `replacement` (`false` deletes each invalid byte;
`true` emits `EF BF BD` for each invalid byte). Input shapes come from the
`valid_1`, `valid_2`, `valid_3`, and `valid_4` branches, their boundary
conditions, the valid-input `strdup` shortcut, first-invalid positioning, and
the 4096-byte replacement reserve.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `w_utf8_drop` | empty string (loop is skipped) | [x] |
| 2 | `w_utf8_drop` | nonempty ASCII-only strings (`valid_1`) | [x] |
| 3 | `w_utf8_drop` | valid two-byte sequences, including `C2 80` and `DF BF` boundaries | [x] |
| 4 | `w_utf8_drop` | ordinary valid three-byte sequences (`E1`-`EC`, `EE`-`EF`) | [x] |
| 5 | `w_utf8_drop` | `E0` three-byte sequences at/above second-byte lower bound `A0` | [x] |
| 6 | `w_utf8_drop` | `ED` three-byte sequences below second-byte upper bound `A0` | [x] |
| 7 | `w_utf8_drop` | ordinary valid four-byte sequences (`F1`-`F3`) | [x] |
| 8 | `w_utf8_drop` | `F0` four-byte sequences at/above second-byte lower bound `90` | [x] |
| 9 | `w_utf8_drop` | `F4` four-byte sequences at/below second-byte upper bound `8F` | [x] |
| 10 | `w_utf8_drop` | randomized mixtures of all valid widths; return points at terminator | [x] |
| 11 | `w_utf8_drop` | invalid first byte and valid-prefix-then-invalid forms: continuation, `C0`/`C1`, malformed/truncated 2/3/4-byte, surrogate, overlong, and above `F4`; return points at first invalid byte | [x] |
| 12 | `w_utf8_filter` | `replacement=false` and `true`; empty or all-valid randomized mixtures take the `strdup` path unchanged | [x] |
| 13 | `w_utf8_filter` | `replacement=false`; invalid bytes followed by ASCII exercise delete + `valid_1` copy | [x] |
| 14 | `w_utf8_filter` | `replacement=false`; invalid bytes followed by valid two-byte sequences exercise delete + `valid_2` copy | [x] |
| 15 | `w_utf8_filter` | `replacement=false`; invalid bytes followed by valid three-byte sequences exercise delete + `valid_3` copy | [x] |
| 16 | `w_utf8_filter` | `replacement=false`; invalid bytes followed by valid four-byte sequences exercise delete + `valid_4` copy | [x] |
| 17 | `w_utf8_filter` | `replacement=false`; randomized interleaved valid widths and malformed/truncated/overlong/surrogate/above-`F4` bytes | [x] |
| 18 | `w_utf8_filter` | `replacement=true`; invalid bytes followed by ASCII exercise replace + `valid_1` copy | [x] |
| 19 | `w_utf8_filter` | `replacement=true`; invalid bytes followed by valid two-byte sequences exercise replace + `valid_2` copy | [x] |
| 20 | `w_utf8_filter` | `replacement=true`; invalid bytes followed by valid three-byte sequences exercise replace + `valid_3` copy | [x] |
| 21 | `w_utf8_filter` | `replacement=true`; invalid bytes followed by valid four-byte sequences exercise replace + `valid_4` copy | [x] |
| 22 | `w_utf8_filter` | `replacement=true`; randomized interleaved valid widths and malformed/truncated/overlong/surrogate/above-`F4` bytes | [x] |
| 23 | `w_utf8_filter` | both option values; invalid byte after a randomized valid prefix and before a randomized valid suffix | [x] |
| 24 | `w_utf8_filter` | `replacement=true`; one invalid byte triggers the first `+4096` reallocation | [x] |
| 25 | `w_utf8_filter` | `replacement=true`; 1365 invalid bytes consume one replacement reserve to its `< 3` boundary | [x] |
| 26 | `w_utf8_filter` | `replacement=true`; 1366 invalid bytes trigger a second `+4096` reallocation | [x] |
| 27 | `w_utf8_filter` | both option values; large randomized mixed input above 4096 source bytes | [x] |
