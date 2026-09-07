# ERRORS.md — Phase C error-surface table

Mechanically derived from an exhaustive grep of the *entire* C source
(`c_src/src/lib.c`, 20 lines; `c_src/include/lib.h`, 7 lines) for every
rejection mechanism:

```sh
grep -nE 'return -|return NULL|RETURN_ERROR|assert|errno|if *\(|NULL|\?|goto' c_src/src/lib.c c_src/include/lib.h
```

Findings: **zero** `if` statements, **zero** `assert`s, **zero** error enums,
**zero** error-return statements, **zero** `NULL` checks, **zero** range
checks, **zero** min/max constants, **zero** `errno` uses, **zero** `goto`s.

`next_double` is *total* over its declared domain: every one of the
2^128 possible `cn_rnd_t` states is a valid input that produces a `double`.
The function has no failure mode and no sentinel return value — `-1.0` is not
reachable, and neither is any NaN/Inf (see row 5).

The error surface therefore consists only of the **generic C-API boundaries**
that exist implicitly for any function taking a pointer, plus the value
boundaries of the arithmetic. Each is listed as its own row and each has a
differential test.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `next_double` | `rnd == NULL` — unchecked dereference of `rnd->state[0]` at `lib.c:4` | no error code: memory fault. Process dies from `SIGSEGV` (signal 11). C and Rust must terminate with the **same** signal / exit status. **Verified: both = `signal 11`, exit code `None`.** (**divergence D2 found & fixed here** — see below.) | `err_01_null_pointer_same_fatal_signal` |
| 2 | `next_double` | `rnd` points to a heap block of **exactly** `sizeof(cn_rnd_t)` == 16 bytes, i.e. one byte past the object is unmapped/poisoned | no error: reads and writes stay within `state[0..2]`; no out-of-bounds access. Both must succeed with identical value and identical 16-byte final state. | `err_02_exact_size_allocation_no_overread` |
| 3 | `next_double` | unaligned `cn_rnd_t*` (odd byte offset inside a buffer) — the C code does an aligned-typed access on a misaligned pointer | on x86-64 both perform the unaligned 8-byte accesses and return the identical value; no trap. (**divergence D1 found & fixed here** — see below.) | `err_03_misaligned_pointer` |
| 4 | `next_double` | all-zero state `{0, 0}` — the degenerate xorshift fixed point (would be the "invalid seed" for the algorithm, but the C rejects nothing) | **no rejection.** `x = 0`, `y = 0` → `x` stays 0, state stays `{0,0}`, `value = 0`, `mantissa = 0`, `result = 0x3FF0000000000000` → returns exactly `0.0`. Sticks at `0.0` forever. | `err_04_all_zero_state_is_accepted_fixed_point` |
| 5 | `next_double` | state chosen so the raw `value` has all 52 high bits set (`mantissa == 0xF_FFFF_FFFF_FFFF`), the largest representable output — one step past would overflow into the exponent field | `result = 0x3FFFFFFFFFFFFFFF` (== `2 - 2^-52`) → returns bits `0x3FEFFFFFFFFFFFFE` == `1.0 - 2^-52`, exactly. Never `>= 1.0`, never NaN/Inf: `mantissa` is masked to 52 bits by `>> 12`, so `(1023 << 52) \| mantissa` can never disturb the exponent field. | `err_05_mantissa_all_ones_boundary` |
| 6 | `next_double` | `x + y` at `lib.c:11` overflows `uint64_t` (unsigned wraparound — defined in C, must be `wrapping_add` in Rust, **not** `+` which panics in debug) | wraps modulo 2^64; no trap, no error. Rust must produce the same wrapped `value` and hence the same `double`. | `err_06_return_sum_wraps_modulo_2_64` |
| 7 | `next_double` | `x << 23` at `lib.c:7` with the top 23 bits of `x` set — bits shifted out of the 64-bit type | bits are discarded (shift count 23 < 64, so well-defined); identical result required. | `err_07_left_shift_23_discards_high_bits` |
| 8 | `next_double` | `y >> 26` at `lib.c:9` with `y < 2^26` so the shift yields 0, and `x >> 17` with `x < 2^17` likewise | shifts to zero, no error; identical result required. | `err_08_right_shifts_to_zero` |
| 9 | `next_double` | *out-of-range enum value across the FFI boundary* | **not applicable**: the public API declares no `enum`, no mode/flag parameter, and no integer parameter at all. `next_double` takes exactly one argument, a `cn_rnd_t*`. There is no int-typed parameter whose value could fall outside a valid variant set — every `uint64_t` state bit pattern is in range (row 4/5 cover the extremes). | documented in `err_09_no_enum_or_scalar_parameters_exist` |
| 10 | `next_double` | zero / oversized *length* argument | **not applicable**: the API takes no length, size, or count argument. Row 2 covers the only size-related concern (the fixed 16-byte object). | documented in `err_09_no_enum_or_scalar_parameters_exist` |

## Checklist

- [x] 1 null pointer → same fatal signal
- [x] 2 exact-size allocation, no over-read/over-write
- [x] 3 misaligned pointer
- [x] 4 all-zero degenerate state accepted, not rejected
- [x] 5 mantissa all-ones upper boundary, output stays `< 1.0`
- [x] 6 `x + y` unsigned wraparound
- [x] 7 `x << 23` high-bit discard
- [x] 8 right shifts collapsing to zero
- [x] 9 no enum / scalar parameter exists (documented + asserted by inspection test)
- [x] 10 no length parameter exists (documented + asserted by inspection test)

## Divergences found and fixed

Both were found by the Phase C error-path tests, in the **debug** profile only;
the happy-path Phase B tests and the release build passed throughout. Both were
fixed in the Rust (the C was never touched).

### D1 — misaligned `cn_rnd_t *` aborted instead of returning a value

* **Symptom:** `err_03_misaligned_pointer` / `cfg_25_misaligned_pointer`
  ```
  panicked at src/lib.rs:66: misaligned pointer dereference:
  address must be a multiple of 0x8 but is 0x7f24677fa371
  thread caused non-unwinding panic. aborting.        -> SIGABRT (6)
  ```
  C returned a `double`; Rust killed the process.
* **Cause:** `next_double` did `let rnd = unsafe { &mut *rnd };`. Forming a Rust
  reference from a misaligned raw pointer is UB and is trapped by the
  `debug_assertions` alignment check.
* **Fix:** `cn_rnd_next` now takes a `*mut cn_rnd_t` and accesses the state with
  `core::ptr::addr_of_mut!` + `read_unaligned` / `write_unaligned`. No reference
  is ever formed. This is a genuine code-level fix: the misalignment tests pass
  even with `RUSTFLAGS="-C debug-assertions=on -C overflow-checks=on"`.

### D2 — `next_double(NULL)` died with SIGABRT instead of SIGSEGV

* **Symptom:** `err_01_null_pointer_same_fatal_signal`
  ```
  C   : code=None signal=Some(11)   # SIGSEGV
  Rust: code=None signal=Some(6)    # SIGABRT
  ```
  Both "failed", but with *different* fatal signals — exactly the kind of
  not-merely-both-failed mismatch this phase exists to catch.
* **Cause:** the library UB-check on `read_unaligned` tests the pointer for null
  and panics *before* the load can fault, converting the hardware fault into an
  abort. The C has no such check and faults on the `mov`.
* **Fix:** `[profile.dev] debug-assertions = false` / `overflow-checks = false`
  in `Cargo.toml`, so the debug `.so` carries the same instrumentation as the
  release `.so` (i.e. none) and matches the C in every cargo profile.
  Rationale recorded inline in `Cargo.toml`: these checks are Rust-only
  instrumentation with no counterpart in the C, and `overflow-checks` would also
  panic on the `x + y` wraparound that C defines as modular.
* **Documented residual:** if the crate is *forced* to build with
  `RUSTFLAGS="-C debug-assertions=on"` (a non-default override that contradicts
  the profile settings), Rust's null precondition check re-appears and the NULL
  case again yields SIGABRT rather than SIGSEGV. Every other row, including D1,
  passes under that override. Parity holds for all cargo profiles and all
  feature combinations the crate actually defines.

## Harness sensitivity (proof the tests can fail)

Nine mutants were injected into `src/lib.rs`, each built into its own `.so` and
loaded in place of the real one. Every mutant was caught:

| mutant | failing tests |
|---|---|
| `value >> 12` → `>> 13` | 31 |
| `x ^= x << 23` → `<< 22` | 33 |
| `x ^= x >> 17` → `>> 16` | 33 |
| `y >> 26` → `y >> 25` | 33 |
| `wrapping_add` → `wrapping_sub` | 31 |
| `exponent = 1023` → `1022` | 36 |
| drop the `- 1.0` | 35 |
| `state[0] = y` → `= x` | 32 |
| swap the two state writes | 33 |

## Harness hazard fixed

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` artifact, so a
stale `libnext_double_lib.so` can be silently verified — this initially masked
D1. Mitigations now in place:

* `guard_loaded_shared_objects_are_not_stale` fails if either `.so` is older
  than its source;
* the harness loads the `.so` from the **same profile** the test binary was
  built into (derived from `current_exe()`, not from mtime and not from
  `cfg!(debug_assertions)`, which the profile settings above would make lie);
* `run_verification.sh` always `cargo build`s before `cargo test`.
