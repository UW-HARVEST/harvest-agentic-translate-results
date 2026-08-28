# VERIFICATION.md — completion gate

Differential verification of `translation/` (Rust) against `c_src/` (C, ground
truth). Both are built as shared objects and both are called **only** through
`dlopen`/`dlsym` (`libloading`), so the `#[no_mangle]` export wrappers and the
x86-64 SysV struct-passing convention are part of what is tested; the Rust
implementation is never called directly (the crate is `crate-type = ["cdylib"]`
only, so it cannot be).

## How to reproduce

```sh
# C reference (as instructed; no CMAKE_BUILD_TYPE => -O0)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

cd translation
cargo test                 # whole suite; harness builds the cdylib itself
./check_all_features.sh    # every feature combo x {release, debug} cdylib
./mutation_check.sh        # negative control: does the suite actually bite?
```

The harness builds whatever it needs (the C `.so` via cmake, the Rust cdylib via
a side-target-dir `cargo build`), so a bare `cargo test` in a clean tree works.
`RUST_SO_PATH` / `C_SO_PATH` override either artifact.

## Completion checklist

- [x] **`cargo check` clean** — 0 errors, 0 warnings, for every feature
      combination and `--all-targets`.
- [x] **`SYMBOLS.md`: `nm -D` diff is EMPTY.** The C `.so` exports exactly one
      symbol, `to_barycentric`; the Rust `.so` exports it under the identical
      name. 0 undefined non-libc symbols in the Rust `.so`. Enforced at test
      time by `d1`, `d3`; the `static` helpers are asserted absent from **both**
      libraries by `d2`; `d4` asserts no C source file was skipped.
- [x] **Phase B: all 24 `CONFIGS.md` rows pass** across ≈2.6 M randomised
      comparisons (fixed-seed splitmix64), compared as raw `u32` bit patterns so
      `±0.0` and NaN payloads both count.
- [x] **Phase C: all 18 `ERRORS.md` rows pass**, each asserting the *specific*
      sentinel (`0xFFC0_0000`, `±inf`, the quieted payload) — not merely "both
      failed somehow".
- [x] **Every feature combination.** `Cargo.toml` declares no `[features]`, so
      `{}` ≡ `--no-default-features` ≡ `--all-features`; all three are still run
      explicitly, against both a **release** and a **debug** cdylib (different
      codegen, same required bit-exact behaviour). 6 configurations × 55 tests.
- [x] **Negative control.** 20 injected mutants behave exactly as predicted:
      15 observable ones caught, 5 provably-unobservable ones survive.

## Test inventory (55 tests)

| file | tests | covers |
|------|-------|--------|
| `src/lib.rs` (unit) | 5 | pinned bit patterns from the C |
| `tests/phase_b_valid.rs` | 24 | `CONFIGS.md` rows B1–B24 |
| `tests/phase_c_errors.rs` | 17 | `ERRORS.md` rows E1–E17 (+E6b inside `e16`) |
| `tests/phase_d_symbols.rs` | 6 | symbol parity, source completeness, harness provenance |
| `tests/phase_d_build_sensitivity.rs` | 3 | NaN-payload provenance, reference-build guard |

## Findings

### 1. No bug found in the translation

Across every configuration and ≈2.6 M randomised inputs per feature combo, the
Rust `.so` and the C `.so` returned bit-identical `lm_vec2` values. No change to
`src/lib.rs` was needed. `nm -D` parity was already exact.

The translation faithfully reproduces the C's quirks rather than fixing them:

* the returned pair is `(u, v)` where `u` runs along the `p3 - p1` edge and `v`
  along `p2 - p1` — because the C builds `v0` from `p3` and `v1` from `p2`;
* `1.0f / (dot00*dot11 - dot01*dot01)` has **no** degeneracy guard, so collinear
  and coincident inputs propagate `inf`/`NaN` instead of being rejected.

### 2. The NaN payload is a property of the reference *binary*, not the C source

This is the substantive result, and it constrains how the translation must be
read. On x86-64 the scalar SSE ops are two-operand (`mulss dst, src` ⇒
`dst = dst op src`) and, when more than one operand is NaN, the result keeps the
**destination** operand's payload (quieting it if it was signalling). Which value
the compiler parks in the destination register is a register-allocation decision.
Measured on two builds of the *same* `c_src/src/lib.c`:

| inputs | `-O0` vs `-O2` agreement |
|--------|--------------------------|
| NaN-free | 200 000 / 200 000 — perfect |
| NaN-carrying | 13 302 / 200 000 — they disagree 93 % of the time |

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no optimisation flags, so
the reference `.so` is `-O0`. The Rust translation matches that build on every
input, including all 293 439 sampled cases where `-O2` would have answered
differently (`d8`). The `-O0` disassembly fixes each destination operand:

```
lm_sub2   subss              -> dst = minuend            (left)
lm_dot2   mulss %xmm0,%xmm1  -> dst = a.x                (LEFT)
          mulss %xmm2,%xmm0  -> dst = b.y                (RIGHT)
          addss %xmm1,%xmm0  -> dst = the y term         (RIGHT addend)
body      every mulss/subss/divss -> dst = left operand
```

and `sub_dst_lhs` / `mul_dst_lhs` / `mul_dst_rhs` / `add_dst_rhs` /
`div_dst_lhs` in `src/lib.rs` encode exactly that. Test `d9` fails loudly, with
an explanation, if `CMakeLists.txt` ever gains optimisation flags — because the
helpers would then need recalibrating.

**Caveat, stated plainly:** bit-exactness on multi-NaN inputs is guaranteed
against the reference build produced by `c_src/CMakeLists.txt` as it stands. It
is not, and cannot be, guaranteed against an arbitrarily-optimised build of the
same C — no Rust source could be, since the C itself does not agree with itself
across `-O` levels there. All NaN-free behaviour is optimisation-independent and
verified as such.

### 3. Five internal choices are unobservable through the ABI

`mutation_check.sh` found that 5 of the 20 mutants cannot be detected by *any*
test, and each has a proof (in `ERRORS.md`): the `lm_sub2` operand order is
already what `SUBSS` does; the denominator's `mulss`/`subss` payload is always
masked by a NaN numerator; `dot01*dot01` has identical operands; and the
dividend of the reciprocal is the literal `1.0f`. These are equivalent mutants,
not gaps — but the distinction is only visible because the mutants were run.

### 4. What Phase C could *not* test, and why

`ERRORS.md` records that null-pointer, zero/oversized-length and out-of-range-
enum rejections are **structurally unreachable** here: the public header is

```c
lm_vec2 to_barycentric(lm_vec2 p1, lm_vec2 p2, lm_vec2 p3, lm_vec2 p);
```

— four 8-byte `{float, float}` aggregates by value, no pointer, no count, no
enum, and `float` has no trap representation. The equivalent adversarial input
for this ABI is the full 2^32 domain of each of the 8 fields, which row E17
fuzzes (400 000 fully-random cases plus an exhaustive sweep of all 256 high
bytes per slot). `e17_no_pointer_or_enum_params` re-greps the header and
`src/lib.c` at test time, so this justification fails loudly rather than going
stale if the C ever grows a pointer, an enum, or a branch.
