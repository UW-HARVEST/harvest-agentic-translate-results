# Verification report

Ground truth: `c_src/src/lib.c` (compiled to `c_src/build/libdriver.so`).
Under test: `translation/src/lib.rs` (compiled to `target/{debug,release}/libdriver.so`).

The library is a single public function, `char *decode_base64(const char *src)`,
plus two `static` helpers (`decode`, `is_base64`).

**Result: no behavioural divergence found. The Rust translation matches the C
byte-for-byte on every input tested, and no change to `translation/src/lib.rs`
was required.**

## How the comparison is done

All tests load **both** shared libraries with `libloading` and call
`decode_base64` only through the dynamic symbol — the Rust implementation is
never invoked as a Rust function, so the `#[unsafe(no_mangle)] extern "C"`
export wrapper is exercised as well. Returned buffers are released with the
platform `free`, exactly as a C consumer would.

Comparison is over the **entire returned allocation** (`strlen(src) + 14` bytes,
which both sides `calloc`, hence fully defined), not up to the first `NUL`. That
matters because base64 output routinely contains interior `NUL` bytes — `"AAAA"`
decodes to `00 00 00`, where a C-string comparison would see two empty strings
and pass regardless of a bug.

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` exports exactly one symbol,
      `decode_base64`; the Rust `.so` exports it under the same name. The symbol
      diff is **empty** (asserted by `tests/symbols.rs::d01`, which fails if any
      C symbol is absent). No C source went untranslated: `lib.c` is the only
      file in the CMake target and all three of its functions have Rust
      counterparts, with the two `static` ones correctly *not* exported. No
      stubs, no `unimplemented!()`. Rust's undefined symbols are all libc/libgcc
      runtime (`tests/symbols.rs::d02`).
- [x] **Phase B** — all **33** rows of `CONFIGS.md` pass across randomized
      inputs (fixed-seed xorshift64*), including two exhaustive sweeps: all 255
      one-byte inputs and all 65 025 two-byte inputs.
- [x] **Phase C** — all **6** rows of `ERRORS.md` (1, 2, 3, 4, 5, 5b) have a
      passing differential test asserting the *same* sentinel (`NULL`), not just
      "both failed". Plus null pointers, zero/oversized lengths, and one step
      past every character-range boundary.
- [x] **Every configuration** — `check_all_configs.sh` enumerates the feature
      combinations from `Cargo.toml` (there is no `[features]` table, so:
      default and `--no-default-features`) and runs the full suite in both the
      `debug` and `release` profile: 4 configurations × 44 tests, all passing.
      `tests/symbols.rs::d04` asserts each run really loads the `.so` built for
      its own profile, so a debug run cannot silently re-test the release
      artifact.

## Reaching the unreachable branches

Two of the C's error branches only fire when the allocator fails, and one needs
a >2 GiB string. `tests/fixtures/interpose.c` (a test fixture — nothing in
`c_src/` was modified) is `LD_PRELOAD`ed into a re-exec of the test binary and
interposes `calloc`/`malloc`/`free`/`strlen`, which **both** libraries import
dynamically. Injection is keyed on the exact requested byte count so the test
harness's own allocations are never disturbed:

* row 3 — `calloc(1, strlen+14)` forced to fail ⇒ both return `NULL`.
* row 4 — `malloc(strlen+1)` forced to fail ⇒ both return `NULL`, **and** the
  shim confirms the `dest` pointer was handed to `free` before the return.
  Since the return value is `NULL`, no outside caller could have freed it, so
  this proves neither implementation leaks `dest`.
* row 5 — `strlen` reports `INT_MAX` for one marker pointer, so
  `int l = strlen(src) + 1` wraps to `INT_MIN`, the sign-extended `calloc` size
  becomes astronomical, and both return `NULL` — the real integer-overflow path,
  without allocating 2 GiB.

The same shim also powers an **allocation-traffic differential** (`[E-trace]`):
for 210 inputs it compares the exact `calloc`/`malloc` byte counts and the
`calloc`/`malloc`/`free` call counts between C and Rust, and pins them to what
the C source literally says (`calloc` size `strlen+14`, `malloc` size
`strlen+1`, exactly one of each plus one `free`).

## Evidence the tests can actually detect divergence

Passing tests only mean something if they can fail. `mutation_check.sh` injects
29 deliberate bugs into `translation/src/lib.rs`, rebuilds, and requires the
suite to react correctly:

* **27 non-equivalent mutants — all 27 caught.** Covers every `decode` range and
  offset, `is_base64` membership, a genuine sign-extension bug (accepting
  negative bytes), all three output-byte bit expressions, both `'='` suppression
  checks, the quartet loop bounds and stride, the `'A'` defaults for `c2`/`c3`,
  both allocation sizes, the `strlen + 1`, both `free`s, and both halves of the
  `if (src && *src)` guard.
* **2 mutants are provably semantics-preserving and correctly still pass** —
  the suite does not raise false alarms:
  - `is_base64` comparing the `'a'..'z'` range as `u8` instead of `char`: for
    signed `char`, `0x80..0xFF` are negative and fail `>= 'a'`; as `u8` they are
    `128..255` and fail `<= 'z'`. Both forms reject the identical set.
  - `(b3 & 0x3) << 6` → `(b3 & 0x7) << 6`: the extra bit lands at `1 << 8` and
    is discarded by the store into a byte. (The neighbouring `& 0x1`, which does
    drop a live bit, *is* caught — so this is truncation, not a blind spot.)

## Notable C behaviours faithfully reproduced

These look like bugs but are the ground truth, and the Rust matches them:

* Non-base64 characters are **ignored**, never rejected — so a non-empty string
  with no base64 characters at all returns a non-`NULL`, all-zero buffer rather
  than an error.
* `'='` does **not** terminate decoding: padding in the middle of the input is
  decoded as the value 63 and the following quartets are still processed.
* Only `c3`/`c4` suppress output bytes; a `'='` in slot 2 suppresses nothing.
* A truncated final quartet defaults its missing members to `'A'` (value 0),
  so a 1-character input still emits 3 bytes.
* `int l = strlen(src) + 1` truncates a `size_t` to `int`, and the negative
  result is sign-extended back to `size_t` at the `calloc`/`malloc` call.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation
cargo build --release && cargo test --release   # 44 tests
./check_all_configs.sh                          # 4 configurations
./mutation_check.sh                             # 29 mutants
```
