# CONFIGS.md — Phase A configuration-surface table

## Axes the C code actually branches on

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

**Runtime options / modes / flags: NONE.** The public header is one line —
`char *decode_base64(const char *src);`. There is no context struct, no option
setter, no flags argument, no `#ifdef`, and no compile-time configuration in
`CMakeLists.txt`. The only `#define`s are `TRUE`/`FALSE`. Consequently the
configuration surface is entirely the *shape of the input string*.

**Full set of public entry points (1):** `decode_base64`. The two remaining
functions, `decode` and `is_base64`, are `static` — they are the lowest-level
routines and are unreachable across the FFI boundary, so they are driven
*through* `decode_base64` by inputs chosen to hit each of their branches
individually (axis A2/A3 below).

| axis | what the C branches on | source |
|------|------------------------|--------|
| A1 | `src` non-`NULL` and non-empty | `if (src && *src)` line 46 |
| A2 | `is_base64` character class: `A-Z`, `a-z`, `0-9`, `+`, `/`, `=` vs. everything else (ignored) | lines 31–37, 67–71 |
| A3 | `decode` character class: `A-Z` → `c-'A'`; `a-z` → `c-'a'+26`; `0-9` → `c-'0'+52`; `'+'` → 62; **fall-through** (`'/'`, `'='`) → 63 | lines 12–25 |
| A4 | filtered length `l` mod 4 — controls how many of `c2`,`c3`,`c4` keep their `'A'` default | `if (k+n < l)` lines 79–89 |
| A5 | `c3 == '='` → 2nd output byte suppressed | line 98 |
| A6 | `c4 == '='` → 3rd output byte suppressed | line 102 |
| A7 | `l == 0` after filtering (non-empty input, no base64 chars) → decode loop body never runs | line 73 |
| A8 | sign of `char`: bytes `0x80..0xFF` are *negative*, so they fail every range check in both `is_base64` and `decode` | signed `char` on x86-64 |
| A9 | input length — drives `strlen`, the `calloc(1, l+13)` / `malloc(l)` sizes, and the number of quartets | lines 49–60 |
| A10 | `'\0'` inside the buffer → `strlen`/loop stop early | lines 49, 67 |

Rows below are the cross-product of these axes pruned to the combinations the C
actually distinguishes. **Every row is exercised with many randomized inputs
(fixed seed) — not one hand-picked value.** Output comparison is byte-for-byte
over the *entire* `strlen(src)+14`-byte allocation (both sides `calloc`, so the
tail is defined), never merely as a C string, so bytes past an embedded `NUL`
are compared too.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| B01 | `decode_base64` | `l % 4 == 0`, no padding, uppercase-only alphabet (A3 branch 1) | [x] |
| B02 | `decode_base64` | `l % 4 == 0`, no padding, lowercase-only alphabet (A3 branch 2) | [x] |
| B03 | `decode_base64` | `l % 4 == 0`, no padding, digits-only alphabet (A3 branch 3) | [x] |
| B04 | `decode_base64` | `l % 4 == 0`, no padding, `'+'`-only alphabet (A3 branch 4) | [x] |
| B05 | `decode_base64` | `l % 4 == 0`, no padding, `'/'`-only alphabet (A3 fall-through → 63) | [x] |
| B06 | `decode_base64` | `l % 4 == 0`, no padding, all 64 alphabet chars mixed randomly | [x] |
| B07 | `decode_base64` | `l % 4 == 1` (only `c1` real; `c2`,`c3`,`c4` default to `'A'`) | [x] |
| B08 | `decode_base64` | `l % 4 == 2` (`c1`,`c2` real; `c3`,`c4` default) | [x] |
| B09 | `decode_base64` | `l % 4 == 3` (`c1`,`c2`,`c3` real; `c4` defaults) | [x] |
| B10 | `decode_base64` | `l == 1`, `l == 2`, `l == 3`, `l == 4`, `l == 5` exhaustively — smallest quartets | [x] |
| B11 | `decode_base64` | canonical single `'='` pad at end (A6: `c4=='='`, 3rd byte suppressed) | [x] |
| B12 | `decode_base64` | canonical double `'=='` pad at end (A5+A6: 2nd *and* 3rd byte suppressed) | [x] |
| B13 | `decode_base64` | `'='` in the *middle* of the stream — C does **not** stop at padding; decoding continues past it | [x] |
| B14 | `decode_base64` | `'='` at position `k+1` only (`c2=='='` → `decode`=63, both later bytes still emitted) | [x] |
| B15 | `decode_base64` | leading `'='` (first char of the first quartet is padding) | [x] |
| B16 | `decode_base64` | input consisting *only* of `'='` chars, lengths 1..8 | [x] |
| B17 | `decode_base64` | randomized `'='` sprinkled at random positions among random alphabet chars | [x] |
| B18 | `decode_base64` | valid base64 with **leading** ignored (non-base64) characters | [x] |
| B19 | `decode_base64` | valid base64 with **trailing** ignored characters | [x] |
| B20 | `decode_base64` | valid base64 with ignored characters **interspersed** (incl. `\n`, `\r`, space, tab — the MIME line-wrap shape) | [x] |
| B21 | `decode_base64` | A7: non-empty input with **zero** base64 chars (`l == 0`) → returns non-`NULL` all-zero buffer | [x] |
| B22 | `decode_base64` | A8: bytes `0x80..=0xFF` only (negative `char`) → all ignored, `l == 0` | [x] |
| B23 | `decode_base64` | A8: bytes `0x80..=0xFF` **mixed with** valid base64 chars (sign-extension trap) | [x] |
| B24 | `decode_base64` | control bytes `0x01..=0x1F` and `0x7F` mixed with valid chars | [x] |
| B25 | `decode_base64` | **every** single-byte input `0x01..=0xFF` (255 one-char strings, exhaustive per-character sweep of A2/A3/A8) | [x] |
| B26 | `decode_base64` | **every** two-byte input `0x01..=0xFF` × `0x01..=0xFF` (65 025 pairs, exhaustive boundary sweep) | [x] |
| B27 | `decode_base64` | fully random bytes `0x01..=0xFF`, random length 1..64 (all axes interacting) | [x] |
| B28 | `decode_base64` | fully random bytes `0x01..=0xFF`, random length 1..4096 (many quartets) | [x] |
| B29 | `decode_base64` | A10: `'\0'` embedded mid-buffer — decode must stop there, and the bytes *after* the terminator in the output buffer must match too | [x] |
| B30 | `decode_base64` | A9: large inputs — 4 KiB, 64 KiB, 1 MiB of valid base64 (allocation-size arithmetic, many quartets) | [x] |
| B31 | `decode_base64` | output containing embedded `NUL` bytes (`"AAAA"` → `00 00 00`) — verifies full-buffer, not string, equality | [x] |
| B32 | `decode_base64` | round-trip: encode random binary payloads with a reference base64 encoder, then decode (real-consumer end-to-end shape), all 4 pad classes | [x] |
| B33 | `decode_base64` | repeated / interleaved calls on the same loaded library (no cross-call state; buffers independent, both libs alive simultaneously) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (empty) feature set. `cargo check`/`cargo test
--no-default-features` are still run in Phase D to prove there is no hidden
feature-gated path. The C side likewise has no build options in `CMakeLists.txt`
(no `option()`, no `target_compile_definitions`).

## Row → test mapping

Every row `Bnn` above is implemented by the identically-numbered test function
in `tests/valid_paths.rs` (`b01_…` … `b33_…`), each looping over many randomized
inputs from a fixed-seed xorshift64* PRNG. A row is ticked only after it passes
across all of those inputs under every configuration in `check_all_configs.sh`.

Beyond the per-row content comparison, `tests/error_paths.rs::shim_child`
(`[E-trace]`) compares the *allocator traffic* of the two implementations for
210 inputs: the exact `calloc` and `malloc` byte counts and the number of
`calloc`/`malloc`/`free` calls. This is what makes a wrong allocation size or a
leak visible even when the returned bytes happen to agree.
