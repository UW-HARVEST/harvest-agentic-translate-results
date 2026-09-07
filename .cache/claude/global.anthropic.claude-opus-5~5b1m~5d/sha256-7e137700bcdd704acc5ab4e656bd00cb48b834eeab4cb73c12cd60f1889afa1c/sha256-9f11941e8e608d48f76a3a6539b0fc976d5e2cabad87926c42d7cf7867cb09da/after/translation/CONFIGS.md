# CONFIGS.md — configuration-surface table

## Axes the C actually branches on

Derived from `c_src/CMakeLists.txt` + every `#ifdef`/`switch`/token-paste
branch in `c_src/src/mdmacros.h`:

**Build-time axis 1 — `OP`** (`-DOP=${OP}`, CMake default `add`). Selects, by
token pasting, four *different* things at once, so each value is a genuinely
distinct code path:

| `OP` | `OP_FN(OP)` | `STEP_OP` | `INIT_FOR` | `STR(OP)` |
|------|-------------|-----------|------------|-----------|
| `add` | `op_add` (`a+b`)  | `acc += i`       | `0` | `"add"` |
| `sub` | `op_sub` (`a-b`)  | `acc -= i`       | `0` | `"sub"` |
| `mul` | `op_mul` (`a*b`)  | `acc *= (i + 1)` | `1` | `"mul"` |

**Build-time axis 2 — `REPEAT`** (`-DREPEAT=${REPEAT}`, CMake default `5`).
`RUN_LOOP` ⇒ `CHOOSE_REP(REPEAT)` ⇒ `REP<REPEAT>`, and `REP0..REP7` are the only
definitions ⇒ legal values are exactly `0,1,2,3,4,5,6,7`. Note the asymmetry the
C deliberately has: `RUN_LOOP` supports `7`, but `DISPATCH_REP`'s `switch` only
has `case 0..6`, so `accum_<OP>(7)` hits `default:`.

⇒ **24 build configurations** (3 × 8). Cargo mirror: features
`add|sub|mul` × `0|1|2|3|4|5|6|7`. Feature-resolution axis 3 (Rust-only, must
still compile): no OP feature ⇒ `add`; no REPEAT feature ⇒ `5`; conflicting
features ⇒ `mul>sub>add`, highest REPEAT wins. 36 checked combos
(4 OP-choices × 9 REPEAT-choices) all `cargo check` clean.

**Runtime axes**

* Entry point (all 6 exported functions + 2 exported data objects + the binary).
  Lowest-level first: `op_add`/`op_sub`/`op_mul` → `G_OP`/`G_OP_NAME` →
  `helper_ptr` → `helper_call` → `use_generated` → `main`.
* Input *shape* for the `(int,int)` functions: zero / one / negative /
  `INT_MAX` / `INT_MIN` / overflow-producing pair / random.
* Input *shape* for `use_generated(int n)`: in-`switch` (`0..6`, i.e.
  empty/one/many unrolled steps) vs out-of-`switch` (`7`, `>7`, negative,
  `INT_MIN`, `INT_MAX`).
* `G_OP` is *writable* exported data ⇒ mutated vs pristine is a distinct state.

## Table — one row per combination the C treats differently

Every row is exercised with **many randomized inputs** (`SplitMix64`,
fixed seed `0x5DEECE66D`, ≥256 draws/row plus all boundary values), against
**all 24** `OP`×`REPEAT` builds unless the row pins a specific value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `op_add` | any build (result is `OP`/`REPEAT`-independent); random `(a,b)` over full `i32` | [x] |
| 2 | `op_add` | boundary pairs: `(0,0) (0,1) (1,0) (-1,1) (INT_MAX,0) (INT_MAX,1) (INT_MIN,-1) (INT_MAX,INT_MAX) (INT_MIN,INT_MIN)` | [x] |
| 3 | `op_sub` | any build; random `(a,b)` over full `i32` | [x] |
| 4 | `op_sub` | boundary pairs incl. `(INT_MAX,-1) (INT_MIN,1) (0,INT_MIN) (INT_MIN,INT_MIN)` | [x] |
| 5 | `op_mul` | any build; random `(a,b)` over full `i32` | [x] |
| 6 | `op_mul` | boundary pairs incl. `(0,INT_MIN) (1,INT_MIN) (-1,INT_MIN) (INT_MAX,2) (65536,65536) (INT_MIN,INT_MIN)` | [x] |
| 7 | `G_OP` (read) | `OP=add`, all `REPEAT` — pointer must equal the `.so`'s own `op_add`; calling through it matches C | [x] |
| 8 | `G_OP` (read) | `OP=sub`, all `REPEAT` — must equal `op_sub` | [x] |
| 9 | `G_OP` (read) | `OP=mul`, all `REPEAT` — must equal `op_mul` | [x] |
| 10 | `G_OP` (call through) | all 24 builds × random `(a,b)`; compare `C.G_OP(a,b)` vs `Rust.G_OP(a,b)` | [x] |
| 11 | `G_OP` (write, then read back) | all 24 builds; overwrite with the `.so`'s `op_sub`, call through it, restore — writable-data parity | [x] |
| 12 | `G_OP_NAME` | `OP=add` ⇒ pointee bytes `add\0`; all `REPEAT` | [x] |
| 13 | `G_OP_NAME` | `OP=sub` ⇒ `sub\0` | [x] |
| 14 | `G_OP_NAME` | `OP=mul` ⇒ `mul\0` | [x] |
| 15 | `helper_ptr` | all 24 builds × random `(a,b)` + boundary pairs (return value only depends on `OP`) | [x] |
| 16 | `helper_ptr` **after `G_OP` was overwritten** | all 24 builds; C uses `OP_FN(OP)` directly so the overwrite must NOT affect the result — Rust must match | [x] |
| 17 | `helper_call` | `OP=add`, `REPEAT=0..7` (8 rows collapsed): `acc` = Σ`0..REPEAT-1` = `0,0,1,3,6,10,15,21`; random `(a,b)` | [x] |
| 18 | `helper_call` | `OP=sub`, `REPEAT=0..7`: `acc` = `0,0,-1,-3,-6,-10,-15,-21`; random `(a,b)` | [x] |
| 19 | `helper_call` | `OP=mul`, `REPEAT=0..7`: `acc` = `1,1,2,6,24,120,720,5040`; random `(a,b)` | [x] |
| 20 | `helper_call` | all 24 builds × overflow boundary pairs (`r + acc` wrap) | [x] |
| 21 | `use_generated` | all 24 builds × `n = 0` (empty unroll ⇒ `INIT_FOR`) | [x] |
| 22 | `use_generated` | all 24 builds × `n = 1` (one step) | [x] |
| 23 | `use_generated` | all 24 builds × `n = 2..6` (many steps — the full in-`switch` range) | [x] |
| 24 | `use_generated` | all 24 builds × `n = 7` (`RUN_LOOP`-legal but *outside* the `switch` ⇒ `INIT_FOR`) | [x] |
| 25 | `use_generated` | all 24 builds × `n` random over full `i32` (dominated by the `default:` arm) | [x] |
| 26 | `use_generated` | all 24 builds × `n = REPEAT` (exactly what `main` passes) | [x] |
| 27 | composed pipeline | all 24 builds: run `op_fn`, `RUN_LOOP`, `helper_call`, `helper_ptr`, `use_generated(REPEAT)`, `G_OP` in `main`'s order and compare the full 6-tuple + `summary` sum, random `(a,b)` | [x] |
| 28 | `driver` binary | all 24 builds × random `(A,B)` argv pairs — stdout byte-for-byte, exit status | [x] |
| 29 | `driver` binary | all 24 builds × boundary argv (`0 0`, `2147483647 1`, `-2147483648 -1`, `INT_MIN INT_MIN`) | [x] |
| 30 | `driver` binary | all 24 builds × non-numeric / whitespace / trailing-garbage / overflowing argv (see ERRORS.md 4-8) | [x] |
| 31 | `driver` binary | all 24 builds × `argc<3` (0 and 1 operand) — stderr + status `2` | [x] |
| 32 | `driver` binary | all 24 builds × `argc>3` (surplus argv ignored) | [x] |
| 33 | stdout interleaving | all 24 builds: `helper_call`→`helper_ptr`→`use_generated`→`main`'s two lines must appear in this exact order when stdout is a pipe (C is fully buffered, Rust is line buffered) | [x] |

## Verification results

Driver scripts: `scripts/run_all.sh` (the 24 `OP`×`REPEAT` builds) and
`scripts/run_resolution.sh` (the 18 Cargo-feature *sets* that resolve onto one
of those 24).

* `cargo check --release --all-targets --no-default-features --features <combo>`:
  **36/36** combos (4 OP-choices × 9 REPEAT-choices) clean — 0 errors, 0 warnings.
* `scripts/run_all.sh`: **24/24** configurations PASS, 38 tests each
  (**912** assertions-suites total, 0 failures).
* `scripts/run_resolution.sh`: **18/18** feature sets PASS, incl. the empty set,
  OP-only, REPEAT-only, and all-features-on.
* Binary stdout: 240 independent `C` vs `Rust` `driver` invocations across all
  24 builds — byte-identical stdout, stderr and exit status.

### Divergences found and fixed (Rust side only; `c_src/` untouched)

1. **`G_OP` / `G_OP_NAME` were in the wrong ELF section.** Declared as immutable
   Rust `static`s holding relocated pointers, they were emitted into
   `.data.rel.ro`, which the dynamic loader re-protects read-only. The C objects
   live in writable `.data`, so a consumer's store into `G_OP` — legal against
   the C `.so` — **SIGSEGV'd** against the Rust `.so`. Fixed by declaring both
   `static mut` (rows 11, 12-14; `ERRORS.md` 23-24). Confirmed with
   `readelf -SW`: both symbols are now `D` in `.data` in both libraries.
2. **`helper_ptr` read the global `G_OP`.** The C body is
   `int (*fp)(int,int) = OP_FN(OP);` — a token-paste to `op_<OP>`, *not* a load
   of `G_OP`. With `G_OP` overwritten by a caller, C's `helper_ptr` result is
   unchanged while the Rust version followed the new pointer. Fixed to use the
   compiled-in op directly (row 16; `ERRORS.md` 23).

### Test-harness note (not a translation bug)

`dlopen` of the same path within one process returns the same mapping, so all
tests in a binary share one `G_OP` object. `tests/common::g_op_guard()`
serialises the tests that read or write it; without it the mutating rows raced
the readers and produced spurious `pipeline(...)` mismatches.
