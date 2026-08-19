# CONFIGS.md — configuration-surface table (Phase B)

## Axes the C code actually branches on

Derived from `c_src/src/main.c` (there is no header, no `#ifdef`, no runtime
option/flag: the whole configuration surface is the entry point that is called,
the state of the function-local `static`, and the shape of the input values).

* **A. Entry point** (the full set of public symbols, lowest level first):
  1. `int *static_alias(int *outer)` — the low-level function; callable directly
     through the `.so`, with the caller choosing what `outer` points at.
  2. `int main(int argc, char **argv)` — the one-shot wrapper that drives
     `static_alias` in a loop.
  3. *sequences*: repeated `static_alias` calls, repeated `main` calls, and
     `static_alias`/`main` interleavings — because `inner` has static storage
     duration its value survives every call, so call **order** is part of the
     configuration.
  4. the `driver` **program** (fresh process ⇒ `inner == 1` again).
* **B. `static_alias` branch**: `*outer >= inner` (then: `inner += *outer`,
  returns `&inner`) vs `*outer < inner` (else: `*outer += inner`, returns
  `outer`).
* **C. Pointer/aliasing shape of `outer`**: a caller object distinct from
  `inner` (stack local, heap cell, static cell, array element) vs `outer ==
  &inner`, i.e. feeding the returned pointer back in (what `main` does after the
  first `then` branch).
* **D. State of `inner`**: initial `1`; grown positive; `0`; negative; `INT_MIN`;
  wrapped after signed overflow.
* **E. Value shape of `*outer` / `initial_value`**: `INT_MIN`, negative, `-1`,
  `0`, `1`, positive, `INT_MAX`, random 32-bit; values that make
  `inner + *outer` / `*outer + inner` overflow.
* **F'. Process environment of the `driver` program** (only relevant for the
  program, not for the `.so`): disposition of `SIGPIPE` (the C runtime leaves the
  default, Rust's runtime ignores it), availability of file descriptor 1, and the
  locale environment (the program never calls `setlocale`, so it must stay
  irrelevant).
* **F. `argv[1]` / `argv[2]` string shape** (what `strtol` distinguishes): plain
  digits, leading whitespace (` `, `\t`, `\n`, `\v`, `\f`, `\r`), `+`/`-` sign,
  leading zeros, trailing garbage, `long`-range-but-not-`int`-range values,
  `LONG_MAX`/`LONG_MIN` boundary values, saturating values.
* **G. Iteration count** (`iterations` = `argv[2]`): `0`, `1`, `2`, few, many
  (enough to reach doubling overflow ≈ 32+), `INT_MAX`-narrowed values.

Rejection/invalid combinations live in `ERRORS.md`; this table is the valid
surface.

## Rows

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|--------------------------------------------|------|---|
| 1 | `static_alias` | fresh image (`inner == 1`), single call, `*outer` random in `[-2^31, 2^31)` — hits both branches; `outer` = stack-like local cell owned by the caller | `alias_single_call_random` | [x] |
| 2 | `static_alias` | `*outer < inner` (else branch), `inner == 1`, `*outer` ∈ {`INT_MIN`, -1000000, -1, 0} — returns the caller's pointer, mutates the caller's object; plus 20 randomized values from `[INT_MIN, 0]` | `alias_else_branch_shapes` | [x] |
| 3 | `static_alias` | `*outer >= inner` (then branch), `inner == 1`, `*outer` ∈ {1, 2, 1000, `INT_MAX`} — returns `&inner`, caller object untouched; plus 20 randomized values from `[1, INT_MAX]` | `alias_then_branch_shapes` | [x] |
| 4 | `static_alias` | `*outer == inner` exactly (equality edge of `>=`), for `inner` = 1 and for a grown `inner`, one below and one above the edge; plus 20 randomized edges (`Fresh(k)`, then exactly `k+1`, `2(k+1)`, `4(k+1)-1`, `4(k+1)`) | `alias_equality_edge` | [x] |
| 5 | `static_alias` | self-aliasing: `outer == &inner` (the returned pointer fed back), chained 1..40 times ⇒ doubling until signed overflow wraps, including the `inner == 0` and `inner == INT_MIN` states reached that way | `alias_self_aliasing_chain` | [x] |
| 6 | `static_alias` | overflow in the *then* branch (`inner + *outer > INT_MAX`, e.g. `inner` grown to `2^30`, `*outer = INT_MAX`) and in the *else* branch (`*outer + inner < INT_MIN`, e.g. `inner` negative, `*outer = INT_MIN`); plus 15 randomized pairs per branch | `alias_overflow_both_branches` | [x] |
| 7 | `static_alias` | `inner` driven negative / to `0` / to `INT_MIN` first, then all of `*outer` ∈ {`INT_MIN`, -1, 0, 1, `INT_MAX`} (state × value cross-product, 35 combinations) plus 8 randomized values per state | `alias_state_value_matrix` | [x] |
| 8 | `static_alias` | long randomized call sequences (500 steps, fixed seed) mixing: new random caller cell, chained returned pointer, non-destructive `INT_MIN` probe of `inner` — pointer identity (`ret == outer` vs `ret != outer`), `*ret` and the caller cell compared after every step | `alias_random_sequences` | [x] |
| 9 | `static_alias` | storage class of the caller object varies: heap `Box`, element of an array (with neighbours checked for out-of-bounds writes), and a `static`-lifetime cell; plus 10 randomized 12-step sequences mixing array elements, a heap cell and the chained pointer, checking array neighbours every step | `alias_caller_storage_shapes` | [x] |
| 10 | `main` | `argc == 3`, `iterations == 1`, `initial_value` random — exactly one `static_alias` call, both branches | `cfg10_single_iteration` (in `ffi_main::main_entry_point_differential`) | [x] |
| 11 | `main` | `argc == 3`, `iterations == 0`/`-0`/`+0` (valid but empty loop), fixed and 10 randomized initial values | `cfg11_iterations_zero` | [x] |
| 12 | `main` | `initial_value >= inner` on the first iteration (then branch first) then chained doubling: `iterations` ∈ {2, 3, 5, 10, 31, 32, 33, 40, 64} ⇒ signed overflow of the doubling is exercised; plus 25 randomized `(value ≥ 1, count ∈ [2,70])` pairs | `cfg12_then_first` | [x] |
| 13 | `main` | `initial_value < inner` on the first iteration (else branch first, repeated growth of the caller's local until it catches up with `inner`), `initial_value` ∈ {0, -1, -5, -100, `INT_MIN`}, `iterations` ∈ {1, 3, 10, 120}; plus 40 randomized `(value ≤ 0, count)` pairs | `cfg13_else_first` | [x] |
| 14 | `main` | `initial_value` = `INT_MAX` / `INT_MIN` / `-1` / `0` / `1` × `iterations` ∈ {1, 2, 5, 33} (boundary value × count cross-product); plus 30 randomized values from the ±64 neighbourhood of `INT_MAX`, `INT_MIN`, `±2^32`, `LONG_MAX`, `LONG_MIN` | `cfg14_boundary_matrix` | [x] |
| 15 | `main` | `argv[1]` shape sweep with a fixed valid `argv[2]`: plain, `+`-signed, `-`-signed, leading blanks (all six C-locale space characters), leading zeros, trailing garbage, `int`-overflowing but `long`-representable, `LONG_MAX`, `LONG_MIN`, saturating (33 shapes × 3 counts); plus 60 randomized prefix+digits+suffix strings | `cfg15_arg1_shapes` | [x] |
| 16 | `main` | `argv[2]` shape sweep with a fixed valid `argv[1]`: same shapes, so `iterations` also comes from narrowing/saturating conversions; plus 60 randomized prefix+count+suffix strings | `cfg16_arg2_shapes` | [x] |
| 17 | `main` | randomized `(argv[1], argv[2])` string pairs from a shape generator (200 pairs, fixed seed); `argv[2]` is generated so the iteration count stays ≤ 64 | `cfg17_random_pair*` | [x] |
| 18 | `main` | repeated `main` calls in one loaded image: `inner` carries over, so identical arguments produce different output the second/third time; plus 40 randomized argument pairs replayed against one shared image | `cfg18_repeat_*` (6 identical calls, then negative values, then error paths in between) | [x] |
| 19 | `main`, `static_alias` | interleaved: `static_alias` calls before/after `main` calls (state shared between the two entry points), randomized order (fixed seed) | `cfg19_interleaved*` | [x] |
| 20 | `main` | `argc == 3` with `argv[0]` varying (ignored by the code) and with extra `argv` entries present past index 2 | `cfg20_argv0`, `cfg20_extra_argv` | [x] |
| 21 | `driver` program | fresh process per invocation (`inner == 1`): randomized `(initial, iterations)` pairs, 300 cases, fixed seed, compared on stdout bytes, stderr bytes and exit status | `cli_random_pairs` | [x] |
| 22 | `driver` program | fresh process, hand-picked shapes: `1 40` (overflow), `-2147483648 120`, `2147483647 5`, `0 5`, `1 1`, `1073741824 4`, whitespace/sign/garbage argument strings (29 × 12 matrix); plus 80 randomized prefix+digits+suffix pairs | `cli_shape_matrix` | [x] |
| 23 | `driver` program | argument-count sweep 0..5 and unparsable arguments (mirrors `ERRORS.md` rows 1–3 at the process level); plus 180 randomized digit-free strings in either or both positions | `cli_err_argc_sweep`, `cli_err_unparsable` | [x] |
| 24 | `driver` program | high iteration counts (`1 1000`, `-1000 3000`, `-2147483648 5000`) — long output streams compared byte for byte; plus 20 randomized `(value, 500..4000 iterations)` pairs | `cli_long_output`, `cli_oversized_arguments` | [x] |
| 25 | `main` | 400 random byte strings for `argv[1]` drawn from the alphabet `strtol` reacts to (digits, all six C-locale spaces, `+`/`-`, `.eExX`, punctuation, `0x80`/`0xa0`/`0xff`), one iteration so the converted value is visible | `cfg25_random_bytes1_*` in `main_entry_point_differential` | [x] |
| 26 | `main` | 250 random byte strings for `argv[2]` (≤ 3 characters, so the iteration count stays bounded) | `cfg26_random_bytes2_*` in `main_entry_point_differential` | [x] |
| 27 | `driver` program | the reader of the output pipe closes early during a long stream: the C program dies from the default `SIGPIPE` disposition (status 141) — the Rust runtime sets `SIGPIPE` to `SIG_IGN`, which `src/main.rs` has to undo | `cli_closed_pipe_sigpipe` | [x] |
| 28 | `driver` program | file descriptor 1 closed before `exec`: `printf` fails and the C code ignores the error | `cli_closed_stdout` | [x] |
| 29 | `driver` program | locale environment (`LC_ALL`/`LANG`/`LC_NUMERIC` = `C`, `POSIX`, `de_DE.UTF-8`, `tr_TR.UTF-8`, `en_US.UTF-8`, invalid): the program never calls `setlocale()`, so nothing may change | `cli_locale_environment` | [x] |
| 30 | `driver` program | executed with a completely empty `argv` (`argc == 0`, allowed by `execve`) | `cli_empty_argv` | [x] |

## Build configurations

`Cargo.toml` declares **no `[features]`**, and `c_src/CMakeLists.txt` declares no
options, no `#ifdef`-selected backends and no build types (it is a bare
`add_executable`). The complete set of valid feature combinations is therefore:

| # | feature combination | command |
|---|---------------------|---------|
| 1 | *(none — default)* | `cargo check --no-default-features` / `cargo test --no-default-features` |
| 2 | *(all features = none)* | `cargo check --all-features` / `cargo test --all-features` |

Both are the same configuration; `scripts/check_features.sh` enumerates the
`[features]` table mechanically and runs `cargo check`/`cargo test` for every
combination it finds (plus the empty one), so the loop stays correct if features
are ever added.
