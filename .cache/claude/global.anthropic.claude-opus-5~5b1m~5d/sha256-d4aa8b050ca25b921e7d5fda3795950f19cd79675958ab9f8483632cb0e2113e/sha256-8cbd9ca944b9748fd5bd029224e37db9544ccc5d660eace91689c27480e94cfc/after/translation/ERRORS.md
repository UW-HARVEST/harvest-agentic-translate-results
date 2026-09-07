# ERRORS.md — error-surface table

Mechanical grep of the entire C source (`c_src/src/lib.c`, `c_src/include/lib.h`)
for rejection mechanisms:

```
grep -n 'return\|assert\|ERROR\|NULL\|if (\|if(\|MAX\|MIN\|<=\|>=' c_src/src/lib.c c_src/include/lib.h
```

Findings: the library contains **exactly one** function, `crc16`, and it has:

* `return` statements: 1 (`return crc16;` — the success/only path)
* `assert`: 0
* error enums / error-return macros (`RETURN_ERROR`, `return -1`, `return NULL`): 0
* explicit range checks / null checks: 0
* MIN/MAX constants: 0
* the only conditionals are the two loop conditions `len >= 8` and `len--`,
  which are control flow, not rejection.

So `crc16` has **no error surface**: it is total over its documented domain and
returns a `tflac_u16` for every input. The rows below therefore enumerate the
*generic* C-API boundary conditions that must nevertheless behave identically
(they are "non-rejections" — both implementations must agree on the returned
value rather than on an error code).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `crc16` | `len == 0`, `d` a valid pointer | returns the seed `crc16` unchanged; `d` never dereferenced |
| 2 | `crc16` | `len == 0`, `d == NULL` (null pointer) | returns the seed unchanged; no dereference, no crash |
| 3 | `crc16` | `len == 0`, `d == NULL`, seed `0xFFFF` (max seed) | returns `0xFFFF` |
| 4 | `crc16` | `len == 1` with a 1-byte buffer (minimum non-empty length; tail loop only, slice-by-8 loop skipped) | one tail iteration: `(crc<<8) ^ T0[(crc>>8)^d[0]]` |
| 5 | `crc16` | `len == 7` (one below the slice-by-8 threshold — boundary one step below the "valid range" of the fast loop) | 7 tail iterations only |
| 6 | `crc16` | `len == 8` (exact threshold) | exactly one slice-by-8 iteration, no tail iterations |
| 7 | `crc16` | `len == 9` (one step past the threshold) | one slice-by-8 iteration + 1 tail iteration |
| 8 | `crc16` | seed out of "expected" sub-range: seed `0xFFFF` / `0x8000` / `0x00FF` (high byte 0) with non-empty data — exercises `crc >> 8` and `crc & 0xFF` index extremes (table index 255 and 0) | value-dependent result, no out-of-range table index (index is always masked to 0..255) |
| 9 | `crc16` | `len` "oversized" relative to the pointer only in the sense of a large count: `len = 1 << 20` over a real 1 MiB buffer (largest length exercised; `tflac_u32` upper region) | full slice-by-8 traversal, deterministic result |
| 10 | `crc16` | data bytes at the extremes `0x00` and `0xFF` for every position of the 8-byte group (drives table indices 0 and 255 in all eight tables) | deterministic result |
| 11 | `crc16` | unaligned `d` (pointer offset 1..7 into a buffer) — C makes no alignment assumption | same result as the aligned copy of the same bytes |
| 12 | `crc16` | out-of-range "enum"/flag values passed across FFI | **N/A** — the API takes no enum, mode, or flag parameter; `tflac_u16`/`tflac_u32` accept their full value range, all of which is covered by rows 1–11 |

Every row is covered by a differential test in
`tests/differential.rs` (module `error_paths`), asserting the C and Rust
`.so` exports return the *same* `tflac_u16` (there is no error code or sentinel
in this API — the returned CRC is the observable).
