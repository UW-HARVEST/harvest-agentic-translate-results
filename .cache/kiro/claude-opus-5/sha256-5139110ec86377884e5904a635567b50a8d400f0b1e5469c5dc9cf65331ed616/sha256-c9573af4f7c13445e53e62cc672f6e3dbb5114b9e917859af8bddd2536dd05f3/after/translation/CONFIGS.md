# CONFIGS.md — Configuration-surface table (Phase B gate)

## Axes derived from the C source

The library has **no** runtime options, modes, flags, `#ifdef`s or global state.
`c_src/include/lib.h` exports exactly one entry point:

```c
char *decode_base64(const char *src);
```

There is no context struct, no init/teardown, no byte-order or width option.
`grep -n '#if\|#ifdef\|switch\|extern\|static [a-z_]* [a-z_]* =' c_src/src/lib.c`
finds no configuration state — only `#define TRUE 1` / `#define FALSE 0`.

Therefore the whole configuration surface is the **input-shape** axis, and the
axes are exactly the branch conditions in the source:

* **A1 — `decode()` character class** (`lib.c:12-25`): `A-Z` / `a-z` / `0-9` /
  `'+'` / fall-through-63 (reachable as `'/'` and `'='`). 5 values.
* **A2 — `is_base64()` accept/reject** (`lib.c:31-37`): accepted
  (`[A-Za-z0-9+/=]`) vs. dropped (everything else, incl. bytes ≥ 0x80 which are
  negative `char`). 2 values.
* **A3 — filtered length modulo 4** (`lib.c:79-89` guards `k+1<l`, `k+2<l`,
  `k+3<l`): `l%4 ∈ {0,1,2,3}`, plus the degenerate `l == 0`. 5 values.
* **A4 — `'='` position within a 4-char group** (`lib.c:98`, `lib.c:102`): none /
  at `c3` / at `c4` / at both / at `c1` or `c2` (which the C does **not**
  special-case — it decodes `'='` as 63 there). 5 values.
* **A5 — group count**: 0 / 1 / 2 / many. 4 values.
* **A6 — output containing NUL bytes** (the returned buffer is NUL-terminated
  *and* may contain interior `0x00`, so comparison must be over the whole
  `strlen(src)+1+13` allocation, not `strlen(dest)`). 2 values.

Rows below are the pruned cross-product: one row per combination the C actually
treats differently. Each row is driven with **many randomized inputs**
(deterministic `SplitMix64`, fixed seed `0x5EED_1234_ABCD_F00D`), and both `.so`s
are compared over the entire allocated buffer (`strlen(src) + 1 + 13` bytes) plus
the NULL-ness of the returned pointer and the `strlen` of the result.

Allocation sizes and counts are compared separately and exactly, by interposing
`calloc`/`malloc`/`free` (`tests/alloc_contract.rs`) — see `ERRORS.md`.
`malloc_usable_size` is deliberately **not** used as a size oracle: glibc reuses
binned chunks and hands a chunk over whole when the remainder is too small to
split, so it reflects heap state rather than the requested size, and it produced
two false divergences before being replaced.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `decode_base64` | A1=`A-Z` only, A2=all accepted, A3 random, A5=many, randomized lengths 1..256 | `cfg_01_upper_only` | [x] |
| 2 | `decode_base64` | A1=`a-z` only, A2=all accepted, randomized lengths 1..256 | `cfg_02_lower_only` | [x] |
| 3 | `decode_base64` | A1=`0-9` only, randomized lengths 1..256 | `cfg_03_digits_only` | [x] |
| 4 | `decode_base64` | A1=`'+'` only (decode→62), randomized lengths 1..256 | `cfg_04_plus_only` | [x] |
| 5 | `decode_base64` | A1=fall-through, `'/'` only (decode→63), randomized lengths 1..256 | `cfg_05_slash_only` | [x] |
| 6 | `decode_base64` | A1=fall-through, `'='` only (decode→63 **and** triggers both suppression branches), randomized lengths 1..256 | `cfg_06_equals_only` | [x] |
| 7 | `decode_base64` | A1=full 64-char alphabet mixed, A3=`l%4==0`, A5=many | `cfg_07_alphabet_mod4_0` | [x] |
| 8 | `decode_base64` | A1=full alphabet, A3=`l%4==1` (c2,c3,c4 default `'A'`) | `cfg_08_alphabet_mod4_1` | [x] |
| 9 | `decode_base64` | A1=full alphabet, A3=`l%4==2` (c3,c4 default `'A'`) | `cfg_09_alphabet_mod4_2` | [x] |
| 10 | `decode_base64` | A1=full alphabet, A3=`l%4==3` (c4 defaults `'A'`) | `cfg_10_alphabet_mod4_3` | [x] |
| 11 | `decode_base64` | A5=1 group exactly (4 chars), random alphabet | `cfg_11_single_group` | [x] |
| 12 | `decode_base64` | A5=2 groups exactly (8 chars), random alphabet | `cfg_12_two_groups` | [x] |
| 13 | `decode_base64` | canonical RFC-style padded base64, one `'='` at tail (A4=`c4`) | `cfg_13_canonical_pad1` | [x] |
| 14 | `decode_base64` | canonical RFC-style padded base64, two `'=='` at tail (A4=`c3`+`c4`) | `cfg_14_canonical_pad2` | [x] |
| 15 | `decode_base64` | canonical padded base64, no padding needed (A4=none) | `cfg_15_canonical_pad0` | [x] |
| 16 | `decode_base64` | `'='` injected at **random interior** positions (A4=`c1`/`c2`/`c3`/`c4` uniformly, mid-string, multiple times) | `cfg_16_equals_interior_random` | [x] |
| 17 | `decode_base64` | base64 chars interleaved with random **non**-base64 ASCII (A2 mixed) — filter loop drops them, so filtered length ≠ input length | `cfg_17_mixed_with_noise` | [x] |
| 18 | `decode_base64` | base64 chars interleaved with **high-bit** bytes 0x80..0xFF (negative `char`, all rejected by `is_base64`) | `cfg_18_mixed_with_high_bit` | [x] |
| 19 | `decode_base64` | A3=`l == 0`: input is entirely non-base64 (noise only) → decode loop never runs, returns zero-filled buffer | `cfg_19_all_noise_empty_result` | [x] |
| 20 | `decode_base64` | A6: input engineered so decoded output contains interior `0x00` bytes (`"AAAA..."`, `"QUJD AAAA"` style) | `cfg_20_output_with_interior_nuls` | [x] |
| 21 | `decode_base64` | boundary characters one step outside each `decode`/`is_base64` range: `'@' '[' '`' '{' '/' ':' '*' ',' '.' '-' '_' ' '` mixed with valid chars | `cfg_21_range_boundary_chars` | [x] |
| 22 | `decode_base64` | fully arbitrary bytes `0x01..0xFF` (uniform fuzz), randomized lengths 1..512 — crosses A1×A2×A3×A4×A6 simultaneously | `cfg_22_arbitrary_bytes_fuzz` | [x] |
| 23 | `decode_base64` | single-character inputs, **exhaustive** over all 255 non-NUL byte values | `cfg_23_exhaustive_single_char` | [x] |
| 24 | `decode_base64` | two-character inputs, **exhaustive** over the 64-char base64 alphabet + `'='` (65×65 = 4225 pairs) — covers every `c1`×`c2` pair with `c3`/`c4` defaulted | `cfg_24_exhaustive_char_pairs` | [x] |
| 25 | `decode_base64` | long inputs: randomized lengths 1000..4096 over the full alphabet, ensures no divergence in the `int`-typed `k`/`l` arithmetic at scale | `cfg_25_long_inputs` | [x] |
| 26 | `decode_base64` | ASCII-only fuzz (0x20..0x7E) with `'='` over-represented, randomized lengths 1..300 — stresses A4 suppression interacting with A2 filtering | `cfg_26_ascii_fuzz_equals_heavy` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent). This is
verified mechanically by `check_features.sh`, which parses `Cargo.toml` and loops
over the feature power set.

## Test-sensitivity evidence

Passing tests only mean something if they can fail. `mutation_sweep.sh` perturbs
one behaviour of the Rust translation at a time, forces a rebuild, and reruns the
suite. Current result: **23 of 23 non-equivalent mutants caught, 0 missed**, and
the one deliberately semantically-equivalent mutant correctly not caught
(`(b3 & 0x7) << 6` equals `(b3 & 0x3) << 6` in 8 bits, in Rust and in C alike).

The sweep forces a rebuild after every edit because cargo's mtime fingerprinting
can treat a same-second source edit as up to date and silently test a **stale**
`.so`. That happened during this verification and briefly produced two bogus
"divergences"; `run_verification.sh` and `mutation_sweep.sh` both delete or
`touch` artifacts to prevent it.
