# Verification report

Differential verification of the Rust translation in `src/lib.rs` against the C
ground truth in `c_src/`. **The C is always correct**; every divergence found was
fixed on the Rust side, and no file under `c_src/` was modified.

Reproduce everything with:

```sh
./verify.sh           # Phases A-D, every feature combination, dev + release
./mutation_check.sh   # proves the suite is sensitive, not vacuous
```

## Phase A — the surface

| artifact | content |
|----------|---------|
| `SYMBOLS.md` | `nm -D` surface. 1 exported C symbol (`tfm`), 1 exported Rust symbol (`tfm`), symbol diff **empty**. |
| `ERRORS.md` | 28 rows (29 counting 21a/21b), one per distinct rejection the C performs. |
| `CONFIGS.md` | 34 rows, one per valid configuration the C treats differently. |

Build-time configuration surface:

* `Cargo.toml` declares `[features] default = []` and no other feature; there are
  no `#[cfg(feature = ...)]` sites in `src/`.
* `c_src/CMakeLists.txt` declares no `option()`, no
  `target_compile_definitions`, and compiles a single translation unit
  (`src/lib.c`); that file contains no `#if` / `#ifdef`.

So the complete set of valid combinations is **`--no-default-features`** and the
equivalent **default** build. `verify.sh` enumerates the feature power set
mechanically from `Cargo.toml` (so it stays correct if a feature is ever added)
and runs every one in both the `dev` and the `release` profile.

## Phase B — valid-path differential tests

`tests/phase_b_configs.rs`: 34 tests, one per `CONFIGS.md` row, each driving
`tfm` through the `.so` export with hundreds to tens of thousands of randomized
inputs from the fixed seed `0x2b7e151628aed2a6`, compared **bit-for-bit** on the
raw `u32` of every output element plus guard words on both sides of `dest`.

`tfm` is both the only and the lowest-level public entry point, so every test
calls it directly; there is no convenience wrapper to hide behind.

Rows whose condition is not reachable through the public API (row 8, and the
`4*dxy*dxy` half of row 12) use `assert_unreachable()`, which asserts 0 hits over
the whole search *and* differentially checks that entire search space, rather
than passing vacuously.

## Phase C — error-path differential tests

`tests/phase_c_errors.rs`: 28 tests, one per `ERRORS.md` row.

`tfm` returns `void` and defines no error code, so each test asserts the
strongest observable form of the rejection:

* the loop guard (`count <= 0`, incl. `INT_MIN`) — `dest` provably untouched,
  asserted on both implementations, including with `NULL` `dest`/`src`;
* the branch guard `src[0] < src[1]` — the exact arm taken, for every reason the
  guard can be false (greater, equal, NaN in either or both operands, `-0.0`
  vs `+0.0`);
* the inlined range check `(0 > sqd) ? 0 : sqd` — clamped vs not, at
  `+0.0`, `0x00000001`, `0x80000001`, NaN, and one step past the range;
* the IEEE-754 "errors" — overflow to `±inf`, `inf - inf`, `0 * inf`,
  `inf + (-inf)` — asserted to produce the **exact** bit pattern the C
  produces, which on x86 is the *real indefinite* QNaN `0xffc00000` with the
  sign bit **set**, not `0x7fc00000`;
* signaling-NaN quieting and the SSE destination-operand payload rule;
* aliasing (`dest == src`, `dest` ahead of `src`, `dest` behind `src`).

Generic boundaries required by the task and not otherwise in the table: null
pointers (rows 4-6), zero and negative/extreme lengths (rows 1-3, 28, plus a
dense sweep of every `count` in `-512..=0` and `1..=300`), and values one step
past a documented range (rows 17-18, 28). **Out-of-range enum values: the API
declares no `enum` and no flag parameter** — the only non-pointer parameter is
`int count`, so its full 32-bit input space is what rows 1-3 and 28 cover.

## Phase D — symbol parity, feature combos, completion gate

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort  ->  tfm
$ nm -D --defined-only target/release/libtfm_lib.so      | awk '{print $3}' | sort  ->  tfm
$ comm -23 c.syms r.syms   ->  (empty)
```

`verify.sh` re-runs this diff for every feature combination and both profiles,
and additionally checks that the Rust `.so` has no non-libc undefined symbols.
The C `.so`'s one library import, `sqrtf@GLIBC_2.2.5`, is implemented inline by
`fsqrt()` (`f32::sqrt` lowers to `sqrtss`, the same instruction glibc uses).

Result:

```
=== Phase B/C/D — configuration: --no-default-features ===
  cargo check              ok
  cargo build (dev)        ok
  symbol parity (dev)      ok (1 C symbol(s), 0 missing)
  no non-libc undefined    ok (dev)
  differential tests (dev) ok (74 test(s) passed)
  cargo build (release)    ok
  symbol parity (release)  ok (1 C symbol(s), 0 missing)
  no non-libc undefined    ok (release)
  differential tests (release) ok (74 test(s) passed)
=== Phase B/C/D — configuration: default features (implicit) ===
  ... identical ...
=== SUMMARY ===
  ALL PHASES PASSED for every feature combination.
```

## Suite sensitivity (`./mutation_check.sh`)

Passing tests only mean something if the tests can fail. 29 plausible
mis-translations are injected into `src/lib.rs`, rebuilt, and re-run:

* **21 behavioural mutants are all caught** — including the commuted final
  `+ 4*dxy*dxy` addition, `f32::max` instead of the inlined ternary,
  `0x7fc00000` instead of the x86 indefinite QNaN, source-operand instead of
  destination-operand NaN propagation, missing NaN quieting, `<=` instead of
  `<`, swapped `dest` slots, swapped arm bindings, unsigned `count`, wrong
  strides, writing `dest` before `src` is fully read, and each wrong constant.
* **8 mutants are declared and confirmed *equivalent*** — provably
  indistinguishable through the public API. Each one's justification is itself
  asserted against the C `.so` in `tests/nan_masking.rs`:
  * commuting `2.0f*dx2*dy2`, `dy2 + dx2` or `0.5f * (...)`, and using `mulss`
    against `2.0f` instead of `addss dx2,dx2` — these differ only when *both*
    SSE operands are NaN, which requires `src[0]` and `src[1]` to both be NaN;
    the guard is then unordered, the `else` arm runs, and
    `dest = (src[2], src[1] | 0x00400000)` regardless of `lambda`
    (asserted over 187 264 cases). On the `if` arm neither `dx2` nor `dy2` can
    be NaN at all.
  * `fsqrt` not quieting its NaN operand — the result is consumed only by
    `fadd(sum, root)`, which re-applies the quiet bit; `quiet` is idempotent.
  * `sqrtf` of a negative returning `+qNaN` — dead code, the clamp makes a
    negative argument unreachable.
  * the clamp constant becoming `-0.0f` — `sqd < -0.0` is the same predicate as
    `sqd < +0.0`, and the sign could only survive `(dy2 + dx2) + root` if
    `dy2 + dx2 == -0.0`, which forces `dy2 == dx2 == -0.0` and hence a
    non-negative `sqd` (clamp not taken).
  * dropping `flt`'s redundant NaN guards — Rust's `f32 <` is already ordered.

## Independent cross-check

A standalone **C** driver (`dlopen` both `.so`s, no Rust test harness involved)
compared 40 000 000 output elements from uniformly random 32-bit inputs:

```
C(-O0 cmake reference) vs Rust release cdylib: differing 0 / 40000000
C(-O0 cmake reference) vs Rust debug   cdylib: differing 0 / 40000000
```

## Note on the reference build's optimization level

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and adds no optimization
flags, so the reference `.so` produced by the documented build command

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

is an **unoptimized** (`-O0`) build. That is what the Rust matches bit-for-bit.

The same C source built at `-O1`/`-O2`/`-O3`/`-Os` differs from `-O0` **only in
NaN payloads**, never in a numeric value — measured with the same independent C
driver over 8 000 000 output elements per level:

| comparison | differing elements | of which both-NaN | non-NaN value differences |
|------------|--------------------|-------------------|---------------------------|
| `-O0` vs `-O1` | 944 (0.024 %) | 944 | **0** |
| `-O0` vs `-O2` | 944 (0.024 %) | 944 | **0** |
| `-O0` vs `-O3` | 514 (0.013 %) | 514 | **0** |
| `-O0` vs `-Os` | 944 (0.024 %) | 944 | **0** |

The cause is that `fadd`/`fmul` are commutative for values but not for SSE NaN
payloads (a binary op with two NaN operands returns the *destination* operand),
so the optimizer is free to pick either operand order. `src/lib.rs` pins the
order the `-O0` reference emits, which is plain C source order except for the
two spots documented in `step()`. Every reachable *numeric* result is identical
under every `-O` level.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols and **0**
      non-libc undefined symbols in the Rust `.so`.
- [x] Phase B: **all 34** `CONFIGS.md` rows pass across randomized inputs.
- [x] Phase C: **all 28** `ERRORS.md` rows have a passing error-path
      differential test.
- [x] All of the above hold under **every** feature combination
      (`--no-default-features` and default) and in **both** profiles
      (`dev` and `release`, the latter being the shipping profile with
      `panic = "abort"`).
