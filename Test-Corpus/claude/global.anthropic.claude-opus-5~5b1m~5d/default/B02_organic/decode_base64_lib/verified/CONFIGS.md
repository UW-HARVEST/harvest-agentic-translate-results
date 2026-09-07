# CONFIGS.md — Configuration-surface table (valid inputs)

## Axes the C actually branches on

Derived from `c_src/include/lib.h` (the whole public API is one function) and
every `if` / range check in `c_src/src/lib.c`.

There are **no** runtime options, flags, modes, `#ifdef`s, enums, or context
structs in this library. The only public entry point is:

```c
char *decode_base64(const char *src);          /* include/lib.h:1 */
```

So the configuration surface is entirely made of **input shapes**. The branches
the code takes are:

| axis | source location | distinct values |
|------|-----------------|-----------------|
| A. `src` non-null & non-empty | line 46 | yes / no (no ⇒ ERRORS.md rows 1-2) |
| B. `l` = number of surviving base64 chars, mod 4 | lines 73, 79, 83, 87 | 0, 1, 2, 3 — selects how many of `c2 c3 c4` default to `'A'` |
| C. `l` magnitude | line 73 | 0 (all chars filtered), 1, 2, 3, 4, 5..8 (multi-group), large |
| D. `c3 == '='` | line 98 | yes ⇒ 2nd output byte suppressed / no ⇒ emitted |
| E. `c4 == '='` | line 102 | yes ⇒ 3rd output byte suppressed / no ⇒ emitted |
| F. `decode()` class of each char | lines 12-25 | `A-Z` (0-25), `a-z` (26-51), `0-9` (52-61), `'+'` (62), everything-else-that-survives-the-filter i.e. `'/'` and `'='` (63) |
| G. `is_base64()` verdict per byte | lines 31-37 | kept (`A-Za-z0-9+/=`) / dropped (all other bytes, incl. `0x80..0xFF` which are **negative** `char` on x86-64) |
| H. `'='` position | lines 74-104 | none / trailing-1 / trailing-2 / **interior** (C does *not* stop at padding; interior `'='` decodes as 63 and only suppresses bytes when it lands on the c3/c4 slot of its group) |
| I. output contains embedded `0x00` | — | yes / no (buffer is NUL-terminated by `calloc`, so the byte *after* the payload is 0 too; comparison must be over the full `strlen+14` region, not `strcmp`) |
| J. filtered-out bytes interleaved | lines 67-71 | none / leading / interior / trailing / only-junk |

`decode_base64` is simultaneously the lowest-level and the highest-level public
entry point; the two `static` helpers (`decode`, `is_base64`) are not exported,
so axes F and G are driven through it.

## Row table

Every row is exercised with **many randomized inputs** (fixed seed, xorshift64\*
PRNG in `tests/common/mod.rs`) unless it is inherently a single input.
Comparison is byte-for-byte over the entire `strlen(src) + 14` allocation plus
the NULL/non-NULL verdict.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `decode_base64` | `l % 4 == 0`, no `'='`, alphabet `A-Za-z0-9+/`, lengths 4,8,...,64 — randomized | [x] |
| 2 | `decode_base64` | `l % 4 == 1` (axis B=1: `c2=c3=c4='A'`), randomized alphabet | [x] |
| 3 | `decode_base64` | `l % 4 == 2` (axis B=2: `c3=c4='A'`), randomized alphabet | [x] |
| 4 | `decode_base64` | `l % 4 == 3` (axis B=3: `c4='A'`), randomized alphabet | [x] |
| 5 | `decode_base64` | `l == 1` — single base64 char, all 64 alphabet chars + `'='` swept | [x] |
| 6 | `decode_base64` | `l == 2`, `l == 3` exhaustive-ish random sweep of both chars | [x] |
| 7 | `decode_base64` | canonical padding `xxx=` (axis E: `c4=='='`, 3rd byte suppressed), randomized prefix | [x] |
| 8 | `decode_base64` | canonical padding `xx==` (axes D+E: `c3=='='` and `c4=='='`, both trailing bytes suppressed), randomized prefix | [x] |
| 9 | `decode_base64` | `'='` in the **c1** slot of a group (interior padding, decodes to 63, no suppression) | [x] |
| 10 | `decode_base64` | `'='` in the **c2** slot of a group (decodes to 63, no suppression) | [x] |
| 11 | `decode_base64` | `'='` in the **c3** slot of a *non-final* group (suppresses byte 2 of that group but decoding continues) | [x] |
| 12 | `decode_base64` | multiple `'='` scattered at random positions and random counts | [x] |
| 13 | `decode_base64` | alphabet restricted to `A-Z` only (axis F class 0-25) | [x] |
| 14 | `decode_base64` | alphabet restricted to `a-z` only (axis F class 26-51) | [x] |
| 15 | `decode_base64` | alphabet restricted to `0-9` only (axis F class 52-61) | [x] |
| 16 | `decode_base64` | alphabet restricted to `'+'` / `'/'` only (axis F classes 62 and 63) | [x] |
| 17 | `decode_base64` | full 64-char alphabet, boundary chars `A Z a z 0 9 + /` at every position of a 4-group (exhaustive 8×4) | [x] |
| 18 | `decode_base64` | valid base64 with **dropped** bytes interleaved (axis G/J): random junk from `0x01..0x7F \ alphabet` | [x] |
| 19 | `decode_base64` | valid base64 with **high-bit** bytes `0x80..0xFF` interleaved (negative `char`, must be dropped) | [x] |
| 20 | `decode_base64` | leading-only junk, interior-only junk, trailing-only junk (three shapes) | [x] |
| 21 | `decode_base64` | **only** junk, non-empty (axis C=0 → `l==0`, decode loop never runs, all-zero buffer returned, non-NULL) | [x] |
| 22 | `decode_base64` | fully random bytes `0x01..0xFF`, random length 1..300 — the unconstrained fuzz row | [x] |
| 23 | `decode_base64` | random bytes biased to be ~70 % base64 chars, random length 1..1024 | [x] |
| 24 | `decode_base64` | input that decodes to bytes containing embedded `0x00` (axis I) — e.g. `"AAAA"`, `"AA=="`, random zero-heavy payloads | [x] |
| 25 | `decode_base64` | input whose decoded payload spans all 256 output byte values (round-trip of `0x00..0xFF` encoded properly) | [x] |
| 26 | `decode_base64` | large input: 4 KiB, 16 KiB, 64 KiB of random base64 (axis C large) | [x] |
| 27 | `decode_base64` | large input consisting entirely of `'='` (64 KiB) — every group suppresses bytes 2 and 3 | [x] |
| 28 | `decode_base64` | every single-byte input `0x01..0xFF` (256-value sweep through the one pointer parameter) | [x] |
| 29 | `decode_base64` | every 2-byte input over a reduced representative alphabet ∪ junk (exhaustive pair sweep) | [x] |
| 30 | `decode_base64` | input length exactly at group boundaries ±1: 3,4,5,7,8,9,11,12,13 pure base64 chars | [x] |

## Binary / driver

`c_src/CMakeLists.txt` builds **only** `add_library(driver SHARED src/lib.c)` —
there is no `add_executable`, and `translation/Cargo.toml` declares
`crate-type = ["cdylib"]` with no `[[bin]]`. **No binary to diff.**

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` section, so the only
configuration is the default (empty) feature set. `cargo test`,
`cargo test --no-default-features`, and `cargo test --all-features` are all the
same build; the automation script runs all three anyway.

## Verification run (Phase B)

`tests/phase_b_configs.rs` — **30/30 rows PASS**, **113 069** byte-for-byte
C-vs-Rust comparisons (counter reported by the harness at process exit, not an
estimate). Combined with Phase C: **116 870** total differential comparisons.

All 30 rows pass under all six build configurations:

| profile | features | phase B | phase C | phase D |
|---------|----------|---------|---------|---------|
| debug   | `<default>` | 30/30 | 7/7 | 2/2 |
| debug   | `--no-default-features` | 30/30 | 7/7 | 2/2 |
| debug   | `--all-features` | 30/30 | 7/7 | 2/2 |
| release | `<default>` | 30/30 | 7/7 | 2/2 |
| release | `--no-default-features` | 30/30 | 7/7 | 2/2 |
| release | `--all-features` | 30/30 | 7/7 | 2/2 |

The **debug** rows matter independently: `debug-assertions`/`overflow-checks`
are ON there, so they prove none of the translated arithmetic
(`l += 1`, `k += 4`, `b1 << 2`, `strlen + 1`) overflows or panics where the C
silently wraps.

Reproduce everything with `./run_all_checks.sh`.

## Harness sensitivity (mutation check)

To prove the suite is not vacuously green, **20** single-token mutations were
injected into `src/lib.rs`, **one at a time** (the file was restored from a
pristine copy and md5-verified before each mutation), then the full suite was
re-run. Result: **18 killed, 2 provably-equivalent survivors, 0 genuine gaps.**

| mutation | tests failed |
|----------|--------------|
| `b1 << 2` -> `b1 << 1` | 29 |
| `k += 4` -> `k += 3` | 26 |
| `b2 >> 4` -> `b2 >> 3` | 26 |
| `decode()` fallthrough `63` -> `0` | 26 |
| `is_base64` rejects `'/'` | 26 |
| `b3 >> 2` -> `b3 >> 1` | 26 |
| `decode('+')` `62` -> `61` | 25 |
| lowercase offset `26` -> `27` | 25 |
| digit offset `52` -> `53` | 25 |
| `is_base64` rejects `'='` | 19 |
| guard `k+1 < l` -> `k+1 <= l` | 18 |
| `c2` default `'A'` -> `'B'` | 18 |
| guard `k+2 < l` -> `k+2 <= l` | 17 |
| guard `k+3 < l` -> `k+3 <= l` | 17 |
| `c3` padding sentinel `'='` -> `'#'` | 13 |
| `c4` padding sentinel `'='` -> `'#'` | 13 |
| `calloc` size `l + 13` -> `l + 12` | 13 |
| empty-string guard (`*src != 0`) removed | 4 |
| `b2 & 0xf` -> `b2 & 0x1f` | 0 — **equivalent mutant** |
| `b3 & 0x3` -> `b3 & 0x7` | 0 — **equivalent mutant** |

### Why the two survivors are not coverage gaps

`decode()` returns only `0..=63`, and both results are truncated by `as u8`:

* `(b2 & 0x1f) << 4` differs from `(b2 & 0xf) << 4` only in bit 8, discarded by `as u8`.
* `(b3 & 0x7) << 6` differs from `(b3 & 0x3) << 6` only in bit 8, discarded by `as u8`.

Verified by exhaustive enumeration over the entire `0..=63` domain of `decode()`
(4 096 + 4 096 pairs): **0 counterexamples**. The extra mask bits are dead in
both the C and the Rust, so no input can distinguish these mutants. They are
semantically equivalent, not untested.

`src/lib.rs` was restored after every mutation and md5-verified identical to the
original (`194e4b168ba63db17a7d1f524ae5591c`).
