# Verification report

Differential verification of `translation/` (Rust) against `c_src/` (C, the
ground truth). Both objects are loaded with `libloading` and are only ever
called through their exported `hsv_to_rgb` symbol, so the `#[no_mangle]
extern "C"` wrapper is under test too — nothing calls the Rust crate directly
(it is `crate-type = ["cdylib"]`, so it cannot even be linked as an rlib).

## How to reproduce

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .   # C .so
cd translation
cargo build --offline && cargo build --release --offline                 # Rust .so
./check_all_features.sh      # every feature combo x dev/release, symbols + all tests
./mutation_check.sh          # proves the suite can actually fail
```

`--offline` is used because the crates.io index is unreachable from this
sandbox; `libloading 0.8.9` is already in the local cargo cache.

## Files

| file | purpose |
|---|---|
| `SYMBOLS.md` | Phase A: `nm -D` surface of both objects, C-source inventory |
| `ERRORS.md` | Phase C: 25-row error/rejection surface, mechanically grepped |
| `CONFIGS.md` | Phase B: 42-row configuration surface (options × input shapes) |
| `tests/common/mod.rs` | harness: dlopen both `.so`s, bit-exact compare, PRNG, buffer-shape runner |
| `tests/phase_b_configs.rs` | 42 tests, one per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 25 tests, one per `ERRORS.md` row (incl. child-process fault parity) |
| `tests/phase_d_parity.rs` | symbol parity + artifact/test cross-checks |
| `check_all_features.sh` | the full sweep (feature combos × profiles) |
| `mutation_check.sh` | 18 mutants; sensitivity proof for the suite |

## Result

* `cargo check` — clean (no errors, no warnings) for every combination.
* Symbol parity — the C `.so` exports exactly one symbol, `hsv_to_rgb`; the Rust
  `.so` exports it with the same name and type. **Symbol diff is empty.** No C
  translation unit was skipped (`CMakeLists.txt` compiles only `src/lib.c`), so
  no module needed to be translated to close a gap, and nothing is stubbed.
* Phase B — all 42 `CONFIGS.md` rows pass.
* Phase C — all 25 `ERRORS.md` rows pass.
* Phase D — 7 parity/cross-check tests pass; 77 tests green under every one of
  the 4 configurations (`<default>` and `--no-default-features`, each × dev and
  release; `Cargo.toml` declares no `[features]`, so those two are the whole
  feature space, and `phase_d_parity::cargo_toml_declares_no_features` fails if
  that ever changes).
* Soak — 600 000 000 additional random `(h, s, v)` triples drawn uniformly from
  the **whole** `f32` bit-pattern space (3 seeds × 200 M, release `.so`): zero
  divergences.
* Robustness — the suite also passes with the Rust `.so` rebuilt with
  `-C target-cpu=native`, where LLVM replaces the `floorf` call with `vroundss`
  and emits 3-operand AVX arithmetic.

## Divergences found and fixed

### 1. NaN payload selection depended on LLVM's register allocation (real bug)

Found by the whole-domain fuzz row (`b34`) after ~5 · 10⁴ triples:

```
h = -2915779300.0 (0xcf2dcb43), s = NaN(0x7f82893c), v = NaN(0xffa3598e)
C    = [0xffa3598e, 0x7fc2893c, 0x7fc2893c]
Rust = [0xffa3598e, 0xffe3598e, 0xffe3598e]
```

When *both* operands of an SSE arithmetic instruction are NaN, x86 returns the
**destination** operand quieted (Intel SDM, "Rules for handling NaNs"), so
`(1-s) * v` and `v * (1-s)` return different payloads even though they are
mathematically identical. gcc's choice is therefore observable in `dest[]`.
Plain Rust `*` leaves the choice to LLVM, and LLVM's choice moved when
unrelated code changed. Fixed by pinning every operand order in `subss` /
`mulss` / `divss`, using the order read off `objdump -d` of the C `.so`:

| C expression | instruction gcc emits | destination operand |
|---|---|---|
| `h / 60.0f` | `divss %xmm1,%xmm0` | `h` |
| `h - i` | `subss %xmm1,%xmm0` | `h` |
| `1 - s` | `subss s,%xmm0` | `1.0` |
| `v * (1 - s)` | `mulss %xmm1,%xmm0` | `(1 - s)` |
| `s * f` | `mulss f,%xmm1` | `s` |
| `1 - s*f` | `subss %xmm1,%xmm0` | `1.0` |
| `v * (1 - s*f)` | `mulss %xmm1,%xmm0` | `(1 - s*f)` |
| `1 - f` | `subss f,%xmm0` | `1.0` |
| `s * (1 - f)` | `mulss s,%xmm1` | `(1 - f)` |
| `1 - s*(1-f)` | `subss %xmm1,%xmm0` | `1.0` |
| `v * (1 - s*(1-f))` | `mulss %xmm1,%xmm0` | `(1 - s*(1-f))` |

This table is **stable across gcc -O0, -O1, -O2, -O3 and -Os** (all five were
disassembled and compared), so it is a property of the reference object rather
than of one optimisation level.

*Caveat, stated honestly:* the order is compiler-dependent, not
language-defined. A clang-built reference (`clang -O0`) picks the opposite
destination for the `* v` multiplications, and the suite then reports
divergences on double-NaN inputs only. The reference here is the object produced
by the documented build (`cmake` → `/usr/bin/cc` → gcc 11.5), and that is what
the translation matches. Non-NaN results are unaffected: for finite/infinite
operands SSE multiplication is commutative bit-for-bit.

### 2. `src[0]` was loaded conditionally (real bug, out-of-contract input)

The C reads `src[0]`, `src[1]`, `src[2]` before testing `s == 0`. LLVM sinks the
`src[0]` load into the `s != 0` branch, so with `src[0]` on an unmapped page and
`s == 0` the C faulted and the Rust returned normally. Fixed with `movss` inline
asm (`load_f32`), which is unconditional, ordered, misalignment-tolerant and
non-removable. `ptr::read_volatile` was rejected: its debug-mode alignment
precondition turns the misaligned-pointer case (which the C accepts) into a
`SIGABRT`. Covered by `err22_unconditional_h_load_faults`.

### 3. Store granularity/order (hardening)

`dest[0..2]` are three separate ordered `movss` stores in the C, so a fault on
`dest[2]` leaves `dest[0]`/`dest[1]` committed. `store_f32` pins that, and
`err23_partial_store_before_fault` checks the committed prefix through a
file-backed shared mapping after the child process dies of `SIGSEGV`.

## Behaviour that looks wrong but is faithful

Reproduced deliberately, not "fixed":

* no hue wrap-around: `h = 400` is *not* folded into `[0, 360)`; it selects
  `default:` (`i == 6`), so `hsv_to_rgb` is not periodic in `h`;
* `s`, `v` are never clamped: `s > 1` makes `p` negative, `s < 0` makes it
  exceed `v`, and both are returned as-is;
* `s == -0.0` takes the "grey" short-circuit (`-0.0 == 0` is true);
* a NaN `s` does *not*: it flows into the arithmetic;
* `(int)floorf(h/60)` on NaN/overflow is UB in C but `INT_MIN` in the object, so
  NaN and huge hues land in `default:` rather than `case 0`;
* `default:` is reached by every negative `i` because gcc bound-checks the
  switch with an unsigned `ja`;
* null pointers are dereferenced without a check.

## Test-sensitivity evidence (`mutation_check.sh`)

18 mutants, all predictions held:

| # | mutant | outcome |
|---|--------|---------|
| 1 | plain (sinkable) loads | killed by `err22` |
| 2 | `p`: multiply operands swapped | killed by `b41` (+6 more) |
| 3 | `q`: `s*f` operands swapped | killed by `b41` (+4) |
| 4 | `q`: `*v` operands swapped | killed by `b41` (+10) |
| 5 | `t`: `(1-f)*s` operands swapped | **survives — provably equivalent**: both operands can only be NaN when `f` is, which needs `h` NaN, which forces `i == INT_MIN`, i.e. `default:`, which never reads `t` |
| 6 | `t`: `*v` operands swapped | killed by `b41` (+3) |
| 7 | saturating `as` cast | killed by `b23` (+13) |
| 8 | cast bound off by one ULP at `2^31` | killed by `b23` (+1) |
| 9 | reversed store order | killed by `err23` |
| 10 | `switch` arms 3/4 swapped | killed by `b13` (+24) |
| 11 | `default:` returns `(v,q,p)` | killed by `b15` (+40) |
| 12 | `-0.0` no longer short-circuits | killed by `err02` (+5) |
| 13 | short-circuit widened to subnormal `s` | killed by `b27` (+15) |
| 14 | SNaN not quieted on propagation | killed by `err04` (+22) |
| 15 | `trunc` instead of `floor` | killed by `b17` (+21) |
| 16 | `60.0` perturbed by one ULP | killed by `b34` (+40) |
| 17 | division done in `f64` | **survives — provably equivalent** (53 > 2·24+2 bits ⇒ no double rounding) |
| 18 | `i32→f32` routed through `f64` | **survives — provably equivalent** (`i32→f64` is exact) |

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc
      symbols in the Rust `.so`.
- [x] Phase B: every one of the 42 `CONFIGS.md` rows passes across randomized
      inputs (plus a 600 M-triple soak).
- [x] Phase C: every one of the 25 `ERRORS.md` rows has a passing error-path
      differential test, including null pointers, guard pages, misalignment,
      one-past-range values and out-of-domain `switch` selectors.
- [x] All of the above hold under every feature combination (both of them) and
      both build profiles.
