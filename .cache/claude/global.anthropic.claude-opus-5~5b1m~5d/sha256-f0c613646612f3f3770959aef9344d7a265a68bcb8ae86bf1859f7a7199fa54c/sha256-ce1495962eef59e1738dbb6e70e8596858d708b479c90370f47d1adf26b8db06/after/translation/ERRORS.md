# ERRORS.md — Phase A: error-surface table

Mechanically derived from the *complete* C source. The library is two functions
in one translation unit; the exhaustive grep below is the whole basis for this
table.

## Mechanical grep of every rejection construct in `c_src/`

```
$ grep -nE 'RETURN_ERROR|return +-|return +NULL|assert|errno|goto|if *\(|else|switch|case |\?|#if|enum ' \
      c_src/src/lib.c c_src/include/lib.h
(no matches)

$ grep -c 'return' c_src/src/lib.c
2          # line 11: `return x + y;`   (unconditional)
           # line 19: `return *(double *)&result - 1.0;` (unconditional)
```

Findings:

* **0** error-return macros (`RETURN_ERROR`, …)
* **0** `return -1` / `return NULL` / error enums / status codes
* **0** `assert` / `static_assert`
* **0** `if` / `else` / `switch` / ternary — the code is completely branch-free
* **0** explicit range checks, null checks, bounds checks
* **0** min/max constants, capacity limits, length parameters
* **0** enums in the public header (`lib.h` declares only `struct cn_rnd_t`
  and `double next_double(cn_rnd_t *)`)
* **0** `#if` / `#ifdef` conditional compilation
* the only integer literals are `23`, `17`, `26`, `1023`, `12`, `52` — all
  fixed shift/bias constants, none of them a validated bound

`next_double` therefore has **no in-band error surface at all**: every
`uint64_t` state pair is a valid input, every call succeeds, and the returned
`double` is always a normal value in `[0.0, 1.0)`. There is no reachable
`RETURN_ERROR`-style branch to mirror.

Consequently the error table consists solely of the *generic C-API boundaries*
that exist implicitly for any function with this signature. Each still gets a
differential test, because the requirement is that C and Rust **reject/behave
identically**, not merely that "both failed somehow".

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `next_double` | `rnd == NULL` (null pointer dereference at `rnd->state[0]`, `lib.c:4`) | no in-band error: dereferences address 0 → process killed by `SIGSEGV` (11). Rust must be killed by the *same* signal, not a different signal / not a Rust panic message / not a silent return. **FOUND A DIVERGENCE — see "Divergences found and fixed" below.** | `error_paths::row01_null_pointer_same_fatal_signal` | [x] |
| 2 | `next_double` | `rnd` = misaligned pointer (`(cn_rnd_t*)(base+1)` … `(base+7)`, violates the natural 8-byte alignment of `uint64_t`) | no check exists; on x86-64 both toolchains emit alignment-agnostic 64-bit loads/stores, so the call succeeds and returns the *same* value + performs the *same* state mutation as the aligned case | `error_paths::row02_misaligned_pointer_matches` | [x] |
| 3 | `next_double` | `rnd` = valid pointer, state = `{0, 0}` (the degenerate all-zero seed — the one input a "real" PRNG would reject) | no check exists; returns exactly `+0.0` (bits `0x0000000000000000`) and leaves state `{0,0}` — the generator is stuck forever. Rust must reproduce the stuck state, *not* reseed or error. | `error_paths::row03_zero_seed_is_not_rejected` | [x] |
| 4 | `next_double` | `rnd` = valid pointer, state = `{u64::MAX, u64::MAX}` (largest representable state; `x + y` at `lib.c:11` overflows `uint64_t`) | no check exists; C wraps modulo 2^64. Rust must wrap too — an `overflow-checks=on` build must **not** panic. | `error_paths::row04_max_state_add_overflow_no_panic` | [x] |
| 5 | `next_double` | one step past every "documented range": there is no documented range, so this is *the whole `u64`×`u64` domain*. Probed at every single-bit, every all-but-one-bit, and every `2^k ± 1` boundary value of both state words. | no check exists; every one of these succeeds with a bit-exact `double`. No value is out of range. | `error_paths::row05_boundary_value_sweep_never_rejected` | [x] |
| 6 | `next_double` | out-of-range enum value passed across the FFI boundary | **N/A — not reachable.** `lib.h` declares no `enum` and `next_double` takes no integral/enum parameter, so there is no int-with-no-valid-variant input to construct. Asserted structurally by grepping the header for `enum`. | `error_paths::row06_no_enum_parameters_exist` | [x] |
| 7 | `next_double` | zero-length / oversized length argument | **N/A — not reachable.** `next_double` takes exactly one parameter, a pointer; there is no length, count or capacity argument. `state` is a fixed `uint64_t[2]`. Asserted structurally. | `error_paths::row07_no_length_parameters_exist` | [x] |
| 8 | `next_double` | reading/writing one element past `state[1]` | no bounds check exists, but the C never indexes past `state[1]`: it touches exactly bytes `[0,16)` of the struct. Verified differentially with red-zone canaries around a 16-byte struct — both libraries must leave the canaries untouched, i.e. **neither** over-reads nor over-writes. | `error_paths::row08_no_out_of_bounds_write` | [x] |

Rows 1-8 all checked. `[x]` is set only after the named test passed against
both `.so`s, in every configuration driven by `run_all_configs.sh`
(3 cdylib profiles × 2 feature combinations).

## Generic FFI boundaries beyond the table

| boundary | how it is covered | test |
|----------|-------------------|------|
| same `cn_rnd_t` alternately driven by the C and the Rust `.so` | the mixed chain must be indistinguishable from a pure-C chain, proving neither side keeps hidden extra state | `generic_same_struct_shared_between_libraries` |
| re-entrancy / no global or TLS state | 8 threads, each with its own struct, each replaying 2048 pre-recorded C outputs, per profile | `generic_multithreaded_reentrancy` |
| symbol is real, not a stub | the exported `next_double` is actually `dlsym`'d and invoked 256× per profile | `symbol_parity::every_c_symbol_is_callable_through_dlsym` |

## Divergences found and fixed

Both were found by the Phase C tests; the **C was never changed**.

### 1. Misaligned `cn_rnd_t *` aborted in Rust but worked in C  (row 2)

The original translation did `let rnd = unsafe { &mut *rnd };`. Forming a Rust
reference asserts alignment, so with `debug-assertions` on the Rust `.so`
aborted:

```
panicked at src/lib.rs:66:24:
misaligned pointer dereference: address must be a multiple of 0x8 but is 0x7fe95ebfc0a1
thread caused non-unwinding panic. aborting.
```

while the C returned a value normally. **Fix:** `cn_rnd_next` now takes a raw
`*mut cn_rnd_t` and uses `ptr::read_unaligned` / `ptr::write_unaligned`, which
is what `gcc`/`clang` actually emit for `rnd->state[i]` on x86-64. Row 2 now
passes for offsets 1-7 in all three profiles.

### 2. NULL `cn_rnd_t *` aborted (`SIGABRT`) instead of faulting (`SIGSEGV`)  (row 1)

`rustc` inserts a null-pointer check on every dereference when
`-C debug-assertions` is on, and `ptr::read_unaligned` additionally carries a
`copy_nonoverlapping` non-null library precondition:

```
unsafe precondition(s) violated: ptr::copy_nonoverlapping requires that both
pointer arguments are aligned and non-null
```

C dies with `SIGSEGV` (11); the Rust died with `SIGABRT` (6) — a genuinely
observable difference for an identical input. **Fix:** `[profile.dev]` in
`Cargo.toml` now sets `debug-assertions = false` (documented in place), because
this crate is a C-ABI shared object whose pointer contract is owned by the
caller, exactly as in the C library. Crucially `overflow-checks = true` is kept
so the wrapping arithmetic (`x + y`) stays continuously verified — confirmed
independently: `-C debug-assertions=off -C overflow-checks=on` still panics on
`u64` overflow, and still faults with `SIGSEGV` (139) on a null dereference.

A third profile, `[profile.ubcheck]` (`debug-assertions = true`), is kept in
the test matrix so this trade-off stays visible and bounded: under it, **every
valid input and every error row still matches the C bit-for-bit** (row 33 of
`CONFIGS.md`); the only permitted difference is that the NULL dereference —
undefined behaviour in *both* languages — becomes a fail-safe `SIGABRT`. Row 1
asserts `SIGSEGV` parity for `dev` and `release`, and asserts that `ubcheck`
still dies fatally and never silently returns a value.

## Test-sensitivity evidence (mutation testing)

An error table is only as good as the tests' ability to fail. `mutation_check.sh`
injects 18 single-edit mutations into `src/lib.rs`, rebuilds the `.so`s and
re-runs the suite. **16 were killed; the 2 survivors are provably equivalent
mutants:**

| mutation | outcome |
|----------|---------|
| `x >> 17` → `x >> 18`; `x << 23` → `x << 24`; `y >> 26` → `y >> 25` | killed |
| `wrapping_add` → `wrapping_sub`; `wrapping_add` → plain `+` | killed |
| `value >> 12` → `value >> 13`; `>> 12` → mask | killed |
| `1023` → `1022`; `<< 52` → `<< 51`; drop the `- 1.0` | killed |
| `x ^= y ^ (y >> 26)` → `x ^= y \| (y >> 26)` | killed |
| drop the `state[0] = y` write; drop the `state[1] = x` write | killed |
| write to the wrong slot; read from the wrong slot; swap the two reads | killed |
| `- 1.0` → `- (1.0f32 as f64)` | **survived — equivalent:** `1.0f32 as f64` is exactly `1.0f64` |
| `(exponent << 52) \| mantissa` → `^ mantissa` | **survived — equivalent:** `mantissa` occupies bits 0-51 and `exponent << 52` bits 52-61, so they never overlap and `\|` ≡ `^` |
