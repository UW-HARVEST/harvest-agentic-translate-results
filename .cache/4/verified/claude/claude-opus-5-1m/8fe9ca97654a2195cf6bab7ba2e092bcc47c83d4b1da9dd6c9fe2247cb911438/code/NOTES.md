# NOTES.md -- translation decisions, and the places where bit-identity is not
# achievable in principle

## Verification status

| | |
|---|---|
| C symbols exported by `cbuild/libcdriver.so` | 63 (47 functions incl. `main`, 15 data objects, 1 in `.bss`) |
| of those exported by the Rust cdylib | **63** (missing: 0) |
| `w_*` hooks for the header's macros / `static ID_INLINE` functions | 36, all present on both sides |
| `CONFIGS.md` rows (valid-path configurations) | 67, all checked |
| `ERRORS.md` rows (rejection / edge conditions) | 50, all checked (2 of them are non-terminating C loops, replicated and asserted instead of executed) |
| differential tests | 101, in 9 test binaries, all passing |
| build configurations verified | 4 (`--no-default-features`, default, `--all-features`, `--release`) |
| mutations injected to prove the suite is not vacuous | 12, all caught (`./mutation_check.sh`) |

## How to reproduce the verification

```sh
./build_c.sh                 # cbuild/libcdriver.so, cbuild/libcwrap.so, cbuild/cdriver
cargo build --offline        # target/debug/libdriver.so + target/debug/driver
./check_symbols.sh           # Phase D symbol parity (must print "0 missing symbols")
./run_all.sh                 # every test in every feature combination
./mutation_check.sh          # proof that the suite is not vacuous
DIFF_STRICT_NAN=1 cargo test # the NaN-payload survey (see Deviation 1)
```

`cargo build` before `cargo test` is not optional: `cargo test` builds the
*rlib* but not the *cdylib*, so a source edit that is not followed by
`cargo build` would be checked against a stale `target/debug/libdriver.so`.
`tests/harness/mod.rs` guards against that -- it compares the mtimes of the three
shared objects against their sources and fails loudly instead of silently testing
old code.

## What was translated

The previous translation step only covered `VectorNormalizeFast`, `Q_rsqrt` and
the `DotProduct` macro -- the three things reachable from `main()`.  All of
`q_math.c` (46 functions + 15 data objects) and all of the executable parts of
`q_shared.h` (22 macros + 8 `static ID_INLINE` functions) are now translated:

| Rust file | C source |
|---|---|
| `src/q_math.rs` | `c_src/src/q_math.c`, 1:1, every exported symbol |
| `src/q_shared.rs` | the types, macros, `static ID_INLINE` functions and constants of `c_src/inc/q_shared.h` |
| `src/lib.rs` | `c_src/src/main.c` (exported as the `main` symbol, reused by the `driver` binary via `#![no_main]`) |
| `src/wrappers.rs` | `w_*` test hooks so the header's internal-linkage code can be reached through `dlsym` |
| `src/cstd.rs` | the exact `atof` / `printf("%f")` behaviour `main.c` relies on |

## Faithfulness decisions worth knowing

1. **`f32` everywhere `vec_t` is used.** `typedef float vec_t`, and x86-64 has
   `FLT_EVAL_METHOD == 0`, so every vector expression is evaluated in single
   precision. Where the C source mixes in a `double` (`M_PI`, `2.0`, `0.5`,
   `360.0/65536`, `65536/360.0`, `180.0`) the translation promotes to `f64` at
   exactly the same place and rounds back on assignment, e.g.
   `AngleNormalize180` subtracts `360.0` in `f64`.
2. **`(int)` casts of floats are NOT Rust's `as`.** Rust's `as` saturates; C
   leaves it undefined and gcc emits `cvttss2si`/`cvttsd2si`, which return
   `0x80000000` for NaN and for anything out of range. `q_shared::f32_to_i32`,
   `f64_to_i32` and `f32_to_byte` reproduce that. This is load-bearing for
   `AngleMod`, `AngleNormalize360`, `ANGLE2SHORT`, `SnapVector`, `ColorBytes3`
   and `ColorBytes4` (verified by `angle_normalize_int_overflow`,
   `angle2short_overflow`, `snap_vector_overflow`, `color_bytes_out_of_range`).
3. **Signed overflow wraps.** `Q_rand`'s `69069 * *seed + 1` and
   `ColorIndex`'s `c - '0'` use `wrapping_*`, matching gcc's two's-complement
   result (and avoiding a debug-build panic that C does not have).
4. **`sqrt`/`sin`/`cos`/`atan2` are the libm ones.** Rust's `f64::sqrt` etc.
   lower to calls into the very same glibc the C library links, so the results
   are bit-identical. `sqrt` is called on a `f64` and rounded to `f32`, like
   `(vec_t)sqrt(...)` in C.
5. **Raw pointers, source order, no aliasing assumptions.** Every pointer
   parameter stays a raw pointer, including the ones that may be `NULL`
   (`DirToByte`, `AngleVectors`) and the ones callers are allowed to alias.
   Reads and writes happen in the same order as the C statements, so
   `MakeNormalVectors(f, f, up)`, `VectorNormalize2(v, v)`,
   `MatrixMultiply(m, m, m)`, `ClearBounds(b, b)` and friends produce the same
   results as gcc -O0 does (tested).
6. **`static float sr, sp, sy, cr, cp, cy;` in `AngleVectors`** are always
   assigned before use, so they are plain locals in Rust; they are `float`
   either way, so no extra rounding is lost.
7. **The chained assignments** (`mins[0] = mins[1] = mins[2] = 99999`,
   `zrot[0][0] = zrot[1][1] = zrot[2][2] = 1.0F`,
   `tempvec[0] = tempvec[1] = tempvec[2] = 0.0F`) are written out
   right-to-left, reading back each stored value, so that the aliased cases
   behave identically.
8. **`#![no_main]` for the binary.** The C `.so` exports `main` (it is one of
   the two translation units `CMakeLists.txt` compiles), so the Rust cdylib has
   to export `main` too. Having the `driver` binary declare `#![no_main]` and
   let the C runtime call that exported `main` avoids a second, drifting copy of
   `main.c`, and is what the C build does as well.

## Deviation 1 -- NaN payload selection when two *different* NaNs meet

IEEE 754 and ISO C both leave open which NaN a binary operation returns when
both operands are NaN. x86-64 `addss`/`mulss` return the *first source operand*
quieted, and gcc -O0 does not pick the source order consistently -- not even
within one expression. In `_DotProduct` the first product is

```
mulss  %xmm0,%xmm1     # dest = xmm1 = v1[0]  -> left operand is src1
```

and the second is

```
mulss  %xmm2,%xmm0     # dest = xmm0 = v2[1]  -> right operand is src1
```

so `_DotProduct` returns the NaN payload of `v2[2]` for some inputs and of
`v1[0]` for others. Reproducing that would mean encoding gcc's register
allocation, expression by expression and call site by call site, into the Rust
source; a different `-O` level or gcc version would need a different encoding.

A second pattern can enter the computation in two ways, both of which need a
non-finite or overflowing input:

* an invalid operation manufactures the x86 default NaN `0xffc00000`
  (`inf - inf`, `0 * inf`, `sin(inf)`, an overflowing product, ...);
* a NaN gets negated (`right[1] = -forward[0]`, `VectorMA(right, -d, ...)`,
  `forward[2] = -sp`, `-1*cr*-sy`, ...), which flips its sign bit.

What the translation guarantees instead:

* Every finite, non-overflowing input is compared **bit for bit**, including all
  signed zeroes and all denormals.
* For non-finite inputs everything is still compared bit for bit -- the
  NaN-ness itself, every finite result, every infinity, every sign -- with the
  single exception that when *both* sides return a NaN, its payload and sign bit
  are not compared (`harness::check_f32` / `check_vec`, gated on
  `harness::nan_payload_ambiguous`).
* Where no arithmetic is involved (`_VectorCopy`, `AxisCopy`, `Q_fabs`,
  `IS_NAN`, `VectorCompare`, the data symbols) every NaN payload *is* compared
  strictly, including signalling NaNs.

### Where it was worth eliminating: `AngleVectors`

`AngleVectors` writes `right[0] = (-1*sr*sp*cy + -1*cr*-sy)` and friends. gcc
folds every `-1 * x` and every double negation into a single `xorps` sign flip,
and `xorps` flips the sign bit of a NaN whereas `mulss` by `-1.0f` does not --
so this was a systematic, reproducible difference, not a register-allocation
accident. `src/q_math.rs` therefore spells those nine expressions the way gcc
folds them (and in the operand order gcc's `mulss`/`addss` use, which is exact
for every non-NaN value because both operations are commutative). Result:
`AngleVectors` and `AnglesToAxis` are bit-identical for all 5 832 combinations of
NaN/inf/finite angles, see `tests/nan_payloads.rs`.

### Where it was not: the inlined `DotProduct`/`CrossProduct` sums

`DIFF_STRICT_NAN=1 cargo test` disables the tolerance and shows the complete
residual list -- 12 of 99 tests, always and only a NaN-vs-NaN payload/sign
difference:

| test | entry point(s) |
|---|---|
| `scalar::lerp_angle` | `LerpAngle` |
| `vectors::dot_product` | `_DotProduct` |
| `vectors::vector_ma` | `_VectorMA` |
| `vectors::matrix_multiply` | `MatrixMultiply` |
| `vectors::vector_rotate` | `VectorRotate` (inlined `DotProduct`) |
| `angles::project_point_on_plane` | `ProjectPointOnPlane` |
| `angles::make_normal_vectors` | `MakeNormalVectors` |
| `angles::rotate_point_around_vector` | `RotatePointAroundVector` |
| `angles::rotate_around_direction` | `RotateAroundDirection` |
| `planes::plane_from_points` | `PlaneFromPoints` (inlined `CrossProduct`) |
| `qshared::macro_forms` | the `DotProduct`/`VectorMA` macros |
| `qshared::lengths_and_distances` | `VectorLength`, `Distance`, ... |

Each of these is a sum of products where gcc -O0 picks the `mulss`/`addss` first
source operand per call site (`_DotProduct` alone uses the left operand for the
first product and the right one for the second). Matching that would mean
transcribing gcc's register allocation into every one of ~60 expressions -- and
the correct transcription would depend on the compiler and the `-O` level used
to build the reference. The suite pins the class instead
(`tests/nan_payloads.rs::documented_nan_payload_divergences_are_nan_on_both_sides`).

Two concrete examples found by the test suite:

* `LerpAngle(inf, inf, NaN)` -- C returns `0xffc00000` (the NaN made by
  `inf - inf` wins), Rust returns `0x7fc00000` (the NaN passed in as `frac`
  wins).
* `MakeNormalVectors([NaN, 1, 0], right, up)` -- `right[1] = -forward[0]` is
  `0xffc00000` while `0 * forward[0]` is `0x7fc00000`, and the two meet in
  `DotProduct(right, forward)`; C keeps `0x7fc00000`, Rust keeps `0xffc00000`.

## Deviation 2 -- `ColorBytes3`'s uninitialised top byte

```c
unsigned ColorBytes3 (float r, float g, float b) {
    unsigned    i;                     /* never initialised */
    ( (byte *)&i )[0] = r * 255;
    ( (byte *)&i )[1] = g * 255;
    ( (byte *)&i )[2] = b * 255;
    return i;                          /* byte 3 is whatever the stack held */
}
```

gcc -O0 returns the 4 bytes of a stack slot of which only 3 were written, so bit
24..31 of the C result is indeterminate -- it changes with the call sequence and
with the compiler. The Rust translation leaves that byte `0`; the tests compare
the 24 defined bits (`color_bytes3`,
`color_bytes3_top_byte_is_indeterminate`) and additionally check that those 24
bits agree with `ColorBytes4`, whose 4th byte *is* written and is compared
strictly.

## Deviation 3 (build profile only) -- NULL dereference signal

`c_src` dereferences all its pointer parameters unchecked, so `NULL` is
undefined behaviour. gcc's code faults with `SIGSEGV`. The Rust library does the
same when built with `-Cdebug-assertions=off` (`cargo test --release`); in the
dev/test profile rustc's `ub_checks` notice the null read before the load in
some functions and turn it into a non-unwinding panic (`SIGABRT`).
`null_pointer_crashes_match` asserts `SIGSEGV` for C in both profiles, exactly
`SIGSEGV` for Rust in release, and `SIGSEGV`-or-`SIGABRT` in the dev profile.

## Deviation 4 -- `AngleVectors` is not reentrant in C

```c
void AngleVectors( const vec3_t angles, vec3_t forward, vec3_t right, vec3_t up) {
    float       angle;
    static float        sr, sp, sy, cr, cp, cy;
    // static to help MS compiler fp bugs
```

All six are always assigned before they are read, so for a single call they
behave exactly like locals -- which is what `src/q_math.rs` uses. They are
*shared process state* though, so two threads calling `AngleVectors` (directly
or through `AnglesToAxis`) race and read each other's sines and cosines. That is
a data race, i.e. undefined behaviour, and it is observable: `cargo test` runs
the tests of one binary on parallel threads, and the C library then returns
values computed from another test's angles while the Rust library (locals)
returns the correct ones.

The translation deliberately keeps locals -- reproducing a data race would make
the Rust library nondeterministic without making it more faithful for any
defined execution. Every test that reaches `AngleVectors` takes
`harness::angle_vectors_guard()` so that the C side is only ever entered by one
thread at a time.

## Non-terminating C functions (cannot be compared, replicated instead)

* `AngleSubtract(a1, a2)` / `AnglesSubtract` -- `while (a > 180) a -= 360;`
  never ends once half an ulp of `a` exceeds 360 (from `2^33 + 1 ulp` upwards,
  and for `±inf`).
* `Q_log2(val)` with `val < 0` -- `val >>= 1` is an arithmetic shift, so `val`
  sticks at `-1`.

Both are translated as the same loops, so the Rust code hangs on exactly the
same inputs. The tests assert the *arithmetic* that causes the hang
(`angle_subtract_hangs_doc`, `q_log2_negative_hangs_doc`) and compare every
input for which the C function does terminate.
