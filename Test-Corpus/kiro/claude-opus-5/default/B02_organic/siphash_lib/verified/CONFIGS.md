# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| entry point | `stbds_hash_bytes` (low-level, 3 args), `siphash` (one-shot driver, prints 64 rows) | `include/lib.h`, `src/lib.c:110,114` |
| `len` vs block size | `len < 8` (main loop never runs) / `len >= 8` (loop runs `len/8` times) | `src/lib.c:18` |
| `len % 8` | 0,1,2,3,4,5,6,7 — eight distinct fall-through paths of the tail `switch` | `src/lib.c:48-65` |
| block count | 0, 1, 2, many blocks (loop trip count changes `v0..v3` mixing depth) | `src/lib.c:18-46` |
| `d[3]` high bit, main loop | `< 0x80` / `>= 0x80` → signed-`int` overflow sign-extends into the upper 32 bits of `data` | `src/lib.c:20` |
| `d[7]` high bit, main loop | `< 0x80` / `>= 0x80` → sign extension present but shifted out by `<< 16 << 16` | `src/lib.c:21-22` |
| `d[3]` high bit, tail `case 4` | `< 0x80` / `>= 0x80` → same signed-overflow sign extension in the tail | `src/lib.c:56` |
| `len` top byte | only `len & 0xff` survives `len << 56` | `src/lib.c:47` |
| `seed` | XOR'd in twice → **cancels**; value must be irrelevant, verified rather than assumed | `src/lib.c:10-17` |
| pointer provenance | `p` non-null; `len == 0` allows `p == NULL` (never dereferenced) | `src/lib.c:7` |
| `init` (for `siphash`) | any `int`; wraps `unsigned char` on fill and overflows signed on `z++` at `INT_MAX` | `src/lib.c:117-118` |
| `#ifdef` / compile-time modes | **none** — no preprocessor configuration in the library | grep: 0 `#if` |
| Cargo features | **none declared** in `translation/Cargo.toml` → the only combination is the default (empty) feature set | `Cargo.toml` |

There is no `add_executable` in `c_src/CMakeLists.txt`, so there is no driver
binary to stdout-diff; `siphash`'s stdout is diffed as a library call by
redirecting fd 1 around the FFI call (row C21).

## Configuration table

Every row is exercised through **both** `.so` files via `libloading` with many
randomized inputs (fixed seed `0x5eed_1234_5678_9abc`, `SplitMix64`), and the
returned `size_t` / printed bytes compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `stbds_hash_bytes` | `len == 0`, `p` non-null, `seed == 0` — empty input, no loop, tail `case 0` | [x] |
| C2 | `stbds_hash_bytes` | `len == 0`, `p == NULL` — empty + null pointer | [x] |
| C3 | `stbds_hash_bytes` | `len == 1` — no block, tail `case 1`; randomized byte over full 0..=255 | [x] |
| C4 | `stbds_hash_bytes` | `len == 2` — tail `case 2`, randomized bytes | [x] |
| C5 | `stbds_hash_bytes` | `len == 3` — tail `case 3`, randomized bytes | [x] |
| C6 | `stbds_hash_bytes` | `len == 4`, `d[3] < 0x80` — tail `case 4`, no sign extension | [x] |
| C7 | `stbds_hash_bytes` | `len == 4`, `d[3] >= 0x80` — tail `case 4`, **signed-overflow sign extension** | [x] |
| C8 | `stbds_hash_bytes` | `len == 5` — tail `case 5` (`d[4] << 32`), randomized, both `d[3]` polarities | [x] |
| C9 | `stbds_hash_bytes` | `len == 6` — tail `case 6` (`d[5] << 40`), randomized, both `d[3]` polarities | [x] |
| C10 | `stbds_hash_bytes` | `len == 7` — tail `case 7` (`d[6] << 48`), randomized, both `d[3]` polarities | [x] |
| C11 | `stbds_hash_bytes` | `len == 8` — exactly 1 block, tail `case 0`; randomized, all high-bit polarities of `d[3]`/`d[7]` | [x] |
| C12 | `stbds_hash_bytes` | `len == 8`, forced `d[3] >= 0x80` — main-loop low-word sign extension | [x] |
| C13 | `stbds_hash_bytes` | `len == 8`, forced `d[7] >= 0x80` — main-loop high-word sign bits shifted out | [x] |
| C14 | `stbds_hash_bytes` | `len == 8`, forced `d[3] >= 0x80` **and** `d[7] >= 0x80` — both overflow paths at once | [x] |
| C15 | `stbds_hash_bytes` | `len == 16` — 2 blocks, tail `case 0` | [x] |
| C16 | `stbds_hash_bytes` | `len` in 9..=15 — 1 block + every non-zero tail case, randomized | [x] |
| C17 | `stbds_hash_bytes` | `len` in 17..=255 — many blocks × every `len % 8`, randomized data | [x] |
| C18 | `stbds_hash_bytes` | `len` in 256..=1024 — `len << 56` top-byte aliasing region (`len & 0xff` wraps) | [x] |
| C19 | `stbds_hash_bytes` | all-zero buffer and all-`0xff` buffer at each `len % 8` — extreme value shapes | [x] |
| C20 | `stbds_hash_bytes` | `seed` swept over `0`, `1`, `SIZE_MAX`, and randomized values at fixed data — seed-cancellation must match | [x] |
| C21 | `stbds_hash_bytes` | unaligned `p` (offsets 1..=7 into a buffer) — C reads bytewise, must be identical | [x] |
| C22 | `siphash` | `init == 0` — stdout of all 64 rows captured via fd-1 redirect, byte-compared | [x] |
| C23 | `siphash` | `init` = 1, 42, 200, 255, 256, `-1`, `INT_MIN`, `INT_MAX`, plus randomized `i32` values | [x] |
| C24 | composed pipeline | `siphash(init)` cross-checked against 64 direct `stbds_hash_bytes` calls on the same `mem` fill, in both libraries, so the composition (not just each wrapper) is verified | [x] |
| C25 | `stbds_hash_bytes` | broad randomized fuzz over the whole space: 30 000 short (`len` 0..=32), 8 000 long (`len` 0..=2048, many blocks), 20 000 sparse-high (mostly zero with 1..4 bytes forced to `0x80`/`0xff`), each with a randomized `seed` | [x] |

## How to run

```
cd translation && ./scripts/verify_all.sh
```

The script builds the C `.so`, then for every feature combination and for both
the `release` and `dev` profiles: builds the Rust `.so` + driver, diffs
`nm -D` output, and runs all test binaries. Building before testing is
mandatory — see the methodology note in `SYMBOLS.md`.

Test files:

* `tests/common/mod.rs` — `libloading` harness, SplitMix64 PRNG (fixed seed
  `0x5eed_1234_5678_9abc`), stale-artifact guard, driver-subprocess stdout capture.
* `tests/phase_b_valid.rs` — rows C1..C25.
* `tests/phase_c_errors.rs` — rows E1..E20 plus generic boundaries.
* `tests/phase_d_symbols.rs` — `nm -D` parity, enforced as tests.
* `examples/siphash_driver.rs` — dlopens one `.so` and calls `siphash(init)` in
  its own process, so captured stdout cannot be polluted by test-harness output.
