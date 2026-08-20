# CONFIGS.md — Configuration-surface table (valid inputs)

Mechanically derived from the branches the C code actually takes.

## Axes the C source distinguishes

There are no compile-time options (no `[features]`, no `#ifdef`s besides the
include guard) and no runtime option/flag setters — the library has no config
struct, no mode enum, no init function. The axes are therefore the *state* and
*input shapes* that `c_src/src/staticalias.c` branches on:

| axis | values the C code distinguishes | where |
|------|--------------------------------|-------|
| **A. entry point** | `static_alias` (lowest level, exercised directly) · `driver` (the convenience wrapper that loops over `static_alias`) | header lines 27–28 |
| **B. branch in `static_alias`** | `*outer >= inner` → then (`inner += *outer`, returns `&inner`) · `*outer < inner` → else (`*outer += inner`, returns `outer`) | line 30 |
| **C. argument aliasing** | `outer` points to caller-owned storage (distinct from `inner`) · `outer == &inner`, i.e. the pointer returned by a previous then-branch fed back in, so `inner += inner` — the aliasing this library is named for | lines 32/35 + line 46 |
| **D. hidden static state `inner`** | fresh (`1`) · small positive · large positive · `INT_MAX` · `0` · negative · `INT_MIN`. Process-lifetime, mutated by every then-branch call, and **shared between both entry points** | line 29 |
| **E. input value shape** | `> inner` · `== inner` (the `>=` equality boundary) · `inner - 1` (one step below) · `0` · negative · `INT_MAX` · `INT_MIN` · random 32-bit patterns | line 30/31/34 |
| **F. call multiplicity** | 0 calls · 1 · 2 · many (state accumulates and wraps) | line 45 |
| **G. `iterations` shape (`driver`)** | `1` · `2` · many (long enough to overflow) | line 45 |
| **H. observable outputs** | returned **pointer identity** (`ret == outer` vs `ret == &inner`, and `&inner` stable across calls) · `*ret` value · the exact **bytes** `printf("%d\n", …)` writes | lines 32/35/47 |

Observables compared for every row: `*ret`, the *pointer-identity class* of `ret`
(`== outer`, or `== ` the address returned by earlier then-branch calls), the
caller's buffer value after the call (`*outer`, which the else branch mutates),
the probed value of `inner` afterwards, and for `driver` the captured stdout
byte stream.

### Keeping the hidden static in lockstep

`inner` cannot be reset (it is a process-lifetime `static` with no setter), and the
two `.so`s each hold their own copy. The harness therefore issues **exactly the
same call sequence to both libraries**, which keeps `C.inner == Rust.inner` as an
invariant no matter what order the tests run in. It also implements, purely
through the public API:

* `probe_inner()` — calls `static_alias` with `INT_MIN` (which takes the else
  branch and so leaves `inner` untouched) and recovers `inner` from the mutated
  buffer. A **non-mutating read** of the private static.
* `set_inner(T)` — drives `inner` to *any* target `T` by doubling through the
  aliased then-branch until it wraps to `0`, up to `INT_MIN`, then one final
  then-branch call. This is what makes rows D=`0`/negative/`INT_MIN`/`INT_MAX`
  reachable at all.

Both helpers drive C and Rust identically, so they preserve the invariant.

## Rows (each = one combination the C treats differently)

Every row runs **many randomized inputs** (fixed seed `0x5A17_A11A5` SplitMix64,
reproducible) unless the row is inherently a single fixed shape.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `static_alias` | B=then, C=distinct ptr, D=current, E=random `*outer > inner` · 200 random inputs | `cfg_c1_then_branch_distinct_random` | [x] |
| C2 | `static_alias` | B=then, C=distinct, E=`*outer == inner` exactly (the `>=` equality boundary) | `cfg_c2_then_branch_equality_boundary` | [x] |
| C3 | `static_alias` | B=else, C=distinct, E=random `*outer < inner` · 200 random inputs | `cfg_c3_else_branch_distinct_random` | [x] |
| C4 | `static_alias` | B=else, C=distinct, E=`*outer == inner - 1` (one step below boundary) | `cfg_c4_else_branch_one_below` | [x] |
| C5 | `static_alias` | C=**aliased** (`outer == &inner`), B=then necessarily, F=many — `inner += inner` doubling until it wraps and fixes at 0 | `cfg_c5_aliased_doubling` | [x] |
| C6 | `static_alias` | C=**chained** — feed the returned pointer straight back in, mixing both branches, exactly as `driver` composes it, but driven from the test · 64 chains × 40 steps | `cfg_c6_chained_returned_pointer` | [x] |
| C7 | `static_alias` | D=`0`, E ∈ {negative, `0`, positive, `INT_MAX`, `INT_MIN`} + random | `cfg_c7_inner_zero_state` | [x] |
| C8 | `static_alias` | D=negative, E spanning both branches + random | `cfg_c8_inner_negative_state` | [x] |
| C9 | `static_alias` | D=`INT_MAX`, E ∈ {`INT_MAX`, `INT_MAX-1`, `0`, negative} (then-branch overflows) | `cfg_c9_inner_intmax_state` | [x] |
| C10 | `static_alias` | D=`INT_MIN`, E arbitrary (then-branch is *always* taken since `x >= INT_MIN`) | `cfg_c10_inner_intmin_state` | [x] |
| C11 | `static_alias` | D=large positive, E ∈ {`INT_MAX`, `INT_MIN`, `0`, ±1} extremes | `cfg_c11_extreme_input_values` | [x] |
| C12 | `static_alias` | F=**many**, fully random state machine: 4000 random calls, randomly aliased or distinct, asserting every observable each step | `cfg_c12_random_state_machine` | [x] |
| C13 | `driver` | G=1, `initial_value >= inner` (first call takes then, prints `inner`) | `cfg_c13_driver_one_iteration_then` | [x] |
| C14 | `driver` | G=1, `initial_value < inner` (first call takes else, prints the local) | `cfg_c14_driver_one_iteration_else` | [x] |
| C15 | `driver` | G=2 — both then/else first steps, showing the switch to aliased doubling on step 2 | `cfg_c15_driver_two_iterations` | [x] |
| C16 | `driver` | G=many (40), so the aliased doubling overflows mid-run; full byte-compare of ~40 printed lines | `cfg_c16_driver_many_iterations_overflow` | [x] |
| C17 | `driver` | `initial_value` negative with `inner` positive — stays in the else branch across iterations | `cfg_c17_driver_negative_initial` | [x] |
| C18 | `driver` | D=`0` preset, G ∈ {1,2,5}, initial ∈ {negative, 0, positive} | `cfg_c18_driver_inner_zero` | [x] |
| C19 | `driver` | D=negative preset, initial spanning both branches | `cfg_c19_driver_inner_negative` | [x] |
| C20 | `driver` | `initial_value` ∈ {`INT_MIN`, `INT_MAX`, `INT_MIN+1`, `INT_MAX-1`}, G ∈ {1,3,10} | `cfg_c20_driver_extreme_initial` | [x] |
| C21 | `driver` | output **formatting** shapes: values that print as `0`, negative, and the 11-byte `-2147483648` | `cfg_c21_driver_output_formatting` | [x] |
| C22 | `driver` + `static_alias` | **cross-entry-point state sharing**: interleave `driver` calls and direct `static_alias` calls and check both libraries' shared `inner` evolves identically | `cfg_c22_interleaved_entry_points` | [x] |
| C23 | `driver` | G=many, called **repeatedly** (state carries from one `driver` call into the next) | `cfg_c23_driver_called_repeatedly` | [x] |
| C24 | `driver` | randomized: 150 random `(initial_value, iterations)` pairs, full stdout byte-compare each | `cfg_c24_driver_randomized` | [x] |

**Gate: every row C1–C24 passes across its randomized inputs. PASS.**

## How to reproduce

```
./run_diff_tests.sh          # everything: C build, feature combos, both
                             # profiles, symbol parity, C at -O2/-O3
```

`cargo test` alone is NOT sufficient: the crate is `crate-type = ["cdylib"]`
only, so integration tests cannot link it and cargo will leave a **stale**
`target/<profile>/libStaticAlias.so` in place — the tests `dlopen` that file, so
`cargo build` must run first. (This bit during development: a fix appeared not
to work because the old `.so` was still on disk.) Tests also require
`--test-threads=1`, since `driver` is verified by capturing the process-wide
fd 1; the harness detects pollution and fails loudly rather than silently.

## Results

| configuration | selfcheck | Phase B (C1–C24) | Phase C (E1–E11) |
|---------------|-----------|------------------|------------------|
| dev profile, C default cmake | 6/6 | 24/24 | 12/12 |
| release profile (`panic="abort"`, no debug-assertions) | 6/6 | 24/24 | 12/12 |
| dev profile vs C built `-O2` | 6/6 | 24/24 | 12/12 |
| dev profile vs C built `-O3` | 6/6 | 24/24 | 12/12 |

The `-O2`/`-O3` C builds matter because rows C5/C9/C16 and E3–E5/E9–E10 depend on
signed-overflow wraparound, which is UB in C; the ground truth turned out to be
identical at `-O0`/`-O2`/`-O3`, and the Rust (`wrapping_add`) matches all three.
