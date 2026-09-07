# CONFIGS.md — Phase A: configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from the C
source and the public header, not from what looks important.

## Axis enumeration (what the C actually branches on)

**Runtime options / modes / flags:** *none.* `include/lib.h` declares exactly one
function and one struct. There is no init function, no options struct, no
setter, no global, no `#ifdef` in `src/lib.c`, and no `if`/`switch` anywhere —
`grep -cE 'if|switch|#if' c_src/src/lib.c` is `0`. So the configuration surface
is entirely **input shape**, i.e. the 128-bit `cn_rnd_t` state, plus the **call
sequence length**, because the function is stateful and mutates `*rnd`.

**Cargo features:** `translation/Cargo.toml` has no `[features]` section, so the
only build configurations are `--all-features` ≡ `--no-default-features` ≡
default. Verified in Phase D by looping over them.

**Public entry points (full set, lowest level included):**

| entry point | linkage | reachable from a consumer? |
|---|---|---|
| `next_double(cn_rnd_t *)` | global, in `lib.h`, in `nm -D` | yes — the only ABI entry point |
| `cn_rnd_next(cn_rnd_t *)` | `static`, file-local, absent from `nm -D` and from `lib.h` | **no** — cannot be called across the `.so` boundary in C either. It is exercised *indirectly and completely*: `next_double` calls it exactly once per call and the full 64-bit result is observable — bits 63..12 through the returned mantissa, and bits 11..0 through the resulting `state[1]`, which the caller can read back out of the struct. Rows below assert **both** the return value and the post-call `state[0]`/`state[1]`, so a divergence in the low 12 bits of `cn_rnd_next` cannot hide behind the `>> 12`. |

**Input shapes the code distinguishes** (the shift/mask structure of
`cn_rnd_next` and the `>> 12` / `<< 52` field packing in `next_double`):

* `state[0]` (`x`) and `state[1]` (`y`) independently: zero, one, low-bits-only,
  high-bits-only, `MAX`, random;
* bit positions that the shifts move across word boundaries: `x << 23`,
  `x >> 17`, `y >> 26`;
* mantissa field `value >> 12`: min (`0`) and max (all 52 bits set);
* stream length: 1 call, 2 calls, many calls (state feedback).

## Configuration-surface table

One row per meaningful combination the C treats differently. Every row is driven
through the `.so` exports of **both** libraries, with **many randomized inputs**
per row (fixed seed `0x9E3779B97F4A7C15`, `N` as noted), asserting
byte-for-byte equality of the returned `double` **bits** *and* of the mutated
`state[0]`, `state[1]`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `next_double` | no options exist; `state = {0, 0}` — degenerate fixed point, 1 call | [x] |
| 2 | `next_double` | `state = {0, 0}`, long stream (4096 sequential calls) — confirms the fixed point never escapes in either impl | [x] |
| 3 | `next_double` | `state[0]` random, `state[1] = 0`; N = 4096 | [x] |
| 4 | `next_double` | `state[0] = 0`, `state[1]` random; N = 4096 | [x] |
| 5 | `next_double` | both words uniformly random over all 64 bits; N = 65536, single call each | [x] |
| 6 | `next_double` | both words random, **stream of 64 sequential calls** per seed (state feedback path); N = 512 seeds × 64 calls | [x] |
| 7 | `next_double` | `state = {MAX, MAX}` and the three other all-extreme corners `{0,MAX} {MAX,0} {MAX,MAX}`, 1 call and 256-call streams | [x] |
| 8 | `next_double` | single-bit seeds: exactly one of the 128 state bits set (128 cases), 1 call — covers every bit the `<<23`/`>>17`/`>>26` shifts can shift in or out | [x] |
| 9 | `next_double` | two-bit seeds: every pair of set bits within a word for the shift-critical positions (bits 0,1,16,17,22,23,25,26,51,52,62,63 in each word, cross product) | [x] |
| 10 | `next_double` | low-bits-only seeds: both words drawn from `[0, 2^12)` — makes `value >> 12` frequently `0`, i.e. result exactly `0.0`; N = 4096 | [x] |
| 11 | `next_double` | high-bits-only seeds: both words drawn from `0xFFFF_FFFF_FFFF_F000 | rand` — drives the mantissa toward all-ones, result the largest `double < 1.0`; N = 4096 | [x] |
| 12 | `next_double` | seeds crafted so `x ^ (x<<23)` cancels bits (`x` with alternating `0x5555…`/`0xAAAA…`/`0x0F0F…` patterns × same for `y`); 9 pattern pairs × 64-call streams | [x] |
| 13 | `next_double` | seeds where the two words are equal (`x == y`) and where they are bitwise complements (`x == !y`); N = 2048 each — exercises the `x ^= y ^ (y >> 26)` cancellation | [x] |
| 14 | `next_double` | struct passed at a **non-default alignment / offset inside a larger buffer** (valid pointer, 8-aligned but not 16-aligned) with random seeds; N = 1024 — the ABI shape axis | [x] |
| 15 | `next_double` | interleaved calls against **two independent `cn_rnd_t` objects** (no shared/global state in either impl); N = 1024 seed pairs × 32 interleaved calls | [x] |
| 16 | `next_double` | result-range invariant across a large random stream: every returned value is in `[0.0, 1.0)` in *both* impls, and the C/Rust bit patterns are identical; N = 262144 calls | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no** `add_executable`, and `translation/Cargo.toml` has no `[[bin]]` and no
`src/main.rs`. There is therefore **no driver binary whose stdout could be
compared**; that completion-gate item is vacuous for this project. Verified:
`grep -c add_executable c_src/CMakeLists.txt` → `0`.

## Phase B results

All 16 rows pass. Each row is implemented as `tests/differential.rs::row_NN_*`,
loads both `.so`s with `libloading`, and for every call asserts:

1. the returned `double`'s **raw bit pattern** is identical (no float-tolerance
   comparison), and
2. the **mutated `state[0]` / `state[1]`** are identical after *every* call in
   the stream — this is what makes the un-exported `static cn_rnd_next` fully
   observable, including the low 12 bits that `value >> 12` throws away, and
3. the returned value lies in `[0.0, 1.0)`.

Every row is compared against **both** the debug and the release Rust `.so`
(the debug build has overflow checks on, which is where a missing
`wrapping_add` on `x + y` would surface).

Total differential calls executed by the suite: > 1.1 million per profile.

Independent cross-check outside the Rust harness: a ctypes driver ran 300,400
calls (3,004 seeds × 100 sequential calls, including `{0,0}`, `{MAX,MAX}`,
`{0,1}`, `{1,0}` and 3,000 random seeds), packing `(value, state[0], state[1])`
for each call into a 7,209,600-byte stream. The C stream and both Rust streams
are **byte-identical**.

## Phase D results

* Symbol diff C `.so` vs Rust `.so`: **empty**.
* Feature combinations: `Cargo.toml` declares no `[features]`, so
  `run_all_features.sh` enumerated and ran `--all-features`,
  `--no-default-features`, and default features — each in both the dev and the
  release profile (6 runs). All 6: `26 passed; 0 failed; 1 ignored` (the one
  ignored test is the deliberate out-of-process worker for the NULL row).
* Binary/driver stdout comparison: vacuous — neither project builds an
  executable (verified: `grep -c add_executable c_src/CMakeLists.txt` → 0, and
  no `[[bin]]` / `src/main.rs` in the crate).
