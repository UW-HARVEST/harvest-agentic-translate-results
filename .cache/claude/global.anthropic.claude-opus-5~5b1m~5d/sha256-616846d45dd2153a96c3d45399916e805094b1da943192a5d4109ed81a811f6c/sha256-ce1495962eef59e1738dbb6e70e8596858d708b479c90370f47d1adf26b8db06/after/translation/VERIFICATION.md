# Verification report

Differential verification of `translation/` (Rust) against `c_src/` (C, ground
truth). Every call in every test goes through `dlopen`/`dlsym` (`libloading`) on
**both** shared objects — the Rust crate is never linked directly, so the
`#[no_mangle]` / `extern "C"` export wrappers and the SysV struct-passing ABI are
themselves under test.

## Reproduce

```sh
# 1. C shared object (exactly as documented; no CMAKE_BUILD_TYPE => -O0)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. everything, for every feature combination and both cargo profiles
cd translation && ./check_features.sh
```

`./check_features.sh` builds the `cdylib` in **release and debug**, diffs
`nm -D` against the C `.so` for each, and runs the whole suite. The test harness
loads *every* Rust `.so` it finds (`target/release` **and** `target/debug`) and
compares each independently against the C `.so`.

```
== combination: <default>
ok   <default> / release : symbol parity (12 symbols)
ok   <default> / debug   : symbol parity (12 symbols)
ok   <default>           : 107 tests passed
== combination: --no-default-features
ok   --no-default-features / release : symbol parity (12 symbols)
ok   --no-default-features / debug   : symbol parity (12 symbols)
ok   --no-default-features           : 107 tests passed
== summary
ALL FEATURE COMBINATIONS VERIFIED
```

## Completion gate

| gate | status |
|------|--------|
| `SYMBOLS.md`: `nm -D` shows 0 missing / 0 undefined non-libc symbols in Rust | **PASS** — 12/12, empty diff |
| Phase B: every one of the 59 `CONFIGS.md` rows passes across randomized inputs | **PASS** — 59/59 |
| Phase C: every one of the 42 `ERRORS.md` rows has a passing error-path test | **PASS** — 42/42 |
| All of the above under EVERY feature combination | **PASS** — no `[features]` table ⇒ 2 builds (`<default>`, `--no-default-features`) × 2 profiles, all green |

## Test inventory (107 tests)

| file | tests | covers |
|------|-------|--------|
| `tests/common/mod.rs`            | — | harness: FFI types, dual `dlopen`, `diff()`, SplitMix64 PRNG, IEEE-754 special-value grid, geometry steering helpers |
| `tests/smoke.rs`                 | 2  | both `.so`s load; all 12 symbols callable; `circle_collide` reference values |
| `tests/phase_b_primitives.rs`    | 26 | `CONFIGS.md` rows 1–26 (`c2V`, `c2Sub`, `c2Dot`, `c2Mulvs`, `c2Maxv`, `c2Minv`, `c2Clampv`) |
| `tests/phase_b_kernels.rs`       | 20 | `CONFIGS.md` rows 27–46 (the three `c2Circleto*` kernels, all branches) |
| `tests/phase_b_dispatch.rs`      | 13 | `CONFIGS.md` rows 47–59 (`c2Collided`, `circle_collide`, composed pipeline, all-symbol sweep) |
| `tests/phase_c_errors.rs`        | 42 | `ERRORS.md` rows 1–42 |
| `tests/nan_operand_order.rs`     | 4  | pins the C's SSE NaN-propagation convention (see below) |

All floating-point comparisons are on **raw bits** (`f32::to_bits`), so `+0.0`
vs `-0.0` and differing NaN payloads/signs are failures, not passes.

## Divergences found and fixed

Two real bugs, both found by feeding distinct NaN payloads through the primitives
and both confirmed against the C's disassembly before fixing.

IEEE-754 does not specify *which* NaN a binary op returns. On SSE the rule is:
if the **destination** operand is a NaN, the destination is returned (quieted if
signalling); otherwise, if the source is a NaN, the source is returned. So the
NaN that comes out identifies which operand the compiler placed in the
destination register — and `fmul`/`fadd` are commutative in LLVM IR, so Rust and
GCC are free to pick opposite orders.

### 1. `c2Dot` — wrong operand order in *two* of three operations

`objdump` of the C `.so`:

```asm
movss  -0x8(%rbp),%xmm1   ; xmm1 = a.x
movss  -0x10(%rbp),%xmm0  ; xmm0 = b.x
mulss  %xmm0,%xmm1        ; px = mulss(dst = a.x, src = b.x)
movss  -0x4(%rbp),%xmm2   ; xmm2 = a.y
movss  -0xc(%rbp),%xmm0   ; xmm0 = b.y
mulss  %xmm2,%xmm0        ; py = mulss(dst = b.y, src = a.y)   <-- a/b SWAPPED
addss  %xmm1,%xmm0        ; res = addss(dst = py,  src = px)   <-- px/py SWAPPED
```

The translation had `add(mul(a.x,b.x), mul(a.y,b.y))` — right for the `x`
product, wrong for the `y` product and wrong for the sum. Observed:

| input | C | Rust (before) |
|---|---|---|
| `a=(n1,1)  b=(n2,1)`  | `7fc00001` | `7fc00001` ✓ |
| `a=(1,n1)  b=(1,n2)`  | `ffc00002` | `7fc00001` ✗ |
| `a=(n1,n3) b=(n2,n4)` | `ffc00004` | `7fc00001` ✗ |
| `a=(n1,n3) b=(1,1)`   | `7fc00003` | `7fc00001` ✗ |

Fixed to `add_keep_lhs_nan(mul_keep_lhs_nan(b.y, a.y), mul_keep_lhs_nan(a.x, b.x))`.

### 2. `c2Mulvs` — destination operand was the scalar instead of the vector

The doc comment claimed GCC vectorises to `mulps` with the broadcast scalar as
destination. It does not at the documented optimisation level:

```asm
movss  -0x8(%rbp),%xmm0   ; xmm0 = a.x
mulss  -0xc(%rbp),%xmm0   ; mulss(dst = a.x, src = b)
```

| input | C | Rust (before) |
|---|---|---|
| `c2Mulvs((n1,n3), n2)` | `(7fc00001, 7fc00003)` | `(ffc00002, ffc00002)` ✗ |

Fixed to `mul_keep_lhs_nan(a.x, b)` / `mul_keep_lhs_nan(a.y, b)`.

### 3. `A.r + B.r` in both `c2CircletoCircle` and `c2CircletoCapsule` (pinned)

GCC emits `addss(dst = B.r, src = A.r)`; the Rust had `A.r + B.r`. This is
**not** externally observable — both functions only return `d2 < r2`, which is
false for every NaN regardless of payload — but it is now pinned to match, so no
latent mismatch is left behind.

## What was *not* a bug (verified, not "fixed")

- `c2Sub`, `A.r*A.r`, `r2*r2`, and `da / c2Dot(n,n)` are non-commutative or
  same-operand, so their orders are fixed by the language; confirmed identical.
- `c2Maxv`/`c2Minv` are `movss` selects of `a` or `b`, so `a > b ? a : b`
  (unordered ⇒ `b`, `±0` ⇒ `b`, SNaN returned un-quieted) is exactly what LLVM's
  `maxss`/`minss` lowering does. Confirmed bit-identical, including SNaN
  payloads and signed zeros.
- `c2Collided` with a NaN in `B.a` only can legitimately return **`1`**: both
  `da<0` and `db<0` are false, so the after-B-cap arm runs and never touches the
  poisoned `B.a`. Three of my initial test expectations were wrong about this and
  about `inf < inf`; the C was right in all three cases and the *tests* were
  corrected, never the C.

## Assumption worth knowing: the C's optimisation level

The NaN operand orders above are an artifact of instruction scheduling, so they
change with `-O`. Measured on the same `lib.c`:

| build | `c2Dot` NaN results | `c2Mulvs((n1,n3), n2)` |
|-------|---------------------|------------------------|
| `-O0` **(the documented build)** | `7fc00001 ffc00002 ffc00004 7fc00003` | `(7fc00001, 7fc00003)` |
| `-O1` / `-O2` / `-Os` | `7fc00001 7fc00001 7fc00001 7fc00001` | `(7fc00001, ffc00002)` |
| `-O3` / `-Ofast` | `7fc00001 7fc00001 7fc00001 7fc00001` | `(ffc00002, ffc00002)` |

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no `-O` flag, so the
documented build command produces `-O0`, and that is what the translation is
pinned to and verified against. `tests/nan_operand_order.rs` asserts the C
`.so`'s observed convention explicitly: if the C is ever rebuilt with
optimisation, those tests fail with a message naming the exact pin to move in
`src/lib.rs`, instead of the mismatch surfacing as a mysterious payload diff.

This affects **only** direct calls to `c2Dot`/`c2Mulvs` with two *different* NaN
bit patterns. Every integer-returning entry point — including the entire public
header surface (`circle_collide`) — is unaffected at every optimisation level.

## Notes

- `c_src/` is unmodified: `md5sum` of `lib.c` = `4a2c9ca7dff835275c7888245640c8ef`,
  `lib.h` = `59896bd5bd560642bbed9f9cae9958c1`,
  `CMakeLists.txt` = `8889366dbb1535b8a848547af64fbc79`.
  Only `c_src/build/` (cmake output) was created.
- `check_features.sh`'s own failure detection was validated by temporarily
  adding a deliberately failing test: the script correctly reported
  `VERIFICATION FAILED` and exited 1. (An earlier version silently passed,
  because piping into `grep -q` under `set -o pipefail` made the pipeline
  inherit cargo's non-zero exit status.)
- `cargo check --all-targets` is warning-free.
