# Verification report

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `c_src/src/lib.c`. Both are compiled to shared libraries and
loaded side by side with `libloading`; **no Rust function is ever called
directly**, so the `#[no_mangle] extern "C"` export wrappers are under test too.

Reproduce everything with:

```
./run_all.sh          # build both .so, run all phases, all feature combos, both profiles
./mutation_check.sh   # negative control: prove the suite detects real divergences
python3 check_coverage.py   # prove every ERRORS/CONFIGS row maps to a real test
```

## Result

**No behavioural divergence was found.** `src/lib.rs` is byte-for-byte faithful
to the C for every input exercised, and needed no changes. Two defects were found
and fixed **in the test harness** (see "Harness defects found" below) — both of
which had been making the suite report false passes.

## Build-time configuration surface

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` has no
options or `#ifdef`s, so there is exactly **one** feature combination. It is
verified under **both** cargo profiles, because `[profile.release]` sets
`panic = "abort"` and enables optimisations that can alter float→int codegen:

| # | feature combination | profile | result |
|---|--------------------|---------|--------|
| 1 | *(empty / default)* | dev     | 6 test binaries, 63 test functions, all pass |
| 1 | *(empty / default)* | release | 6 test binaries, 63 test functions, all pass |

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` on the C `.so` lists 6 exports; the Rust `.so`
      exports all 6 under identical names. The symbol diff is **empty**. The Rust
      `.so` loads under `RTLD_NOW`, proving 0 unresolved non-libc symbols. No C
      module was left untranslated (`c_src` contains a single `src/lib.c`, and
      every non-`static` function in it is translated). Asserted by
      `tests/phase_d_symbols.rs` (4 tests) and `check_coverage.py`.
- [x] **Phase B** — all **40** `CONFIGS.md` rows pass across randomized inputs
      (fixed-seed SplitMix64, ~4000 cases per row). `tests/phase_b_valid.rs`
      (32 tests, rows 1–30/39/40) and `tests/phase_b_doubleneg.rs` (rows 31–38).
- [x] **Phase C** — all **30** `ERRORS.md` rows have a passing error-path
      differential test asserting the *same* sentinel/value, not merely "both
      failed". `tests/phase_c_errors.rs` (25 tests, rows 1–24/30) and
      `tests/phase_c_doubleneg_errors.rs` (rows 25–29).
- [x] **Every feature combination** — the single combination passes in both
      profiles (table above).

## What the tests actually compare

* Return values of all six exports (`int` exactly; `double` by **raw bits**, so
  `-0.0` vs `0.0` and NaN payload differences count as divergences).
* Full output buffers of `create_numeric_buffer`, including a sentinel-filled
  tail, so a write past `size` is caught.
* **Byte-for-byte stdout** of `doubleneg` (~1.5 KiB per call), captured by
  `dup2`-ing fd 1 to a temp file around each call. This covers `%d` / `%e` /
  `%ld` formatting and GCC's `printf("lit\n")` → `puts("lit")` rewrite (which is
  why `puts` appears in the C `.so`'s imports but not in the C source).
* Cross-library composition (row 40): a buffer produced by C is searched by Rust
  and vice versa, so the intermediate byte representation is verified, not just
  the final accumulator.

## Negative control (mutation testing)

Matching symbols and green happy-path tests prove nothing on their own, so
`mutation_check.sh` injects 50 known changes into `src/lib.rs` and checks the
suite's verdict:

| category | count | result |
|----------|-------|--------|
| real behavioural divergences | 45 | **45 caught** (0 blind spots) |
| provably behaviour-preserving | 5 | **5 survived** (suite does not over-specify) |

The 5 survivors were each proven equivalent on x86-64 by exhaustive/large-scale
enumeration rather than assumed to be:

1. `(x % 256) as i8` ≡ `x.rem_euclid(256) as i8` — both keep the low 8 bits
   (0 differences over 3M values incl. `INT_MIN`/`INT_MAX`).
2. Dropping the `(char)` narrowing in `find_value_in_buffer` — `memchr` converts
   its `int` argument to `unsigned char` anyway (0 differences over 3M values).
3. `%ld` → `%d` for the memchr offset — the offset is always `0..=255`, so both
   render identically.
4. and 5. The lower-bound branch in `double_to_int_trunc`
   (`truncated < -2147483648.0`) is **unobservable**: Rust's `as i32` already
   saturates to `i32::MIN` below `-2^31`, which coincides with x86-64
   `cvttsd2si`'s "integer indefinite" `0x80000000` (0 differences over 20M random
   `f64` plus exhaustive ULP walks around both endpoints). The branch is
   harmless and kept because it documents the C semantics explicitly.

## Harness defects found and fixed

These are the reason the first "all green" run was meaningless.

1. **Stale-library false pass (critical).** `cargo test` does **not** rebuild a
   `crate-type = ["cdylib"]` lib target — only `cargo build` emits the `.so`. The
   first mutation run scored **0 caught / 17 survived** because every test was
   loading a stale `.so`. Fixed by adding a hard staleness guard in
   `tests/common/mod.rs` (`assert_not_stale`) that refuses to run if the `.so` is
   older than any of its sources, and by making `run_all.sh` /
   `mutation_check.sh` always `cargo build` before `cargo test`.
2. **Swallowed failures.** A panic raised while fd 1 was redirected left stdout
   pointing at the capture file, so libtest's panic message and
   `test result: FAILED` went to `/dev/null` — a real failure looked like a pass
   and the mutation script's text-matching missed it. Fixed with a `Drop` guard
   that always restores fd 1, plus exit-code-based (not grep-based) detection in
   `mutation_check.sh`.

Additional hardening: `capture_stdout` asserts the captured C output contains
`doubleneg`'s expected marker lines, so a silently broken capture cannot pass;
a `REDIRECT_ACTIVE` flag turns nested/concurrent fd-1 redirection into a loud
panic (which is why the stdout-capturing tests each live in their own
single-`#[test]` binary).

## Notable C behaviours deliberately replicated

* `(int)double` uses x86-64 `cvttsd2si` (confirmed by `objdump`): out-of-range
  values, `±INFINITY` and `NaN` all yield `0x80000000` = `-2147483648`. In
  `doubleneg`, `-1.0 * pow(2,40)` therefore *always* converts to `INT_MIN`, and
  `INT_MIN % 1000 == -648` is always folded into the result.
* C's `%` truncates toward zero, so `(seed + i*7) % 256` and `c % 10` are
  **negative** for negative operands (not Euclidean).
* Signed `char` on this platform: `create_numeric_buffer` stores negative bytes.
* `create_numeric_buffer(buf, 256, seed)` is a **permutation** of all 256 byte
  values because `gcd(7, 256) == 1`. Consequently the two "not found" branches
  inside `doubleneg` (`ERRORS.md` rows 25, 26) are **unreachable**; the tests
  assert that neither implementation ever takes them, rather than pretending to
  exercise them.
* `size <= 0` in `create_numeric_buffer` writes nothing (the loop bound is
  signed — it does not wrap into a huge unsigned count).

## Out of scope

Inputs where the **C itself** is out of bounds, so the only "expected result"
would be a shared segfault rather than a value: `find_value_in_buffer` /
`create_numeric_buffer` with a `size` larger than the real allocation, and
`buffer == NULL` with `size > 0`. Both implementations forward to the same glibc
`memchr` / perform the same stores, so there is no behavioural difference to
observe. `NULL` with `size <= 0` *is* tested (rows 3, 20).
