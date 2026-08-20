# VERIFICATION.md — completion gate

C ground truth: `c_src/src/lib.c` + `c_src/include/lib.h` (24 + 19 lines, one
translation unit, one exported function).
Rust translation: `src/lib.rs` → `libbitwriter_add_lib.so`.

Both are loaded as shared objects with `libloading` and compared through the FFI
boundary; the Rust crate is **never** linked directly, so the `#[no_mangle]`
export wrapper and the `#[repr(C)]` struct ABI are themselves under test.

## Completion checklist

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** missing symbols and **0**
      undefined non-libc symbols in the Rust `.so`. C exports exactly
      `bitwriter_add`; the Rust `.so` exports `bitwriter_add`. Verified for both
      the `debug` and `release` artefacts by `./check_symbols.sh`.
      No C source file was left untranslated, so no module needed translating.
- [x] **Phase B** — all **29** rows of `CONFIGS.md` pass across randomised
      inputs (fixed-seed SplitMix64), ~1.6 M generated cases per run plus an
      exhaustive `bits ∈ 0..=130 × bw->bits ∈ 0..=130` sweep.
      `tests/phase_b_valid.rs`, 29 tests.
- [x] **Phase C** — all **15** rows of `ERRORS.md` plus the generic-boundary
      sweep have passing error-path differential tests.
      `tests/phase_c_errors.rs`, 17 tests.
- [x] **All feature combinations** — `Cargo.toml` declares no optional features
      (the C build has no configuration axes: no `option()` in `CMakeLists.txt`,
      zero `#ifdef` in the C source), so the only combination is the empty one.
      It is run as `--no-default-features` *and* as the default, each under the
      `dev` and `release` profiles = 4 configurations, all passing
      (`./run_all.sh` → `ALL CONFIGURATIONS PASSED`).

## Divergences found and fixed

Both were in pointer handling, not arithmetic, and both were invisible to
symbol parity and to happy-path testing.

| # | input | C | Rust (before) | fix |
|---|-------|---|----------------|-----|
| 1 | `bw` at a misaligned address | returns `0`, updates state | `misaligned pointer dereference` → `SIGABRT` | replaced `&mut *bw` with `addr_of_mut!` + `read_unaligned`/`write_unaligned` |
| 2 | `bw == NULL` | `SIGSEGV` (signal 11) | Rust null check → `SIGABRT` (signal 6) | same fix; the first touch is `bw->tot` at offset 20, so the fault address is `0x14` (non-null) and a real `SIGSEGV` is raised |

Both only manifested in the `dev` profile — Rust's UB checks are compiled out in
`release` — which is why `run_all.sh` exercises both profiles.

The arithmetic translation was already correct; this was confirmed against the
`-O0` x86-64 disassembly of `lib.c`, which shows 32-bit wrapping `add`/`sub` for
`bw->tot += bits`, `bw->bits + bits`, `63 - bw->bits` and `bits -= b`, and
`shlq`/`shrq %cl` (shift count masked to 6 bits) for all four shifts.

## Negative control (proof the suite has teeth)

`./mutation_check.sh` injects deliberate bugs into `src/lib.rs`, rebuilds and
re-runs the suite. Run in four chunks (`0 11`, `11 8`, `19 6`, `25 6`):

* **25 / 25 real bugs caught** — including dropping the `- 1` off-by-one,
  `u64::MAX << 1` → `u64::MAX`, replacing hardware shift masking with
  `checked_shl().unwrap_or(0)`, masking shifts to 5 or 7 bits instead of 6,
  `wrapping_add` → `saturating_add` on `tot` and on `bw->bits`, promoting the
  loop guard to 64-bit (losing the 32-bit wraparound), `>=` → `>` in the guard,
  removing the loop cap, setting the cap to 0, reading/writing the wrong struct
  offset, OR → XOR, `shr` → `shl`, dropping `bw->val &= mask`, dropping
  `bw->bits += bits`, re-forming an aligned reference, and returning `-1` / `1`.
* **6 / 6 equivalent mutants correctly *not* caught**, each with a proof rather
  than an assumption: the loop makes **at most one progressing iteration**
  (brute-forced over 1.3 M structured + 40 M random `(bw->bits, bits)` pairs),
  after which the body is idempotent, so
  * every cap `>= 1` is observationally identical to `i < 100` — confirmed by an
    8 M-case sweep in which caps 1, 2, 3, 99, 101 and 1000 all agree with 100
    while cap 0 differs in 41 % of cases (cap 0 is therefore in the
    must-be-caught list, and is caught); and
  * `b > bits ? bits : b` ≡ `b >= bits ? bits : b`, since the arms differ only
    when `b == bits`, where they are equal.

`tests/common/mod.rs` additionally refuses to run if either `.so` is older than
its source, because `cargo test` does **not** refresh the `cdylib` artefact — a
stale `.so` silently "passed" the suite once during this work.

## Reproducing

```sh
cd translated_rust
./run_all.sh                                  # all 4 configurations, ~2 min
./check_symbols.sh debug                      # symbol parity
./mutation_check.sh 0 11                      # negative control, chunk 1 of 4
```
