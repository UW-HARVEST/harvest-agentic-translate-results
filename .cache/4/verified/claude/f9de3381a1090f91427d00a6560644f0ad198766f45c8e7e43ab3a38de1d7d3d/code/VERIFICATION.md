# VERIFICATION.md — C ↔ Rust differential verification report

Library under test: `c_src/{include/lib.h,src/lib.c}` → `src/lib.rs`.
Single public entry point: `char *encode_base64(int size, const char *src)`.

## Completion gate

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 unresolvable non-libc symbols in the Rust `.so` | PASS |
| Phase B: every one of the 21 `CONFIGS.md` rows passes across randomized inputs | PASS |
| Phase C: every one of the 16 `ERRORS.md` rows has a passing error-path differential test | PASS |
| All of the above under every feature combination | PASS (see below) |

`cargo test` → **40 passed / 0 failed**.

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section**, so the set of valid feature
  combinations is the single empty combination. It was checked and tested as
  `<default>`, `--no-default-features` and `--all-features` (all identical), and
  additionally under `--release` (which selects `panic = "abort"` and therefore
  a different codegen path for the `extern "C"` boundary).
* `c_src/CMakeLists.txt` declares no `option()` and no
  `target_compile_definitions`, and `lib.c` contains no `#ifdef`, so the C has a
  single configuration too.

| configuration | `cargo check` | `cargo test` |
|---------------|---------------|--------------|
| default | PASS | 40 passed |
| `--no-default-features` | PASS | 40 passed |
| `--all-features` | PASS | 40 passed |
| `--release` | PASS | 40 passed |

The suite was additionally run against the C library rebuilt at `-O2` and `-O3`
(via `DIFF_C_SO=...`, out of tree so `c_src/` is untouched): **40 passed** in
both cases. This matters because the C relies on signed-integer overflow in
`size * 4 / 3 + 4`, which is UB the optimiser is allowed to exploit; the Rust's
`wrapping_*` arithmetic agrees with what GCC actually emits at every level.

## How the tests call the code

`tests/differential.rs` loads **both** shared objects with `libloading` and
resolves `encode_base64` from each. No Rust function is ever called directly, so
the `#[unsafe(no_mangle)] extern "C"` export wrapper is itself under test.

Each comparison checks three things, not just the string:

1. **NULL-ness** — the library's only error signal.
2. **The entire `size*4/3+4` byte extent** of the returned allocation, so the
   `'='` padding *and* `calloc`'s zero-filled tail are compared byte-for-byte
   (not merely the bytes up to the first NUL).
3. **A lower bound on the allocation** via `malloc_usable_size`: each block must
   hold the bytes actually produced plus the terminating NUL.

`malloc_usable_size` is deliberately *not* compared for equality between the two
implementations: glibc may satisfy a request from a larger free chunk when
splitting is unprofitable, so it is not a function of the requested size and an
equality assertion on it is flaky in both directions (observed diverging by ±16
bytes on identical arithmetic). Undersized allocations are instead caught by the
lower bound plus a full run under `MALLOC_CHECK_=3 MALLOC_PERTURB_=165`, which
aborts on the heap corruption an over-running write causes and also makes a
`malloc`-instead-of-`calloc` substitution visible as a non-zero tail.

## Harness self-test (mutation testing)

Passing tests only mean something if they can fail. `./mutation_check.sh`
injects 33 deliberate defects into `src/lib.rs`, rebuilds, and re-runs the
suite (plain and under the hardened allocator), then restores the file.

```
behaviour-changing mutants caught: 31/31
ABI-equivalent mutants that correctly survived: 2/2
All mutants behaved as expected.
```

Caught mutants span the allocation arithmetic (`+4→+1`, `*4→*3`, `/3→/4`,
sign-extend→zero-extend, `calloc→malloc`), the input validation and control flow
(negative-size "sanitising", the `size == 0` strlen trigger, `strlen` off-by-one,
loop bound, loop step, both padding branches, the `b2` read guard), all six bit
manipulations, the signed-`char` conversion, the `b2`/`b3` zero defaults, every
`encode()` threshold, both special alphabet characters and the `'='` padding
byte.

### The two mutants that survive — and why that is correct

1. **`calloc(1, n)` → `calloc(n, 1)`.** Both request the same product and both
   fail identically for a huge `n`; glibc cannot distinguish them, so no FFI
   caller can either.
2. **C truncating division → floor division** (`wrapping_div(3)` →
   `div_euclid(3)`). Exhaustively analysed over the `i32` domain: the two differ
   **only** when the dividend `size*4` is negative, i.e. only when `size` is
   negative — and then the encode loop never executes, so the buffer is
   all-zero and the returned string is empty either way. The difference is at
   most **1 byte** of allocation and **never** flips the sign of `nbytes`, so it
   never flips NULL-ness (checked over the whole negative range: 0 sign
   differences, max value difference 1). Nothing observable through the C ABI
   changes.

   The shipped Rust nonetheless uses `wrapping_div`, which *is* C's `/`.

## Behaviour deliberately preserved (C quirks, not bugs to fix)

* `if (!size) size = strlen(src)` triggers on `size == 0` **only** — a negative
  `size` does *not* fall back to `strlen`, and the encode loop is simply skipped.
* The accept/reject boundary is the sign of `size * 4 / 3 + 4`, **not** the sign
  of `size`. So `size == -1, -2, -3` are *accepted* (they yield `nbytes` of
  `3, 2, 0`; note `calloc(1, 0)` returns a non-NULL zero-length block), while
  `size <= -4` is rejected because `nbytes` goes negative and sign-extends to a
  `size_t` that `calloc` cannot honour.
* `size == i32::MIN` is *accepted*: `size * 4` wraps to `0`, so `nbytes == 4`.
  Sizes just above `i32::MIN` wrap to small positives and are accepted too.
* `size` is truncated from `strlen`'s `size_t` to `int`, and `src[i]` is a
  *signed* `char` converted to `unsigned char` (mod 256).
* The result is NUL-terminated only because `calloc` zeroes the buffer; the code
  never writes a terminator. For every `size >= 0` the buffer is provably at
  least one byte longer than the output, so this holds.
* `encode()`'s final `return '/'` is a catch-all, so it is total over `0..=255`
  even though callers only ever pass `0..=63`.

## Inputs excluded from execution (undefined behaviour in the C itself)

| input | why it cannot be differentially executed |
|-------|------------------------------------------|
| `size >= 2^29` with a valid `src` | `size * 4` overflows, so `calloc` returns a tiny buffer while the loop still runs `size/3` iterations — the **C ground truth** writes far out of bounds and segfaults. Both sides compute `nbytes` with the same wrapping arithmetic (verified by inspection and by the `-O2`/`-O3` runs over the reachable domain). |
| `size > 0` with fewer than `size` readable bytes at `src` | out-of-bounds *read* in the C by construction. The harness asserts every test supplies at least `size` bytes. |
| `calloc` requests above 4 MiB (`MAX_TESTABLE_NBYTES`) | whether such a request succeeds depends on the machine's overcommit settings, so "both agree" is not deterministic. Only reachable via wrapped negative sizes; the wrap path itself is covered at smaller magnitudes by `wrap_positive_nbytes_from_huge_negative_sizes`. |

## Reproducing

```sh
# 1. build the C ground truth
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build . && cd ../..

# 2. build the Rust .so and run the differential suite
cargo build && cargo test

# 3. symbol parity
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only target/debug/libdriver.so | grep encode_base64

# 4. hardened-allocator run
MALLOC_CHECK_=3 MALLOC_PERTURB_=165 cargo test

# 5. prove the suite can fail
./mutation_check.sh
```

All randomized tests use a fixed seed per test (xorshift64\*), so failures are
reproducible.
