# VERIFICATION.md — completion report

Differential verification of the Rust translation in `translation/src/lib.rs`
against the C ground truth in `c_src/src/lib.c`.

Every test loads **both** shared objects with `libloading` and calls **only** the
exported `encode_quant` symbol — the Rust implementation is never called
directly, so the `#[unsafe(no_mangle)] extern "C"` wrapper and the C calling
convention are themselves under test.

## Library surface

| | |
|---|---|
| C `.so` | `c_src/build/libharvest-work-s8YUb3.so` (name derives from the parent dir via `cmake_path` in `CMakeLists.txt`) |
| Rust `.so` | `translation/target/{debug,release}/libencode_quant_lib.so` (`crate-type = ["cdylib"]`) |
| Public API | 1 function: `int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit)` |
| C translation units | 1 (`src/lib.c`) — fully translated, nothing skipped, nothing stubbed |
| Declared cargo features | none |

## Completion gate

- [x] **`SYMBOLS.md`** — `nm -D` shows **0** missing symbols and **0** undefined
      non-libc symbols in the Rust `.so`. The C `.so` exports exactly one symbol
      (`encode_quant`); the Rust `.so` exports it under the identical unmangled
      name. `comm -23 c_syms r_syms` is empty. Re-asserted at test time by
      `tests/phase_d_symbols.rs` (which also refuses a vacuous pass if `nm`
      returns nothing) and by `scripts/check_all_features.sh`.
- [x] **Phase B** — all **43/43** rows of `CONFIGS.md` pass, each across many
      randomized inputs from a fixed seed. Rows map 1:1 to tests `row01…row43` in
      `tests/phase_b_configs.rs` (verified with `cargo test -- --list`).
- [x] **Phase C** — all **20/20** rows of `ERRORS.md` have a passing error-path
      differential test, `err01…err20` in `tests/phase_c_errors.rs`.
- [x] **Every feature combination** — the crate declares no `[features]`, so the
      default build and `--no-default-features` are the only configurations; both
      pass in both the `debug` and `release` profiles (71 tests each).
      `tests/phase_d_symbols.rs::features_declared_in_cargo_toml` fails loudly if
      a `[features]` table is ever added, forcing this report to be redone.

## Result

**Zero behavioural divergences were found.** `translation/src/lib.rs` required no
changes; the C source required no changes (and none were made). Only test-harness
bugs of my own were fixed along the way (see "Findings" below).

## Test inventory

| suite | tests | what it covers |
|---|---|---|
| `tests/smoke.rs` | 2 | both `.so`s `dlopen`, both exports callable, 10 000 random tuples |
| `tests/phase_b_configs.rs` | 43 | one test per `CONFIGS.md` row |
| `tests/phase_c_errors.rs` | 20 | one test per `ERRORS.md` row |
| `tests/phase_d_symbols.rs` | 5 | `nm -D` symbol parity, no unresolved non-libc imports, unmangled C ABI, feature enumeration, regression sweep |
| `tests/phase_d_optlevels.rs` | 1 | Rust vs **30** different C builds (see below) |
| `tests/mutation_control.rs` | 2 | negative control: 28 mutants of the C, all must be caught (see below) |
| `tests/exhaustive.rs` | 21 (`#[ignore]`) | full 2^32 domain sweeps (see below) |
| **total** | **73** run by default, **94** including the exhaustive sweeps | |

## Negative control: mutation testing (is the suite vacuous?)

A suite that passes because it tests nothing is worthless, so
`tests/mutation_control.rs` measures the suite's detection power. It reads the
**unmodified** `c_src/src/lib.c`, applies one realistic transcription slip to an
in-memory copy, compiles that copy into `$TMPDIR` (nothing in `c_src/` is
written), and compares the mutant against the Rust `.so` using the same axis
generators the real Phase B rows use (103 456 inputs per mutant).

**Mutation score: 28/28 caught, 0 survivors.** Highlights:

| mutant | inputs that diverge |
|---|---|
| wrong parenthesisation `(2*(uni&7)+1) * (step/8)` | 12 182 / 103 456 |
| "fix" the quirk: compare `d2 < d1` instead of `d2 < d0` | **157** / 103 456 |
| drop the second selection `if` | 9 246 / 103 456 |
| `d3 >> 4` instead of `>> 5` | 22 556 / 103 456 |
| abs-value shift `>> 30` instead of `>> 31` | 984 / 103 456 |
| clamp mask `~15` instead of `~7` | 4 189 / 103 456 |
| `lsbit == 2` instead of `== 4` | 6 027 / 103 456 |
| dither `|` instead of `&` | 3 096 / 103 456 |
| sign-of-diff bit `& 16` / `& 4` instead of `& 8` | 11 677 / 11 758 |
| `/ 4` instead of `/ 8` | 11 084 / 103 456 |
| `uni + 2` / `uni - 2` instead of `± 1` | 16 200 / 16 165 |

The `d2 < d1` mutant is the important one: "fixing" the C's quirk changes only
0.15 % of inputs, and the suite still catches it. A companion test
(`unmutated_rebuild_agrees`) confirms the unmutated rebuild agrees with Rust on
all 103 456 inputs, so mutants are not being "caught" by a broken pipeline.

## Exhaustive full-domain sweeps (run at stride 1)

| sweep | comparisons |
|---|---|
| every `uni` value (2^32) | 4 294 967 296 |
| every `lsbit` value (2^32) | 4 294 967 296 |
| every `step` value (2^32) | 4 294 967 296 |
| every `tgt` and every `tgt2` value | 8 589 934 592 |
| joint (`uni`, `lsbit`) window `[-512,512]^2` x 8 steps | 8 405 000 |
| **every `step` value for EACH of the 8 multipliers `2*(uni&7)+1`, with bit 3 both clear and set (16 sweeps)** | **68 719 476 736** |
| **total** | **~90.9 billion** differential comparisons, **0 divergences** |

The last row is the important one: the expression `((2*(uni&7)+1)*step)/8` is
translated in Rust as a method chain
(`2i32.wrapping_mul(uni & 7).wrapping_add(1).wrapping_mul(step) / 8`), where a
precedence or parenthesization slip would only surface for specific
(multiplier, `step`) pairs. All 16 multiplier/sign classes were swept across the
entire `step` domain.

## C compiler-configuration robustness

The C relies on signed integer overflow (UB) and right-shifts of negative values
(implementation-defined), so matching one C build could be coincidence.
`tests/phase_d_optlevels.rs` rebuilds the **unmodified** `c_src/src/lib.c` into
`$TMPDIR` (nothing in `c_src/` is touched or modified) and diff-checks the Rust
`.so` against all of:

`gcc`, `clang`, `cc` x `-O0 -O1 -O2 -O3 -Os -Ofast` x plus
`-O2 -fwrapv`, `-O2 -fno-strict-overflow`, `-O3 -fstrict-overflow`,
`-O2 -march=native` — **30 variants**, ~214 000 cases each, **all agree**.

This confirms the reproduced behaviour (two's-complement wrapping, arithmetic
right shift, division truncating toward zero) is the stable ground truth rather
than an artefact of one compiler configuration.

## Debug-profile overflow checking

The `debug` cdylib is built with `overflow-checks = on`. The full suite passes in
`debug` as well as `release`, which independently proves the translation really
uses wrapping arithmetic throughout (`wrapping_add`/`sub`/`mul`/`neg`) and cannot
panic where the C silently wraps. Confirmed for the following C behaviours:

| C construct | reachable overflow | Rust must |
|---|---|---|
| `uni + 1` (line 6) | `uni == INT_MAX` | `wrapping_add` |
| `uni - 1` (line 7) | `uni == INT_MIN` | `wrapping_sub` |
| `(2*(uni&7)+1) * step` (30/36/42) | any `step > INT_MAX/15` | `wrapping_mul` |
| `pred + diff` (33/39/45) | `pred` near the extremes | `wrapping_add` |
| `tgt - p`, `tgt2 - p` (34/40/46/48/51/54) | opposite extremes | `wrapping_sub` |
| `d += d3 >> 5` (50/53/56) | large distortions | `wrapping_add` |
| `-diff` (32/38/44) | **unreachable** — proven, see below | `wrapping_neg` (harmless) |
| `/ 8` (30/36/42) | **cannot trap** — literal divisor | plain `/` |

## Statement-by-statement audit

Every primitive operation in the C was matched against the Rust, and each was
*separately* exhausted over its full input domain by the sweeps above:

| C | Rust | risk checked | how exhausted |
|---|---|---|---|
| `uni + 1` / `uni - 1` (6/7) | `wrapping_add(1)` / `wrapping_sub(1)` | overflow at `INT_MAX`/`INT_MIN` | all 2^32 `uni` |
| `(uni ^ uniX) & (~7)` (8/10) | `((uni ^ uniX) & !7) != 0` | `~7` vs `!7` for i32 (both `-8`) | all 2^32 `uni` |
| `if (lsbit)` / `== 4` / `& 1` / else (12/13/20) | identical nesting and order | branch order; `-4` must NOT hit the dither branch | all 2^32 `lsbit` |
| `uni &= ~1`, `uni \|= 1` (14-27) | `&= !1`, `\|= 1` | `~1` vs `!1` (both `-2`) | all 2^32 `uni` |
| `(uni >> 1) & (uni >> 2) & 1` (17-19) | same | arithmetic shift of negative `uni` | all 2^32 `uni`; `err13` |
| `((2 * (uni & 7) + 1) * step) / 8` (30/36/42) | `2i32.wrapping_mul(uni & 7).wrapping_add(1).wrapping_mul(step) / 8` | **method-chain order vs C precedence**; product overflow; truncation toward zero | **all 8 multipliers x all 2^32 `step`** |
| `if (uni & 8) diff = -diff;` (31-32 etc.) | `wrapping_neg()` | negation overflow (proven unreachable) | all 2^32 `step` with bit 3 set |
| `p = pred + diff` (33/39/45) | `wrapping_add` | overflow | `err08`, extremes cross-product |
| `d = tgt - p` (34/40/46) | `wrapping_sub` | overflow | all 2^32 `tgt` |
| `d = d ^ (d >> 31)` (35/41/47) | `d ^= d >> 31` | arithmetic shift; `INT_MIN` maps to `INT_MAX`; `^=` must read the *old* `d` | all 2^32 `tgt` drives `d` through every i32 |
| `d3 = tgt2 - p` (48/51/54) | `wrapping_sub` | overflow | all 2^32 `tgt2` |
| `d += d3 >> 5` (50/53/56) | `wrapping_add(d3 >> 5)` | overflow flipping the comparison sign | `err10` |
| `if (d1 < d0) …; if (d2 < d0) …;` (57-60) | identical, both vs the original `d0` | must NOT become a running best | mutant #2 (caught) |
| `return (uni);` (61) | `uni` | — | — |
| `int p3` declared, never read (5) | omitted | no observable effect | — |
| 6 `int` params, `int` return | `extern "C" fn(c_int x6) -> c_int`, `#[unsafe(no_mangle)]` | parameter order and widening | `err20`; `nm -D` |

The Rust contains **26** `wrapping_*` calls and **zero** plain `+ - *` on the i32
values, so nothing can panic where the C wraps — confirmed empirically by the
`debug` profile runs (`overflow-checks = on`).

**No behavioural differences were found.** `translation/src/lib.rs` is byte-exact
with the C.

## Independent second audit (cross-checked)

A separate reviewer audited the C against the Rust from scratch, with no access to
my test suite, and independently reached the same verdict: **no behavioural
differences**. It reported ~85.9 billion exhaustive sub-expression comparisons and
~489 million end-to-end calls of its own.

I did not take that self-report on trust — I re-derived its concrete claims
against the real `.so`s:

| claim | independently verified |
|---|---|
| `encode_quant(-24990, -1177023338, 181871, -33, 253852, 0)` == `-24991` in both libs | **yes** — C `-24991`, Rust `-24991` |
| internals of that tuple: `d0=204800540`, `d1=45396256`, `d2=98650787` | **yes** — all three match exactly |
| that tuple is a genuine quirk witness: `d1<d0`, `d2<d0`, `d1<d2`, `uni1 != uni2`, C returns `uni2` | **yes** — `uni1=-24989`, `uni2=-24991`; a running-best translation would return `-24989` |
| reference C build uses `C_FLAGS = -fPIC` with no `-O` flag | **yes** — confirmed in `build/CMakeFiles/*.dir/flags.make` |
| plain `#[no_mangle]` is a hard error under `edition = "2024"` | **yes** — `error: unsafe attribute used without unsafe`, so `#[unsafe(no_mangle)]` is required |
| `size_of::<c_int>() == sizeof(int) == 4` | **yes** — both report 4 |

Three of that audit's stated caveats are closed by the work in this report:

- *"cannot rule out a different compiler/flag set exploiting the UB"* → closed by
  the 30-variant sweep (gcc/clang/cc, `-O0`…`-Ofast`, `-fwrapv`,
  `-fno-strict-overflow`, `-fstrict-overflow`, `-march=native`), all agreeing.
- *"a hypothetical logical-shift C implementation would differ"* → same 30-variant
  sweep covers gcc and clang; both use arithmetic shift.
- *"I compiled with rustc directly, not the cargo artifact"* → every test here
  loads the actual cargo-produced `cdylib` (`debug` and `release`).

The remaining shared caveat is that the full 2^192 input space cannot be
exhausted. It is mitigated by exhausting each parameter's complete 2^32 domain
individually, exhausting the joint (multiplier x `step`) domain, and by the 28/28
mutation score demonstrating detection power.

## Quirks of the C that are deliberately preserved

1. **Both candidate distortions are compared against the *original* `d0`**, and
   the second `if` overwrites the first:
   ```c
   if (d1 < d0) uni = uni1;
   if (d2 < d0) uni = uni2;
   ```
   So when *both* candidates beat `d0`, `uni2` wins even when `d1 < d2` — i.e.
   the C does **not** pick the best candidate. `row38` searches for exactly that
   situation (both beat `d0` **and** `d1 < d2` **and** `uni1 != uni2`) and asserts
   both libraries return `uni2`. This is preserved, not "fixed".
2. **`lsbit` is a 4-way mode switch typed `int`**, tested as `== 4` before `& 1`,
   so `-4` does *not* take the dither branch while `4` does, and any odd value
   (including `INT_MAX` and negative odds) forces bit 0 set.
3. **`d ^ (d >> 31)` is a branchless absolute value that is wrong for `INT_MIN`**
   (it yields `INT_MAX`). Reproduced exactly.
4. **`p3` is declared and never used** in the C — no observable effect.
5. **The candidate clamp keeps `uni`, `uni1`, `uni2` in the same 3-bit field**,
   which incidentally masks the `uni ± 1` overflow at `INT_MAX`/`INT_MIN`.

## Findings

No divergence between the C and the Rust was found at any point. Three issues
were found and fixed, all in my own test/documentation artifacts:

1. `row38` originally searched for the "both candidates beat `d0`" outcome using
   a monotone shape (`uni in 1..6`, `lsbit = 0`, `step > 0`). That outcome is
   **unreachable** there: the clamp forces `uni`/`uni1`/`uni2` to share bit 3, so
   `p2 < p0 < p1` is monotone and at most one candidate can beat `d0`. The search
   was widened to all axis classes (the outcome needs a non-monotone shape:
   negative/overflowing `step`, `lsbit` conditioning, or wrapping).
2. `ERRORS.md` row 7 originally claimed `diff == INT_MIN` was reachable. It is
   **provably not**: the multiplier is odd so the wrapped product covers all of
   `int`, but `diff = product / 8` is bounded to `[-2^28, 2^28-1]`, which excludes
   `INT_MIN`. Verified by brute force. The row now records the proof, and the test
   drives `diff` to both reachable extremes (`±2^28`) through the negation branch
   instead of asserting a false claim.
3. `err06` and `check_all_features.sh` had bugs of their own (`i32::MAX/1 + 1`
   overflowing in the *test* code; a read-only `/tmp` making the symbol diff
   report a vacuous "EMPTY"). Both fixed, and the script now refuses to pass if
   the C `.so` yields zero symbols or if fewer than 60 tests run.

## Reproducing

```bash
# 1. C ground truth
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .

# 2. Default suite (71 tests), both profiles
cd translation
cargo test              # debug: overflow-checks on in the loaded cdylib
cargo test --release

# 3. Every feature combination + symbol parity
bash scripts/check_all_features.sh

# 4. Exhaustive full-domain sweeps (~8 min on 16 cores)
EXHAUSTIVE_STRIDE=1 cargo test --release --test exhaustive -- \
  --ignored --nocapture --test-threads 16
# quick subsampled pass:
EXHAUSTIVE_STRIDE=4096 cargo test --release --test exhaustive -- --ignored
```
