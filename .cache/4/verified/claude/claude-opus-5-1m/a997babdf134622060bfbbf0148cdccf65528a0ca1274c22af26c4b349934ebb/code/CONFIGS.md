# CONFIGS.md — Phase A: configuration surface table (valid inputs)

## Build-time configuration axes

* `Cargo.toml` has **no `[features]` section**, so the *only* feature
  combination is the default (empty) one. `cargo check/test --no-default-features`
  and the plain invocation are the same configuration; both are still exercised
  by `run_all_configs.sh`.
* `c_src/CMakeLists.txt` has **no** `option()`, no `add_definitions`, no
  `#ifdef`/`#if` anywhere in `c_src/src/main.c` (`grep -c '#if' → 0`), and a
  single target. So there is exactly **one** C build configuration too.
* Cargo profiles are not a behavioural axis, but because
  `[profile.release] panic = "abort"` differs from `dev`, the harness script runs
  the whole differential suite against **both** the debug and the release Rust
  binary.

## Runtime configuration axes actually branched on by the C

The program takes no CLI options and no environment variables (`int main()` with
no parameters, no `getenv`). Every runtime axis is therefore a property of the
**input data** and of the **stdio environment**:

| axis | values the C distinguishes | source line |
|---|---|---|
| `A` loop guard | `x>0` / `y>0` / both / neither | `while (x > 0 \|\| y > 0)` |
| `B` special-case jump | `x==1 && y==4` (→ `goto label2`, skipping the `x--` block) vs anything else | `if (x == 1 && y == 4)` |
| `C` `x` decrement gate | `x>0` vs `x<=0` | `if (x > 0)` at `label1` |
| `D` `continue` gate | `y==0` vs `y!=0` | `if (y == 0) continue;` at `label2` |
| `E` back-edge | `x<3` (→ `goto label1`, inner loop) vs `x>=3` (fall out to the guard) | `if (x < 3)` |
| `F` value magnitude | small / `INT_MAX` / `INT_MIN` / past `LONG_MAX` / past `LONG_MIN` / past `INT` range | `scanf("%d")` conversion |
| `G` sign | `+` / `-` / none, on either operand | `scanf("%d")` conversion |
| `H` token layout | space / multiple spaces / `\n` / `\t` / `\v` / `\f` / `\r` / leading / trailing / no separator (`5-6`) / >2 tokens / 1 token / 0 tokens | `"%d %d"` + `%d`'s whitespace skip |
| `I` digit shape | 1 digit / many digits / leading zeros / 1000-digit run | `strtol` accumulation |
| `J` stdout target | pipe (block-buffered) / regular file / `/dev/null` / unwritable / closed / early-closed reader | `printf`→`puts` buffering |
| `K` stdin source | pipe / regular file / `/dev/null` / unbounded stream | `scanf` on `stdin` |
| `L` termination | bounded output vs unbounded (signed-overflow wrap when `x>0 && y<0`) | `y--` |

`x` is partitioned at the values the code compares against — `x<0`, `x==0`,
`x==1` (axis `B`), `x==2`, `x==3` (axis `E` boundary, since `x` is tested
*after* being decremented), `x>3` — and `y` at `y<0`, `y==0`, `y==1`,
`y==3`, `y==4` (axis `B`), `y==5`, `y>5`.

Note a control-flow fact that the rows below rely on: the `x==1 && y==4` test is
only reachable at the **top of the body**, and the body is re-entered only via
the guard, which is reached either by `continue` (only when `y==0`, so `y!=4`) or
after `y--` with `x>=3` (so `x!=1`). Therefore axis `B` can **only** fire on the
first iteration, i.e. only for the literal input pair `(1,4)` — which makes row
C7 mandatory rather than optional.

## Entry points

There is one public entry point — the process itself (`main` → `scanf` → `foo`).
`foo` is the lowest-level unit and is `static`, so it is unreachable except
through `main`; there is no convenience-wrapper/low-level split to test
separately. The rows below therefore drive `foo` through **every** distinct
control-flow shape by choosing the `(x, y)` pairs that reach it, and separately
cover the `scanf` layer (rows C16–C24) and the stdio environment (rows C25–C28).

Every row is checked with **many randomized inputs** drawn from the row's class
(fixed-seed xorshift PRNG in `tests/differential.rs`), not a single hand-picked
value, and asserts byte-identical stdout, stderr and exit status.

| #  | entry point(s) | configuration (options set + input shape) | test | ✓ |
|----|----------------|-------------------------------------------|------|---|
| C1 | `foo` via `main` | `x<=0 && y<=0` — guard false, zero iterations, empty output. Randomized over `x,y ∈ [-4096,0]` incl. `(0,0)` | `c1_guard_false_no_iterations` | [x] |
| C2 | `foo` via `main` | `x>0, y==0` — guard true via first disjunct only; `label1` prints/decrements, `label2` `continue`s every pass; `x` random in `[1,3000]` | `c2_xpos_yzero` | [x] |
| C3 | `foo` via `main` | `x==0, y>0` — guard true via second disjunct; `label1` no-op, back-edge `x<3` spins the inner loop until `y==0`; `y` random in `[1,3000]` | `c3_xzero_ypos` | [x] |
| C4 | `foo` via `main` | `x<0, y>0` — guard via `y` only, `x` stays negative (never decremented), inner loop via `x<3`; `x ∈ [-3000,-1]`, `y ∈ [1,3000]` | `c4_xneg_ypos` | [x] |
| C5 | `foo` via `main` | `1<=x<3` and `y>0` (back-edge `x<3` always taken; `x` and `y` both drain) | `c5_x_below_three_ypos` | [x] |
| C6 | `foo` via `main` | `x==3` exactly with `y>0` — boundary: `x` is decremented to 2 *before* `if (x<3)`, so the back-edge **is** taken on the first pass | `c6_x_equals_three_boundary` | [x] |
| C7 | `foo` via `main` | `x==1 && y==4` exactly — the **only** input that takes `goto label2`, skipping the `x--` block on the first iteration | `c7_goto_label2_special_case` | [x] |
| C8 | `foo` via `main` | near misses of C7: `(1,3)`, `(1,5)`, `(2,4)`, `(0,4)`, `(1,0)`, `(4,4)` — must **not** take the jump | `c8_goto_label2_near_misses` | [x] |
| C9 | `foo` via `main` | `x>3 && y>0` — first pass falls out of the body to the guard (no back-edge) and re-enters, until `x` drops below 3 | `c9_x_above_three_ypos` | [x] |
| C10 | `foo` via `main` | `x>3 && y==0` — repeated guard re-entry with `continue` each pass | `c10_x_above_three_yzero` | [x] |
| C11 | `foo` via `main` | dense randomized sweep of the whole small-value cross product `x ∈ [-6,12] × y ∈ [0,12]` (exhaustive, 19×13 pairs) | `c11_exhaustive_small_grid` | [x] |
| C12 | `foo` via `main` | large bounded workloads: `x,y ∈ [3000,9000]` (tens of thousands of output lines, crosses stdio buffer boundaries many times) | `c12_large_bounded_workloads` | [x] |
| C13 | `foo` via `main` | `x==INT_MAX` with `y==0`, and `y==INT_MAX` with `x==0` submitted as **prefix-compared** unbounded-time runs (output is ~2^31 lines) | `c13_int_max_prefix` | [x] |
| C14 | `foo` via `main` | `x==INT_MIN` / `y==INT_MIN` combinations: `(INT_MIN,0)` and `(INT_MIN,INT_MIN)` are bounded-empty; `(INT_MIN, y>0)` prints `loop`+`y` lines with no `x` line | `c14_int_min_combinations` | [x] |
| C15 | `foo` via `main` | unbounded runs (`x>0 && y<0`, signed-overflow wrap): compared over a 256 KiB output **prefix** | `c15_unbounded_prefix_compare` | [x] |
| C16 | `main`/`scanf` | canonical layout `"<x> <y>"`, randomized values in `[-4096,4096]` (pruned of the unbounded class) | `c16_canonical_layout` | [x] |
| C17 | `main`/`scanf` | separator variants: multiple spaces, `\t`, `\n`, `\v`, `\f`, `\r`, `\r\n`, and mixed runs between the two integers | `c17_separator_variants` | [x] |
| C18 | `main`/`scanf` | leading whitespace before the first integer, and trailing whitespace/newline after the second | `c18_leading_and_trailing_whitespace` | [x] |
| C19 | `main`/`scanf` | explicit signs: `"+x +y"`, `"-x -y"`, `"+x -y"`, `"-x +y"`, and `"+0"`/`"-0"` | `c19_explicit_signs` | [x] |
| C20 | `main`/`scanf` | no separator, sign acts as the delimiter: `"5-6"`, `"5+6"`, `"12-34"` | `c20_sign_as_separator` | [x] |
| C21 | `main`/`scanf` | leading zeros and long digit runs: `"0000005 000006"`, 40-digit and 1000-digit runs, `"0 0"` | `c21_leading_zeros_and_long_runs` | [x] |
| C22 | `main`/`scanf` | more than two tokens (`"5 6 7 8 9"`) and trailing junk after the second int (`"5 6junk"`) | `c22_extra_tokens_ignored` | [x] |
| C23 | `main`/`scanf` | only one token present (`"7"`, `"7 "`, `"7\n"`) → `y` keeps its `0` default | `c23_single_token` | [x] |
| C24 | `main`/`scanf` | boundary/near-boundary magnitudes as valid input on **both** operands: `INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1`, `2147483648`, `4294967296`, `LONG_MAX`, `LONG_MAX+1`, `LONG_MIN`, `LONG_MIN-1` | `c24_magnitude_boundaries_both_operands` | [x] |
| C25 | process/stdio | stdin from a **regular file** vs a **pipe** (same bytes) — must give identical output for both | `c25_stdin_file_vs_pipe` | [x] |
| C26 | process/stdio | stdin from `/dev/null` (immediate EOF) | `c26_stdin_dev_null` | [x] |
| C27 | process/stdio | stdout to a **regular file** vs a **pipe** vs `/dev/null` (block-buffered vs discarded); bytes on disk must be identical to bytes on the pipe | `c27_stdout_file_vs_pipe` | [x] |
| C28 | process/stdio | large stdin payload (64 KiB of leading whitespace, then the two integers) crossing the stdio buffer boundary during the whitespace skip | `c28_large_whitespace_prefix` | [x] |
| C29 | process/env | locale/environment invariance: `LC_ALL`/`LANG`/`LC_NUMERIC`/`LC_CTYPE` set to `C`, `POSIX`, `en_US.UTF-8`, `de_DE.UTF-8`, `tr_TR.UTF-8` — the C never calls `setlocale`, so `%d` stays ASCII-only and no thousands separator is ever accepted | `c29_locale_and_env_invariance` | [x] |

## Results

All 29 rows pass, under **every** configuration from the axes above:

```
$ ./run_all_configs.sh
Feature combinations discovered: 1
  - '<none>'
### cargo check / cargo test   features='<none>' profile=dev      -> 29 + 27 + 4 passed
### cargo check / cargo test   features='<none>' profile=release   -> 29 + 27 + 4 passed
RESULT: all configurations passed
```

Randomization uses a fixed-seed xorshift64\* PRNG (one seed per row), so a failing
row reproduces exactly.

## Comparison strategy for oversized runs

`foo` emits on the order of `x + y` lines, and the `x>0 && y<0` class does not
terminate in practice (see `ERRORS.md` row E15). Rows whose output cannot be
buffered are therefore compared over a fixed-length stdout **prefix** (64–256 KiB)
taken from both artifacts, which is still an exact byte-for-byte comparison over
that window. The harness enforces this: `run_with` caps captured stdout at 64 MiB
and fails with an explicit "must use prefix comparison" message rather than
exhausting memory, so a row can never silently skip its comparison.

One equivalence is *not* reachable in finite test time and is called out
explicitly: distinguishing `y--`'s wrap at `y == INT_MIN` from a hypothetical
saturating decrement would require observing more than 2^32 output lines (~8 GiB),
because both produce the identical unbounded `"y\n"` stream before then. The Rust
uses `wrapping_sub`, which is what the C compiles to at `-O0`; every finite prefix
of the two agrees.
