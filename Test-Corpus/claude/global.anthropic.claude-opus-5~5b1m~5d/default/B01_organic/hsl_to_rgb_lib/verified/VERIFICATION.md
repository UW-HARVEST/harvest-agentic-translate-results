# VERIFICATION.md — how this translation was verified, and what was found

The C in `../c_src` is the ground truth. Everything below compares the **shared
objects**: both the C `.so` and the Rust `.so` are `dlopen`ed with `libloading`
(with `RTLD_NOW`, so all symbols are resolved eagerly) and called only through
their exported `hsl_to_rgb` symbol. The Rust implementation is never called
directly, so the `#[unsafe(no_mangle)] extern "C"` wrapper and the C ABI are part
of what is under test.

Every differential assertion is made against **two** Rust artifacts at once —
the `debug` and the `release` `cdylib` — because they are different codegen and
`release` additionally sets `panic = "abort"`.

## Artifacts

| file | what it is |
|---|---|
| `SYMBOLS.md` | every `nm -D` symbol of the C `.so` and its Rust counterpart; the defined-symbol diff |
| `ERRORS.md` | the error-surface table (E1–E23), derived by grepping the C for every rejection/exit |
| `CONFIGS.md` | the configuration-surface table (C1–C52), derived from the axes the C branches on |
| `tests/common/mod.rs` | harness: locates/builds both `.so`s, canary buffers, layouts, PCG32, special-value pools |
| `tests/symbols.rs` | Phase A/D symbol parity, asserted at test time |
| `tests/configs.rs` | Phase B — one test per `CONFIGS.md` row C1–C42 |
| `tests/errors.rs` | Phase C — one test per `ERRORS.md` row E1–E20 (incl. null-pointer faults in forked children) |
| `tests/fenv.rs` | Phase C — rows E21/E22, C43–C47: the FP status word the call leaves behind |
| `tests/exhaustive.rs` | rows C48–C52: **all 2^32** bit patterns of `h`, of `s`, and of `l` |
| `tests/optlevels.rs` | cross-check against the C compiled at `-O0/-O1/-O2/-O3/-Os` |
| `tests/meta.rs` | sanity: the harness really loaded two distinct Rust builds plus the C |
| `mutate.py` | negative control: 30 injected defects, verifying the suite is not vacuous |
| `verify.sh` | Phase D driver: every feature combination × both profiles + symbol diff + mutation control |
| `sweep.sh` | drives `tests/exhaustive.rs` over every residue class so the union is the whole 2^32 space |

## Reproducing

```sh
./verify.sh          # everything except the full exhaustive sweep (~1 min)
./sweep.sh all       # the complete 2^32 sweeps of h, s and l    (~9 min, 8-way parallel)
```

`verify.sh` ends with `ALL CHECKS PASSED`; `sweep.sh` ends with
`EXHAUSTIVE SWEEP: ALL PASSED`.

## Scale of the evidence

| what | count |
|---|---|
| tests in the suite | 77 (76 run + 1 crash-child helper, `--ignored`); all pass in **both** the debug and the release test profile |
| exhaustive differential comparisons (`sweep.sh`) | **21 474 836 480** = 5 × 2^32, each against **both** Rust builds |
| coverage of each exhaustive sweep | `COMPLETE (2^32 = 4294967296)` — the union of all 16 residue classes, checked arithmetically by the driver |
| randomized comparisons in the in-suite rows | ~2.5 million |
| injected defects in the negative control | 30 (20 caught, 10 proven equivalent) |

## What the C actually does (and what looks like a bug but is not)

`lib.c:27` reads

```c
} else if (h < 120.0f && h < 180.0f) {
```

where a reader expects `h >= 120.0f && h < 180.0f`. This was confirmed to be the
real code, not a misreading, by decoding the compiled constant pool:

```
0x1f7:  movss 120.0f,%xmm0 ; comiss -0x4(%rbp),%xmm0 ; jbe ...   =>  h < 120
0x205:  movss 180.0f,%xmm0 ; comiss -0x4(%rbp),%xmm0 ; jbe ...   =>  h < 180
```

Two consequences, both faithfully reproduced rather than fixed:

* the third branch is reachable **only for negative hues** (a positive `h < 120`
  was already claimed by branch 1 or 2), and
* the hue sector `[120, 180)` falls through to the final `else` and comes out
  **grey**, not cyan.

`CONFIGS.md` row C3 and `ERRORS.md` rows E4/E5 pin both, and `mutate.py`'s first
mutation ("fix the C typo") confirms the suite rejects the "corrected" version.

## Findings — three real divergences were found and fixed

Everything below was found by the phased process, not by inspection.

### 1. `[profile.dev]` turned a `SIGSEGV` into a `SIGABRT` (`ERRORS.md` E23)

The C has no null check anywhere (`ERRORS.md` E11–E13), so a null `dest`/`src`
faults. The `release` Rust `.so` faulted identically (`SIGSEGV`), but the `debug`
one aborted with `SIGABRT` because Rust's debug-assertion-gated UB checks turn the
raw-pointer dereference into a non-unwinding panic:

```
thread '<unnamed>' panicked at src/lib.rs:108:31: null pointer dereference occurred
thread caused non-unwinding panic. aborting.
```

**Fix:** `[profile.dev] debug-assertions = false, overflow-checks = false`, so the
dev artifact is as unchecked as the C. (The crate contains no integer arithmetic,
so nothing else is affected.)

### 2. A quiet-NaN hue did not raise `FE_INVALID` (`ERRORS.md` E21)

The C compiles `h >= 0.0f` and its five siblings to **`comiss`**, the *signalling*
compare, which raises the invalid-operation exception even for a **quiet** NaN.
Rust's `>=`/`<` lower to the *quiet* `ucomiss`, which does not. Instruction
census of the two objects:

```
C    : 12 comiss,  2 ucomiss
Rust :  0 comiss, 44 ucomiss
```

`tests/fenv.rs` measured **4430** inputs on which `fetestexcept` differed after
the call. This is observable: a caller can read it with `fetestexcept`, or turn it
into a trap with `feenableexcept`.

**Fix:** raise it explicitly for a NaN hue, right where the C's dispatch chain
would (`feraiseexcept(FE_INVALID)`). The flag is sticky, so one raise reproduces
the effect of up to twelve `comiss`.

### 3. Short-circuiting the arithmetic on NaN lost the `addss`/`mulss` side effect (`ERRORS.md` E22)

The `*_ss` helpers exist to pin down which NaN survives (SSE forwards the *first*
source operand's NaN, made quiet; plain Rust `+`/`*` leaves that to LLVM's
canonicalisation). They originally *skipped* the hardware operation whenever an
operand was a NaN — which also skipped the `FE_INVALID` that a **signalling**-NaN
operand must raise.

**Fix:** always perform the operation and override only the resulting *value*:

```rust
let raw = black_box(src1 + src2);
match nan_result(src1, src2) { Some(v) => v, None => raw }
```

The `black_box` is deliberate. LLVM currently keeps the dead `fadd` anyway (the
mutation "drop the black_box barrier" is reported as *equivalent*), but LLVM is
free to DCE a dead `fadd`, which would silently reintroduce the bug — so the
barrier is the only actual guarantee.

## Things that look like divergences but are provably not

Recorded because each one cost real analysis, and because a future reader should
not "fix" them:

* **`fmodf` is a different function in the two objects.** The C imports
  `fmodf@GLIBC_2.2.5`; the Rust `extern "C" { fn fmodf }` binds to the *local*
  copy that `compiler_builtins` links in statically (`nm -a` shows
  `t fmodf` at `0x4b370`). Two different implementations, so equivalence is not
  free — which is exactly why `tests/exhaustive.rs` enumerates **all 2^32** hue
  patterns (`h` is the only input that reaches `fmodf`) instead of sampling.
* **The x86 NaN that hardware *generates* is `0xffc00000`, not `0x7fc00000`.**
  The sign bit is set, unlike Rust's `f32::NAN`. Verified against the C
  toolchain for `fmodf(±Inf,2)`, `Inf-Inf`, `0*Inf`, `Inf/Inf`, `0/0`. My first
  draft of `ERRORS.md` asserted the wrong constant; the C corrected me.
* **`2.0f * l` compiles to `addss %xmm0,%xmm0`** (i.e. `l + l`), and the literal
  `1.0f *` in `m = 1.0f * (l - 0.5f*c)` is folded away entirely by gcc. Both are
  value-identical to the spelled-out form, confirmed by mutation.
* **`h/60` in `f64` then rounded to `f32` is bit-identical to `h/60f32`** for all
  2^32 hues (exhaustively checked; `f64`'s 53 bits ≥ 2·24+2, so the double
  rounding is harmless). So that mutation is genuinely equivalent, not a blind
  spot.
* **Store order and `add_ss` operand order.** The order in which the three output
  words are written cannot matter, because the C caches `h`, `s`, `l` in locals
  before its first store — which is also why aliasing (`dest == src ± k`) is
  lossless. The *operand* order of each `addss`, by contrast, does matter (it
  decides which NaN survives when the two differ), so all seven store patterns
  were read off the disassembly and matched one by one.

### 4. (not a Rust bug) The C's own NaN sign bits are not optimisation-invariant

`tests/optlevels.rs` compiles `c_src/src/lib.c` at `-O0`, `-O1`, `-O2`, `-O3` and
`-Os` and compares all five against the CMake-built ground truth and against the
Rust. Over 100 768 inputs:

* the **Rust matches the CMake-built C on every single input**, and
* the C's *own* optimisation levels disagree with `-O0` on **13 180** inputs.

Every one of those 13 180 disagreements is NaN-vs-NaN and confined to the NaN's
**sign and payload bits** — never a numeric value, never a zero's sign, never a
different branch. Cause: at `-O0`, `fabsf` is a real `andps 0x7fffffff` that
clears the NaN's sign; from `-O1` gcc folds it away and the incoming NaN's sign
survives. IEEE-754 leaves NaN sign/payload propagation unspecified, so both are
conforming.

The consequence for this translation: it is pinned to the artifact
`c_src/CMakeLists.txt` produces (which sets no `CMAKE_BUILD_TYPE`, i.e. `-O0`) —
the build the task names as ground truth. `tests/optlevels.rs` documents and
enforces the boundary: it *fails* on any disagreement that is not purely NaN bits,
so if an optimisation level ever changed a real value the suite would say so.

## Negative control

`mutate.py` injects 30 one-line defects and re-runs the suite for each. Result:

* **20 caught** — including the "fixed" typo, every sector permutation, an
  off-by-one sector bound, a swapped `addss` operand order (NaN preference), a
  premature store that breaks `dest == src+2` aliasing, an extra out-of-bounds
  4th store, `h * (1/60)` instead of `h / 60`, an `f64` intermediate for `c`,
  a dropped `FE_INVALID` raise, and raising the wrong FP flag.
* **10 reported as equivalent** — each with a written reason (commutativity,
  `x-y == -(y-x)`, exact power-of-two scaling, `%` lowering to `fmodf`,
  Figueroa's double-rounding bound, and the two `black_box` barriers).

A suite in which no mutation fails would be worthless; this one distinguishes
real defects from semantics-preserving rewrites.

## Completion gate

- [x] `SYMBOLS.md`: the C `.so` exports exactly `hsl_to_rgb`; the Rust `.so`
      exports it too; defined-symbol diff is **empty**; both objects `dlopen`
      under `RTLD_NOW`, so **0 unresolved symbols**.
- [x] Phase B: every row C1–C52 of `CONFIGS.md` passes, with randomized inputs
      per row (fixed seeds) and **exhaustive** enumeration for C48–C52.
- [x] Phase C: every row E1–E23 of `ERRORS.md` has a passing differential test
      that asserts the *specific* result (same three words, or the same fatal
      signal, or the same `fetestexcept` bits) — not merely "both failed".
- [x] Phase D: `Cargo.toml` declares no `[features]` and `src/lib.rs` contains no
      `#[cfg(feature)]`, so there is exactly one feature combination; `verify.sh`
      still runs it under both `default` and `--no-default-features`, in both the
      `dev` and `release` test profiles, and against both the debug and release
      `cdylib`.
