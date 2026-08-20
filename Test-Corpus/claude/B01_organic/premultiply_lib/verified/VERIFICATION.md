# VERIFICATION.md — completion gate

Reproduce everything with `./run_all.sh` (enumerates feature combinations,
`cargo check`s each, builds the C reference with CMake, then runs Phases B/C/D
against a **dev** and a **release** Rust `.so`).

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0 missing** symbols in the Rust `.so`
      and **0 undefined non-libc** symbols. The C `.so` defines exactly one
      symbol (`premultiply`); the Rust `.so` defines exactly the same one.
      The C library is a single translation unit (`c_src/src/lib.c`, 20 lines,
      one function) and `src/lib.rs` is a complete translation of it — no module
      or file was skipped, and there are no stubs (`grep -E
      'unimplemented!|todo!'` on `src/lib.rs` → nothing).
      Automated as `phase_d_symbol_parity` + an independent `nm -D` / `comm`
      diff in `run_all.sh`.
- [x] **Phase B** — all **28** rows of `CONFIGS.md` pass across randomized
      inputs (fixed seed `0x5EED_1234_ABCD_F00D`). `tests/phase_b_configs.rs`,
      28 tests.
- [x] **Phase C** — all **20** rows of `ERRORS.md` have a passing error-path
      differential test, including the two fatal rows compared by *signal*
      (`SIGSEGV` = 11 for both libraries), plus the generic boundaries: null
      pointers, wild pointers, zero lengths, oversized lengths, one-step-past
      values, and a 25×25 matrix of extreme `int` values pushed across the FFI
      boundary. `tests/phase_c_errors.rs`, 21 tests.
- [x] **Phase D / every build configuration** — `Cargo.toml` declares **no
      `[features]`** and `c_src` has no `option()` / `#ifdef`, so the complete
      set of feature combinations is the single empty one. It is verified with
      `--no-default-features` and `--all-features`, and Phases B–C are re-run
      against both the **dev** and the **release** `.so` (release is where
      `panic = "abort"` applies).

Totals: **51 differential tests**, all passing; ~150 000 individual
C-vs-Rust `.so` invocations per run.

## Extra assurance beyond the gate

| cross-check | result |
|-------------|--------|
| Rust `.so` vs C built at `-O0`, `-O1`, `-O2`, `-O3`, `-Ofast` | all 5 pass Phases B and C |
| Rust `.so` built with `-C target-cpu=native` | passes Phases B and C |
| Rust `.so` built with `-C debug-assertions=on -C overflow-checks=on` | passes Phases B and C |
| bare `cargo test` with no pre-built artifacts | passes (harness self-builds both `.so`s) |
| full clean run (`rm -rf target c_src/build && ./run_all.sh`) | ALL CHECKS PASSED |
| arithmetic core | **exhaustively** verified: all 65 536 `(colour, alpha)` byte pairs on every channel (`cfg01`, `cfg03`, `cfg28`) |

## Divergences found and fixed in the Rust (the C was never changed)

1. **`SIGABRT` instead of `SIGSEGV` on `premultiply(NULL)`.**
   `rustc` emits a null-pointer debug assertion for a raw-pointer place
   dereference (`(*img).w`), so with `-C debug-assertions` on the Rust aborted
   with a panic while the C segfaulted. Fixed by loading the `cp_image_t` fields
   through a byte-wise `c_load` helper (`core::ptr::read::<u8>` +
   `wrapping_add`) and the pixel bytes through `core::ptr::read`/`write` +
   `wrapping_offset` — none of which carry a null-ness or alignment
   precondition. The faulting behaviour now matches the C in *every* profile.
   (`ERRORS.md` row 1.)

2. **`(uint8_t)(float)` conversion made literal.** The store is now
   `(x as i32) as u8`, mirroring GCC's `cvttss2si %xmm0,%edx` + `mov %dl,(%rax)`
   rather than relying on Rust's saturating `f32 as u8`. `cfg28` proves by
   exhaustion that the intermediate is always in `[0.0, 255.0]`, so the two can
   never disagree.

3. **Index arithmetic made literal.** `data[i + k]` is now
   `data.wrapping_offset(i as isize + k)`, matching GCC's `cltq` + `lea k(%rax)`
   (sign-extend `i`, then add `k` in 64 bit) instead of a 32-bit
   `i.wrapping_add(k)`.

## Blind spot the harness caught in my own analysis

The first draft of `ERRORS.md` rows 7/8 asserted that a negative `w` or `h` is
always a no-op. The differential harness rejected the precondition: with
`w = -1000000, h = 1000`, `4·w·h ≡ +294 967 296 (mod 2³²)`, so `end > 0` and the
C really does premultiply pixels. Rows 7/8 were re-scoped to the non-wrapping
magnitudes and new rows (`ERRORS.md` 20, `CONFIGS.md` 27) were added for the
mixed-sign combinations whose 32-bit wrap makes `end` positive.

## Layout

```
SYMBOLS.md              Phase A — symbol inventory + parity proof
ERRORS.md               Phase A — error-surface table (20 rows)
CONFIGS.md              Phase A — configuration-surface table (28 rows)
VERIFICATION.md         this file — the completion gate
run_all.sh              automation: feature combos x profiles x phases
src/lib.rs              the Rust translation (annotated with the C it mirrors)
tests/harness/mod.rs    libloading-based differential harness + PRNG + buffers
tests/phase_b_configs.rs  28 valid-path rows
tests/phase_c_errors.rs   21 error-path rows (incl. signal-parity children)
tests/phase_d_symbols.rs   2 symbol-parity tests
```

Both libraries are always driven through `dlopen`/`dlsym` on their `.so`s — no
Rust function is ever called directly, so the `#[no_mangle] extern "C"` export
wrapper is itself under test.
