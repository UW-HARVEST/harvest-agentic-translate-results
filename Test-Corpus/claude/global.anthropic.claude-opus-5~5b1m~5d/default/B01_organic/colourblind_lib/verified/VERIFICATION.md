# Verification report — `colourblind` C → Rust

Reproduce everything with one command:

```sh
cd translation && ./scripts/verify_all.sh
```

## Completion gate

| gate | result |
|---|---|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols in the Rust `.so` | ✅ |
| Phase B: every one of the **29** `CONFIGS.md` rows passes across randomised (and, for rows 25-28, exhaustive) inputs | ✅ |
| Phase C: every one of the **11** `ERRORS.md` rows has a passing error-path differential test | ✅ |
| All of the above under **every** feature combination **and both profiles** (4 configurations) | ✅ |

48 tests (47 running + 1 ignored child-process helper), ~830 M compared calls
per profile, all outputs compared **bit-exactly** via `f32::to_bits`.

## What is being compared

Both libraries are loaded as shared objects with `libloading` and called only
through the exported `colourblind` symbol — the Rust crate is never linked or
called directly, so the `#[no_mangle] extern "C"` wrapper is under test too.

* C: `c_src/build/libharvest-work-QUHmNR.so` (CMake sets no `CMAKE_BUILD_TYPE`,
  so GCC 11.5 at `-O0`)
* Rust: `translation/target/<profile>/libcolourblind_lib.so`

## Surface

The library is one translation unit: three `static` helpers plus one exported
function. `nm -D --defined-only` yields exactly `colourblind` for both `.so`s —
symbol diff empty in both directions. No module was skipped, nothing is stubbed,
and the three `static` helpers are correctly *not* exported. Full inventory in
`SYMBOLS.md`.

## Divergences found and fixed

Both were found by the differential tests, and both were fixed in the Rust (the C
was never touched).

### 1. Misaligned pointers aborted instead of computing

`cfg_row19_misaligned_layout` / `err_row09_misaligned_pointers` failed under
`cargo test` (debug):

```
panicked at src/lib.rs:130: misaligned pointer dereference:
address must be a multiple of 0x4 but is 0x7fc9ee3f32d1
thread caused non-unwinding panic. aborting.
```

GCC compiles the C's `*Red` to a bare `movss`, which places **no** alignment
requirement on the address, so the C library transforms a deliberately unaligned
`float*` perfectly happily. Rust's `*red` promises 4-byte alignment, and any
build with `-C debug-assertions` enforces that promise with an abort.

### 2. NULL pointers faulted with the wrong signal

`err_row07_null_pointers_fault_identically` then failed in debug:

```
row07 r-null, impairment cbProtanopia (0): C terminated with signal=Some(11)
but Rust with signal=Some(6)
```

`SIGSEGV` vs `SIGABRT` — again Rust's UB checks firing where the C simply
executes the instruction and takes the page fault.

### Fix

Both accesses now go through `sse::load` / `sse::store`, which emit a literal
`movss` via `asm!`. That is exactly the instruction GCC emits, so both Rust
profiles now match the C bit-for-bit on aligned, unaligned and null pointers.
The read/write **order** is unchanged (all three components are read before any
is written, then stored Red → Green → Blue), which is what makes aliased
pointers behave identically. Confirmed in the release disassembly: the three
`movss` loads all precede the first store.

## Anti-vacuity gates

Passing tests prove nothing if the harness is not actually comparing two
different libraries. Three gates guard against that.

### Stale-artifact trap (a real bug in the first version of this harness)

The crate is `crate-type = ["cdylib"]` and no test target links it, so **cargo
never rebuilds it during `cargo test`**. The first version of the suite was
therefore comparing the C against a `.so` from an earlier `cargo build`, and
passed happily with a deliberately broken `src/lib.rs`. `tests/common/mod.rs`
now builds the `cdylib` itself (once per process, for the test binary's own
profile) and asserts the artifact is newer than every file in `src/`.

### Mutation testing — `scripts/mutation_check.py`

30 mutants injected into `src/lib.rs`; each must produce the expected verdict.

* **26 behavioural mutants — all 26 caught.** Coefficient tweaks, `addss`
  operand-order swaps, re-association, `f64` intermediates, FMA contraction,
  reversed store order, read-after-write, clamping, NaN canonicalisation,
  signed-zero normalisation, subnormal flushing, dispatch swaps, a `NULL` guard,
  and an `export_name` ABI break.
* **4 mutants are provably equivalent — all 4 correctly accepted.** These
  document which parts of the transcribed operand order are load-bearing:
  * `mulss` operand order is **free**: every `mul` in the C has one constant
    (never-NaN) operand, so the destination-wins NaN tie can never fire.
  * `R` → `R * 1.0f` is **free**: it only quietens an sNaN, which the consuming
    `addss` would quieten anyway.
  * `x - c*y` → `x + (-c)*y` is **free**: IEEE defines subtraction that way, and
    negating the constant gives exactly the negated product. So GCC's `subss`
    and LLVM's preferred "add a negated constant" rewrite are interchangeable
    *here*.
  * Swapping tritanopia's two `0.8739092…` coefficients is **free**: they are
    different decimal literals that round to the *same* `f32` (`0x3F5FB885`).
    All **seven** near-duplicate literal pairs in `lib.c` do this — which is why
    GCC emitted a single shared `.rodata` entry for each pair, and the Rust
    release build independently does the same.

### Harness self-checks — `tests/phase_d_meta.rs`

Asserts the two `.so` paths differ and their bytes differ; that each impairment
actually changes its input (so "outputs match" cannot mean "neither library did
anything"); that the three impairments produce *different* results; that the
comparison rejects a 1-ULP, signed-zero, NaN-payload and NaN-sign difference;
that an out-of-range impairment is observably different from a valid one; and a
9-vector known-answer table recorded from the C (so a change to *both* sides at
once is still caught).

### Row/test cross-check — `scripts/check_artifacts.py`

Verifies in both directions that every `CONFIGS.md` / `ERRORS.md` row references
a test that exists and passes, that every `cfg_row*` / `err_row*` test is
referenced by some row, that row numbers are contiguous, and that every row is
checked off.

## Notes on fidelity that were verified directly against the C binary

* **All 24 float coefficients** match GCC's `.rodata` bit-for-bit, including the
  7 pairs that collapse to shared constants.
* **Every `mulss` / `addss` / `subss` operand order** in `src/lib.rs` was checked
  against `objdump -d` of the C object file, instruction by instruction, for all
  three helpers and all nine output expressions.
* **The dispatch is unsigned.** GCC emits `cmpl $0x2,-0x4(%rbp); ja`, so a
  negative `int` impairment becomes a large unsigned value and falls through to
  the no-op — it does **not** alias `cbProtanopia`. Covered by `ERRORS.md` rows
  1-6, including out-of-range enum values, `INT_MIN`, and a dirty upper half of
  `rdi`.
* **x86's default NaN is negative.** `INF + (-INF)` yields `0xFFC00000` in both
  libraries (known-answer vector 8).
* **Signed zero is not symmetric across the three outputs.** Protanopia on
  `(-0,-0,-0)` gives `(-0, +0, +0)`, because `(-0)+(-0) = -0` but
  `(-0)-(-0) = +0` (known-answer vector 7).

## Error surface

The C has **no** error-reporting channel at all: zero `return` statements, zero
error macros/enums, zero `errno` writes, zero `assert`, zero NULL checks, zero
range checks, zero loops, zero `if`, zero preprocessor conditionals. Its only
rejection is the `default`-less `switch`, which silently leaves all three floats
untouched. `ERRORS.md` therefore asserts the *sentinel* (bit-identical
passthrough), not merely "both failed somehow", and covers the generic C-API
boundaries — NULL, misalignment, aliasing, out-of-range enum values across the
FFI boundary, and one step past every float boundary — with the C's actual
observed behaviour for each.
