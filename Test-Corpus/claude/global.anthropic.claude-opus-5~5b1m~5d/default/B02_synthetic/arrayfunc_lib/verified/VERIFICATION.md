# Verification report

Differential verification of the Rust translation of `c_src/src/lib.c` against the
C original. Both are built as shared libraries and loaded with `libloading`; the
Rust code is **never** called directly, so the `#[no_mangle]` export wrappers are
part of what is tested.

## How to reproduce

```
cd translation
./run_all.sh          # everything, including mutation testing (~4 min)
./run_all.sh quick     # skip mutation testing (~1 min)
cargo test             # just the differential suite (builds both .so files itself)
```

## Result

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc symbols | **PASS** (11 / 11) |
| `CONFIGS.md`: every row passes across randomized inputs | **PASS** (33 / 33) |
| `ERRORS.md`: every row has a passing error-path differential test | **PASS** (36 / 36) |
| All of the above under every feature combination | **PASS** (6 configurations = 3 feature selections × 2 profiles) |
| The suite would actually catch a regression | **PASS** (40 / 40 injected bugs caught) |

84 tests, all green in both the debug and the release profile.

## Artifacts

| file | contents |
|------|----------|
| `SYMBOLS.md` | Phase A symbol surface; the `nm -D` diff and how it is enforced |
| `ERRORS.md` | Phase A/C error-surface table — 36 rows, one per distinct rejection in the C |
| `CONFIGS.md` | Phase A/B configuration-surface table — 33 rows over 9 branching axes |
| `tests/common/mod.rs` | harness: builds and `dlopen`s both `.so`s, ABI type declarations, seeded PRNG, byte-exact comparators |
| `tests/phase_b_valid.rs` | Phase B — 30 valid-path rows (C1–C30) |
| `tests/phase_c_errors.rs` | Phase C — 38 error-path tests (E1–E36 + generic boundaries + deterministic out-of-bounds marching) |
| `tests/crash_probes.rs` | Phase C — out-of-process signal-parity probes for the rows that kill the process |
| `tests/phase_d_symbols.rs` | Phase D — symbol parity, ABI layout, feature-space guards |
| `run_all.sh` | full verification driver |
| `check_features.sh` | enumerates the feature power set from `cargo metadata`, runs the suite per combination |
| `mutation_check.sh`, `mutate.py` | injects 45 deliberate bugs to prove the suite has teeth |

## Bugs found and fixed in the Rust

### 1. `modulo_operation(INT32_MIN, -1)` — wrong termination behaviour (row E2)

The C compiles `a % b` to a single `idivl`. For `INT32_MIN % -1` the implicit
quotient (`2147483648`) does not fit in `eax`, so the CPU raises `#DE` and the
process dies with **SIGFPE**. Measured against the compiled C: exit status 136
(128 + 8).

The Rust used `a.wrapping_rem(b)`, which silently returns `0`. Neither native
Rust operator reproduces the C: `a % b` panics (→ SIGABRT under
`panic = "abort"`), and `wrapping_rem` returns `0`. Fixed by emitting the
instruction the C compiler emits:

```rust
core::arch::asm!(
    "cdq",
    "idiv {b:e}",
    b = in(reg) b,
    inout("eax") a => _,
    out("edx") rem,
    options(nomem, nostack),   // deliberately NOT `pure`: the trap is observable
);
```

Both libraries now terminate with signal 8. A portable `wrapping_rem` fallback is
kept behind `#[cfg(not(target_arch = "x86_64"))]`, where the C compiler would not
emit `idiv` either.

### 2. `process_with_foreach(arr, NULL)` with `count == 0` — spurious abort (row E23)

The C only dereferences `op` inside the `FOREACH` body, so with `count == 0` a
NULL `op` is harmless and the function returns `0`. The Rust hoisted
`op.unwrap_unchecked()` *above* the loop, which (a) is UB even when the loop body
never runs and (b) aborts with SIGABRT under rustc's UB instrumentation. Fixed by
resolving the pointer inside the loop, and by calling through a `transmute`d raw
pointer rather than `unwrap_unchecked` — so a NULL `op` with `count > 0` jumps to
address 0 and yields **SIGSEGV**, matching the C, instead of SIGABRT.

### 3. `compute_weighted_sum` — `offset_from` out of bounds for oversized `count`

A caller can hand-write `ResultArray.count` past the 10-element `data[]`
capacity; the C then marches out of bounds with no bounds check. The Rust used
`current.offset_from(base)`, whose safety contract requires both pointers to be
within the same object — violated for `i >= 10`. Replaced with the numerically
identical `if i > 0 { i } else { 1 }` (a pointer difference in elements *is* `i`),
and base pointers are now derived via `arr.cast::<Result>()` instead of
`(*arr).data.as_mut_ptr()` so provenance covers the whole struct rather than just
the `data` field. Covered by `e24_deterministic_out_of_bounds_marching_*`.

## Things the C does that are deliberately preserved

These looked like bugs but are the C's actual behaviour, so the Rust reproduces
them:

* `compare_results_in_array` bounds-checks **only** the upper limit, so negative
  indices are accepted and compared by address (row E15).
* `compute_weighted_sum` uses `weight = 1` for `i == 0`, not `0`, because
  `current > base` is false there (row E28).
* `init_result_array` stores a negative `count` verbatim rather than clamping to
  `0` (row E21).
* `arrayfunc`'s final compare loop runs to `count - 1`, and signed overflow
  throughout wraps two's-complement as gcc `-O0` emits.

## Notes on methodology

**Byte-for-byte comparison.** `ResultArray` is compared by serialising every
*defined* byte — all ten elements' `value`, the raw bit pattern of `scaled`, and
`rank`, plus `count`. The three padding holes per struct are excluded because C
leaves them indeterminate; comparing them would test the two compilers' scratch
memory rather than the translation. Comparing `scaled` as bits (not as `f64`)
means NaN payloads and `-0.0` are caught.

**Instrumentation and the release artifact.** The signal-parity probes compare the
**release** cdylib against the C `.so`. A debug cdylib carries rustc's optional UB
instrumentation, which converts a null-pointer dereference into `abort()`
(SIGABRT) before the hardware fault (SIGSEGV) can occur; CMake builds the C with
no comparable instrumentation, so the uninstrumented release artifact — the one an
external consumer links — is the correct counterpart when comparing *how* a
process dies. Every defined-behaviour comparison runs against both profiles.

**One row is not signal-deterministic.** Row E24 (negative `count`) makes the
`FOREACH` loop march forward writing 24 bytes at a time until the process dies.
Repeated runs show *both* implementations producing SIGSEGV on some runs and
SIGABRT on others (heap-metadata corruption reaching `abort()` first) — including
runs where the C aborted and the Rust segfaulted. Asserting signal equality there
would be asserting a coin flip, so the test asserts that neither returns normally,
and the substance of the row — identical out-of-bounds address arithmetic and
writes — is verified deterministically on a 5 000-element mapped buffer with a
large positive `count`, which reaches the same code path.

**Mutation testing.** 45 source mutations were injected. 40 changed observable
behaviour and were all caught. The other 5 are provably behaviour-preserving and
are asserted to *survive*, which documents the analysis instead of leaving an
unexplained survivor:

| mutation | why it cannot change behaviour |
|----------|--------------------------------|
| `safe_double_to_int`: `>= INT_MAX` → `> INT_MAX` | only `d == 2147483647.0` changes path, and `(int)2147483647.0 == INT32_MAX` |
| `safe_double_to_int`: `<= INT_MIN` → `< INT_MIN` | only `d == -2147483648.0` changes path, and it is exactly representable as `int` |
| `arrayfunc`: loop bound `count-1` → `count` | the extra iteration calls `compare(count-1, count)`, whose `idx2 >= count` guard returns `0` |
| `compute_scaled_value`: `safe_double_to_int(x)` → `x as c_int` | Rust's float→int `as` cast saturates and maps NaN to 0, coinciding exactly with the three guards (this would *not* hold in C, where the cast is UB) |
| `init_result_array`: `count < 10` → `count < 11` | the branches only disagree for `count == 10`, and both yield `10` |
