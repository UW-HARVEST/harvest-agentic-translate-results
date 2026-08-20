# VERIFICATION.md — how the translation was verified, and how sensitive the suite is

The C in `c_src/` is the ground truth. Every test below loads **both** shared
objects with `libloading` and calls them **only** through the exported
`decode_base64` symbol — the Rust functions are never called directly, so the
`#[unsafe(no_mangle)] extern "C"` wrapper is part of what is under test.

```
c_src/build/libdriver.so   <- gcc, built by c_src/CMakeLists.txt
target/<profile>/libdriver.so <- the Rust cdylib
```

Run everything with:

```sh
./verify.sh              # C build + every feature combination + full suite
./verify.sh --release    # same, against the optimized Rust cdylib
C_DRIVER_SO=/path/to/other/libdriver.so cargo test   # compare against a differently-built C
```

The whole suite takes ~100 s; ~96 s of that is the two multi-GiB tests
(`alloc_wraparound.rs`, `ub_l_zero.rs`) which must allocate 2-4 GiB buffers to
reach the `int`-truncation branches. They serialize themselves with a mutex, so
any `--test-threads` setting works.

## What is compared

`decode_base64` returns a `calloc(1, strlen(src) + 14)` buffer, so the harness
compares **the entire allocation**, not the C string:

* both returned pointers must be NULL or both non-NULL;
* all `strlen(src) + 14` bytes must be identical — including the zero fill past
  the NUL terminator, which is what catches short writes, extra writes and
  off-by-one buffer arithmetic that a `strcmp` would silently hide;
* both blocks must have at least the requested usable size
  (`malloc_usable_size`), and exactly the same size in the mmap-served regime.

Inputs are property-style randomized from a fixed seed (`0x2545F4914F6CDD1D`),
plus three exhaustive sweeps (all 255 single bytes; all 65² alphabet pairs; all
65³ = 274 625 alphabet triples).

## Test inventory (40 tests in 6 test binaries)

| file | what it covers |
|------|----------------|
| `tests/common/mod.rs` | harness: `dlopen` of both `.so`s, full-allocation comparison, PRNG, reference base64 encoder |
| `tests/symbols.rs` | Phase D: `nm -D` symbol parity, no missing symbols, internal helpers stay internal, libc allocator is used |
| `tests/valid_paths.rs` | Phase B: 26 tests, one per `CONFIGS.md` row |
| `tests/error_paths.rs` | Phase C: `ERRORS.md` rows 1-2 + the generic FFI boundary rows (`g1`-`g4`) |
| `tests/alloc_wraparound.rs` | Phase C: `ERRORS.md` rows 3, 3b, 4 reached deterministically through the C's own `int` truncation of `strlen(src)+1` |
| `tests/alloc_failure.rs` | Phase C: `ERRORS.md` rows 3 and 4 reproduced independently at 64 MiB scale under `RLIMIT_AS`, with probe allocations proving *which* allocation the starvation hits |
| `tests/ub_l_zero.rs` | Phase C: `ERRORS.md` row 5 — the one reachable input where the C itself is undefined (`l == 0`); both must die identically |

## Configurations verified

| axis | values | result |
|------|--------|--------|
| cargo features | the empty set (the only combination — `Cargo.toml` has no `[features]`), enumerated mechanically by `verify.sh` | pass |
| Rust profile | `dev` (debug assertions + overflow checks) and `release` (`opt-level=3`) | pass |
| C optimization | `-O0` (CMake default), `-O1`, `-O2`, `-O3` via `C_DRIVER_SO=…` | pass |

## Mutation study — proof that the suite is not vacuous

Each mutation was applied to `src/lib.rs`, the cdylib rebuilt, and the suite
re-run; detection is by test-process exit code (a mutation that *crashes* the
harness counts as detected). **All 23 semantic mutations are caught, and all 6
semantics-preserving controls correctly pass**:

| mutation | detected |
|----------|----------|
| `decode`: `'+'` → 61 instead of 62 | yes |
| `decode`: lowercase offset 26 → 27 | yes |
| `decode`: digit offset 52 → 51 | yes |
| `decode`: fallthrough 63 → 62 | yes |
| `is_base64`: `'/'` no longer accepted | yes |
| `is_base64`: filter inverted | yes |
| group: `k+2 < l` → `k+2 <= l` | yes |
| group: `k+3 < l` → `k+3 < l-1` | yes |
| group: default char `'A'` → `'B'` | yes |
| padding: `c3` compared against `'+'` instead of `'='` | yes |
| padding: `c4` check removed | yes |
| output byte 2: `b3 >> 2` → `b3 >> 1` | yes |
| output byte 1: `b2 >> 4` → `b2 >> 3` | yes |
| loop step `k += 4` → `k += 3` | yes |
| `src == NULL` check dropped | yes |
| `*src == '\0'` check dropped | yes |
| destination size `l + 13` → `l + 12` | yes |
| `l = strlen + 1` → `l = strlen` | yes |
| `calloc` NULL check dropped | yes |
| `malloc` NULL check dropped | yes |
| scratch size: negative `l` clamped to 0 | yes |
| scratch size `malloc(l)` → `malloc(l - 2)` (1-byte heap overflow) | yes |
| scratch size `malloc(l)` → `malloc(l / 2)` | yes |
| *control* `malloc(l)` → `malloc(l - 1)` | not detected (correct: the filter loop writes at most `strlen == l-1` bytes, so `l-1` is exactly big enough — only `l-2` and below overflow, and both of those *are* detected) |
| *control* `calloc(1, n)` → `calloc(n, 1)` | not detected (correct: identical semantics) |
| *control* `63` written as `0x3f` | not detected (correct) |
| *control* `b3 & 0x3` → `b3 & 0x7` | not detected (correct: bit 8 is truncated by the `as u8` store) |
| *control* `(strlen+1) as i64 as i32` instead of `(strlen as i32).wrapping_add(1)` | not detected (correct: identical modulo 2³²) |
| *control* destination size computed as `(l as i64) + 13` | not detected — differs from the C only when `l + 13` overflows `int` *positively*, which needs a ≥2 GiB successful allocation; impossible under this host's 6 GiB `RLIMIT_DATA`, so no input can distinguish the two here |
| *control* scratch size `(l as u32)` instead of sign-extended | not detected — same reason (would need a successful 4 GiB `malloc` while a 4 GiB input is held) |

Two traps found and fixed while building the harness (both would have produced
false confidence):

1. `malloc_usable_size` equality is **not** a function of the request size —
   glibc may serve a request from a larger free chunk — so the strict equality
   check was replaced by a sound lower bound (plus exact equality only in the
   mmap-served regime).
2. In an optimized test binary LLVM **deletes** the dead `malloc` calls of an
   address-space-exhaustion loop and folds the null check to "not null", so the
   loop "allocated" 3.9 TiB while the address space never grew and the
   starvation was fake. Every allocation in the ballast/probe code is now wrapped
   in `std::hint::black_box`, and the child *proves* the intended allocation
   fails before the library is called.

## Completion gate

* [x] `SYMBOLS.md` — `nm -D` diff C→Rust is empty (1 exported symbol,
      `decode_base64`); no unresolved non-libc symbols in the Rust `.so`.
* [x] Phase B — every one of the 25 `CONFIGS.md` rows passes across randomized
      inputs (26 tests including the known-answer pins).
* [x] Phase C — every one of the 6 `ERRORS.md` rows (1, 2, 3, 3b, 4, 5) plus all
      generic FFI-boundary rows has a passing differential test that asserts the
      *same* sentinel, not just "both failed".
* [x] All of the above hold under every feature combination (the empty set — the
      only one), in both the `dev` and `release` profiles, and against the C
      compiled at `-O0`…`-O3`.
* [x] No behaviour of the C was "fixed": the `int` truncation of
      `strlen(src) + 1`, the negative-size `calloc`/`malloc` requests, the
      `'='`-only-at-`c3`/`c4` padding rule, `decode('/') == decode('=') == 63`
      and the missing explicit NUL terminator are all reproduced exactly.
