# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from documentation or assumptions.

## Mechanical derivation

Every rejection/error construct was grepped for across the whole C surface
(`c_src/src/lib.c`, `c_src/include/lib.h` — the only C files):

```sh
grep -nE 'return|assert|NULL|errno|if|switch|while|for|\?|#if|goto|exit|abort|ERROR|enum' \
    c_src/src/lib.c c_src/include/lib.h
```

Result of that grep, classified:

| construct searched for | hits |
|------------------------|------|
| `RETURN_ERROR` / error macro | 0 |
| `return -1` / `return NULL` / error sentinel | 0 (the only two `return`s are `return x + y;` and `return *(double *)&result - 1.0;` — both unconditional value returns) |
| `assert` / `static_assert` | 0 |
| `if` / `switch` / `?:` / `goto` | 0 — the code is fully straight-line |
| explicit range check / min/max constant | 0 (`1023`, `52`, `23`, `17`, `26`, `12` are shift/exponent constants, not bounds checks) |
| NULL check | 0 — `rnd` is dereferenced unconditionally at `lib.c:4` |
| `errno` / `exit` / `abort` | 0 |
| `enum` declaration | 0 — the API has no enum parameters |
| length / count / size parameter | 0 — `next_double(cn_rnd_t *)` takes no length |

**The C library has no error-return surface whatsoever.** `next_double` is
total over every possible 128-bit state: it always returns a `double` and never
signals failure. Consequently the error surface consists *only* of the generic
FFI boundary conditions, which are enumerated below and are all tested.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `next_double` | `rnd == NULL` — dereferenced with no NULL check at `lib.c:4` (`rnd->state[0]`) | undefined behaviour: reads address `0x0` ⇒ process dies with `SIGSEGV` (11). No error code is returned. Rust must die the same way, with the same signal. **Observed: C `Signal(11)`, Rust `Signal(11)`.** | `null_pointer_terminates_identically` | [x] |
| 2 | `next_double` | `rnd` is a **misaligned** `cn_rnd_t*` (byte offset 1..7 into a buffer) — the C code has no alignment check and loads `uint64_t` directly | undefined behaviour per ISO C, but on x86-64 the generated `mov` performs an unaligned load and the function returns the same value it would for the same bytes. Rust must return the identical `f64` bit pattern and write back identical bytes. | `misaligned_pointer_matches` | [x] |
| 3 | `next_double` | zero-valued input: `state = {0, 0}` (the xorshift128+ degenerate/"all-zero" state that a real PRNG would reject) — the C code does **not** reject it | no error: `value == 0`, `mantissa == 0`, `result == 0x3FF0000000000000` ⇒ returns exactly `+0.0`, and the state stays `{0, 0}` forever | `zero_state_is_not_rejected` | [x] |
| 4 | `next_double` | maximal/saturated input: `state = {u64::MAX, u64::MAX}` — no upper-bound check exists | no error: computes normally with C wrapping semantics (`x << 23` truncates, `x + y` wraps mod 2^64) | `saturated_state_is_not_rejected` | [x] |

## Generic C-API boundaries also covered (per instructions), even though the
## C code contains no corresponding check

| boundary | applicability here | how covered |
|----------|--------------------|-------------|
| NULL pointer | applies (`rnd`) | row 1 — differential crash-signal comparison in a child process |
| zero length | **N/A** — `next_double` has no length/count/size parameter | the "zero" analogue is the all-zero state, row 3 |
| oversized length | **N/A** — no length parameter | the "maximal" analogue is the all-ones state, row 4 |
| value one step past a documented valid range | **N/A** — no parameter has a documented range; every one of the 2^128 states is accepted. All 128 bits are exercised individually (single-bit sweep, `CONFIGS.md` rows 6–8) | `CONFIGS.md` rows 6–8 |
| out-of-range enum value across FFI | **N/A** — the public API declares no `enum` and takes no integer mode/flag parameter (`grep -n enum` → 0 hits). There is no int-typed parameter whose value could be out of range; the only parameter is a struct pointer. Verified, not assumed. | n/a |
| unaligned pointer | applies | row 2 |
| out-of-bounds write past the struct | applies (struct is 16 bytes; C writes `state[0]`, `state[1]`) | guard-byte canaries in `CONFIGS.md` row 17 |

## Divergences found and fixed (Rust side only; `c_src/` untouched)

Both were found by the Phase C tests, not by inspection.

| # | divergence | C behaviour | original Rust behaviour | fix |
|---|------------|-------------|--------------------------|-----|
| 2 | misaligned `cn_rnd_t *` | unaligned `mov` (`-O0`) / `movups` (`-O2`); returns a value | `&mut *rnd` tripped the compiler's `misaligned pointer dereference` check ⇒ non-unwinding panic ⇒ **`SIGABRT` (6)** | `src/lib.rs`: access the state through the raw pointer only |
| 1 | `rnd == NULL` | hardware fault ⇒ **`SIGSEGV` (11)** | after the first fix, `ptr::read_unaligned` tripped the `ptr::copy_nonoverlapping requires ... non-null` precondition ⇒ **`SIGABRT` (6)** | `src/lib.rs`: `load_u64`/`store_u64` emit a bare `mov` via `core::arch::asm!` on x86-64, so the access itself faults exactly as the C's does — in **debug and release** alike (rustc 1.94 enables these UB checks whenever `debug_assertions`/`ub_checks` are on) |

Re-verified after the fix: `C = Signal(11)`, `Rust = Signal(11)` for NULL, and
byte-identical results plus byte-identical buffers for offsets 1..7.

## Harness validation (proof the tests are not vacuous)

Two deliberate mutations of `src/lib.rs`, each reverted afterwards:

| mutation | detected by |
|----------|-------------|
| `x ^= x >> 17` → `x ^= x >> 18` | 17 tests failed (rows 2–18) |
| drop the `state[0] = y` write | 17 tests failed (row 13 state comparison first) |
