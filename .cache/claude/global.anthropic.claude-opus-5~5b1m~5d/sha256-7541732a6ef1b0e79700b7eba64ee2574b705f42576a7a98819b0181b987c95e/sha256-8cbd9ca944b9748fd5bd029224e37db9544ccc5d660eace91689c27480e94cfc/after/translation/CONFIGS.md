# CONFIGS.md — Phase B: CONFIGURATION-SURFACE TABLE

Derived mechanically from the C source. The axes below are exactly the things
the C code branches on or formats differently.

## Axis enumeration (mechanical)

### Public entry points (the FULL set, incl. the lowest level)

`nm -D --defined-only` on `libdriver.so` yields exactly one entry point:

| entry point | signature | level |
|-------------|-----------|-------|
| `driver` | `void driver(int x)` | the only one — simultaneously the highest and the lowest level public function |

`print_hex(unsigned char *p, int len)` is the lower-level worker, but it is
`static`, so it is *not* a public entry point and is unreachable from outside
either library. There is no convenience-wrapper/low-level split to miss here:
`driver` **is** the low-level entry point.

### Runtime options / modes / flags

- Public header (`driver.h`) declares no setters, no context struct, no enums,
  no flags: **0 runtime options.**
- `grep` for `if` / `switch` / `#ifdef` in `src/driver.c`: the only conditional
  in the entire library is the `for` loop guard `i < len`, with `len` fixed at
  the compile-time constant `sizeof(int)`. **0 configurable branches.**
- `Cargo.toml` has **no `[features]` section** => the only feature combination
  is the default (empty) one. `--no-default-features` and the default build are
  therefore the same build; both are still exercised (see bottom of file).

### Input shapes the code actually special-cases

The single input is an `int`. What the code *does* with it is
`print_hex((unsigned char*)&x, 4)` -> four `printf("%02x", byte)` calls plus a
`printf("\n")`. The shapes that therefore change the output path are:

| axis | distinct cases the code distinguishes | why it matters |
|------|----------------------------------------|----------------|
| A. per-byte magnitude | byte `< 0x10` vs byte `>= 0x10` | `%02x` zero-pads only the former; a translation using `{:x}` instead of `{:02x}` diverges |
| B. per-byte sign bit | byte `< 0x80` vs byte `>= 0x80` | C reads through `unsigned char*` and the default argument promotion widens to a *non-negative* `int`. A translation using `i8`/`c_char` would sign-extend and print `ffffff80`; this is the single most likely translation bug |
| C. byte position | which of the 4 positions holds the interesting byte | reveals byte order: on this little-endian target byte 0 is the LSB. A translation that iterated in the wrong direction, or used `to_be_bytes()`, diverges |
| D. sign of `x` | negative / zero / positive | negative values set byte 3's high bit (axis B at position 3) |
| E. value class | `0`, `-1`, `INT_MIN`, `INT_MAX`, powers of two, random | boundary + value-dependent coverage |
| F. call count | one call vs many calls in sequence | the library has no `static` state; output must be a pure function of `x`, and the trailing `"\n"` must delimit records identically |

Rows below are the pruned cross-product of A x B x C x D x E x F.

## Table

Every row is run against **both** `.so`s via `libloading`, with `stdout`
captured at the file-descriptor level (so the libc `stdout` that *both*
libraries write through is compared byte-for-byte), and every row uses many
randomized inputs from a fixed seed (`SEED = 0x5EED_1234_ABCD_EF01`,
splitmix64) rather than one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `driver` | no options (none exist); `x = 0` — all four bytes `0x00`, i.e. axis A "all bytes need zero-padding", the "empty" shape | `cfg_row01_zero` | [x] |
| 2 | `driver` | `x = -1` — all four bytes `0xff`, i.e. axis B "every byte has its high bit set" (max sign-extension exposure) | `cfg_row02_all_ones` | [x] |
| 3 | `driver` | `x = INT_MAX` (`0x7fffffff`) — top byte `0x7f` (below sign bit), lower bytes `0xff` | `cfg_row03_int_max` | [x] |
| 4 | `driver` | `x = INT_MIN` (`0x80000000`) — top byte exactly `0x80`, the sign-bit boundary of axis B; lower bytes `0x00` (axis A) | `cfg_row04_int_min` | [x] |
| 5 | `driver` | axis A x C: exactly one byte set to a value `< 0x10` at each of the 4 positions, the rest `0x00` — 4 sub-cases x randomized nibble | `cfg_row05_single_low_nibble_byte_each_position` | [x] |
| 6 | `driver` | axis A x C: exactly one byte set to a value in `0x10..=0x7f` at each of the 4 positions — no padding, no sign bit | `cfg_row06_single_midrange_byte_each_position` | [x] |
| 7 | `driver` | axis B x C: exactly one byte set to a value in `0x80..=0xff` at each of the 4 positions — isolates sign-extension per byte lane | `cfg_row07_single_high_bit_byte_each_position` | [x] |
| 8 | `driver` | axis A/B exhaustive per lane: for each of the 4 byte positions, ALL 256 byte values `0x00..=0xff` with the other bytes zero (1024 calls) — full per-lane coverage of padding + sign extension + byte order | `cfg_row08_exhaustive_byte_per_lane` | [x] |
| 9 | `driver` | axis C: single-bit values `1 << k` for every `k` in `0..32` — pins the bit-to-nibble-to-byte mapping and the endianness | `cfg_row09_every_single_bit` | [x] |
| 10 | `driver` | axis D: uniformly random **negative** `x` (`i32 < 0`), 2000 randomized values | `cfg_row10_random_negative` | [x] |
| 11 | `driver` | axis D: uniformly random **positive** `x` (`i32 > 0`), 2000 randomized values | `cfg_row11_random_positive` | [x] |
| 12 | `driver` | axis A x B x C x D: uniformly random full-range `x` over all 2^32 bit patterns, 5000 randomized values | `cfg_row12_random_full_range` | [x] |
| 13 | `driver` | axis A x B: random `x` whose every byte is forced `< 0x10` (all four lanes zero-padded simultaneously) | `cfg_row13_all_bytes_low_nibble` | [x] |
| 14 | `driver` | axis B: random `x` whose every byte is forced `>= 0x80` (all four lanes sign-bit set simultaneously) | `cfg_row14_all_bytes_high_bit` | [x] |
| 15 | `driver` | axis E: byte-permutation shape — the four distinct bytes `0x00,0x0f,0x80,0xff` placed in all 24 orderings | `cfg_row15_all_byte_permutations` | [x] |
| 16 | `driver` | axis F: "one" — a single call in a freshly captured stdout, asserting the output is exactly 9 bytes (8 hex + `\n`) and nothing more | `cfg_row16_single_call_exact_framing` | [x] |
| 17 | `driver` | axis F: "many" — 500 randomized calls in one captured stdout run, comparing the whole concatenated stream (catches missing/extra `\n` and any hidden state) | `cfg_row17_many_calls_one_stream` | [x] |
| 18 | `driver` | axis F: interleaved C-call/Rust-call in the *same* captured stdout, alternating libraries per call, to prove both share the identical libc `stdout` buffering and produce an interleaved stream identical to the all-C stream | `cfg_row18_interleaved_c_and_rust_same_stdout` | [x] |
| 19 | `driver` | end-to-end "real consumer" run: load lib, call `driver` over a randomized workload, unload, reload, call again — full lifecycle including `dlclose`/`dlopen` | `cfg_row19_full_lifecycle_reload` | [x] |

## Binary executable

`c_src/CMakeLists.txt` contains a single `add_library(driver SHARED ...)` and no
`add_executable`; `translation/Cargo.toml` declares only `crate-type =
["cdylib"]` and has no `src/main.rs` / `[[bin]]`. **The project builds no binary
driver.**

The gate is covered anyway, twice:

1. In-process, rows 16–19 compare the raw `stdout` byte stream emitted through
   the real libc `stdout`.
2. Out-of-process, `scripts/compare_process_stdout.sh` supplies the missing
   driver: `scripts/loader.c` is a tiny external C consumer that `dlopen`s ONE
   `libdriver.so`, calls `driver()` over a 5071-value deterministic workload,
   and **exits without any explicit `fflush`** — so at-exit flush behaviour is
   part of the comparison, which the in-process test (which flushes explicitly)
   cannot see. The same loader is run against the C `.so` and the Rust `.so` and
   the two stdout streams are `cmp`'d. It is checked with stdout redirected to a
   **file**, through a **pipe** (different libc buffering mode), and to
   `/dev/null` (exit-code parity), across many separate process lifetimes.

   Result: `45639 bytes, 5071 records` identical; pipe identical; exit codes
   identical.

## Feature combinations

`Cargo.toml` declares no `[features]`, so the complete set of feature
combinations is `{ default }` = `{ (empty) }`.

`scripts/verify_all.sh` enumerates the feature list out of `Cargo.toml`
mechanically (it computes the full powerset if any features are ever added) and
runs, for each combination: `cargo build --release`, `cargo check --tests`, the
`nm -D` symbol diff, the differential suite, and the process-level stdout
comparison. Current output:

```
declared features: <none>
combinations to verify: 2      # default, --no-default-features
...
=== ALL COMBINATIONS PASSED (2 configurations) ===
```

Both configurations: symbol sets identical, `33 passed; 0 failed`, process
stdout identical. The suite also passes under the debug profile
(`cargo test --offline`): `33 passed; 0 failed`.

## Harness notes

- `tests/differential.rs` uses `harness = false`. `driver`'s only output channel
  is the process-global fd 1, so a comparison must redirect it; under libtest's
  default thread pool, libtest's own progress lines and other tests' output land
  inside a capture and corrupt the diff (this was observed and is why the custom
  sequential runner exists).
- `libloading` is resolved with `--offline` from the local cargo cache
  (`libloading 0.8.9`), since crates.io is not reachable from this sandbox.

## Mutation check (proof the suite is not vacuous)

Three deliberate bugs were injected into `translation/src/lib.rs`, the suite was
run, and the file was restored byte-for-byte:

| injected bug | detected by |
|--------------|-------------|
| read bytes as `*const i8` instead of `*const u8` (sign extension) | 20 rows FAILED (rows 02, 03, 04, 07–12, 14–19 + err3/4/5/7/9) |
| iterate the 4 bytes in reverse (wrong byte order) | 23 rows FAILED |
| use Rust `print!`/`println!` instead of libc `printf` | **row 18 only** — the interleaved C/Rust stdout row; every per-call row still passed, which is exactly the blind spot row 18 exists to close |
