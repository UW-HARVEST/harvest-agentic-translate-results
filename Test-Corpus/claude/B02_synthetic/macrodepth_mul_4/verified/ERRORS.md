# ERRORS.md — error / rejection surface table (Phase C)

Derived mechanically by grepping **every** control-flow and rejection construct in
`c_src/src/*.c` and `c_src/src/*.h`:

```sh
grep -nE "return|assert|NULL|error|exit|abort|if *\(|switch|default:|case |fprintf|stderr" c_src/src/*.c c_src/src/*.h
```

The complete result set is small and is reproduced here in full. The library
translation unit (`mdcore.c`) contains **no** `assert`, **no** `NULL` check, **no**
error enum, **no** negative/sentinel return and **no** range check — its five
public functions are unconditional arithmetic plus `printf`. Consequently the
whole rejection surface of the project consists of the rows below: one `argc`
guard in `main`, the `default:` arm of the `DISPATCH_REP` `switch`, the `atoi`
input-parsing behaviour `main` relies on, and the signed-overflow boundaries of
the arithmetic itself.

Rows are checked off only once the corresponding differential test passes against
**both** libraries.

| # | function | trigger (exact invalid input / condition) | expected C result | test | ✔ |
|---|----------|--------------------------------------------|-------------------|------|---|
| 1 | `main` (`mdmain.c:29`) | `argc < 3` — invoked with **no** arguments (`argc==1`) | `fprintf(stderr,"usage: %s A B\n",argv[0])`, no stdout, exit status **2** | `driver_bin.rs::usage_no_args` | [x] |
| 2 | `main` (`mdmain.c:29`) | `argc < 3` — invoked with **one** argument (`argc==2`) | same usage line on stderr, no stdout, exit status **2** | `driver_bin.rs::usage_one_arg` | [x] |
| 3 | `main` (`mdmain.c:29`) | `argc >= 3` boundary — exactly two arguments | guard **not** taken; full normal output, exit status **0** | `driver_bin.rs::argc_boundary_two_args` | [x] |
| 4 | `main` | extra arguments beyond the first two (`argc > 3`) | surplus `argv` silently ignored; identical output to the 2-arg case | `driver_bin.rs::extra_args_ignored` | [x] |
| 5 | `use_generated` → `accum_<OP>` (`mdmacros.h:91` `default: break;`) | `n` **negative** (e.g. `-1`, `-7`) — no `case` matches | `switch` falls to `default: break;`, accumulator left at `INIT_FOR(OP)`; returns `INIT` (`0` for add/sub, `1` for mul) and prints `gen.acc=<INIT>` | `errors.rs::use_generated_negative` | [x] |
| 6 | `use_generated` → `accum_<OP>` (`default: break;`) | `n == 7` — one step past the last `case 6` | `default: break;` → returns `INIT` (**not** the 7-step value; the `switch` stops at 6 even when built with `REPEAT=7`) | `errors.rs::use_generated_seven_is_default` | [x] |
| 7 | `use_generated` → `accum_<OP>` (`default: break;`) | `n` far out of range (`8`, `100`, `INT_MAX`) | `default: break;` → returns `INIT` | `errors.rs::use_generated_far_out_of_range` | [x] |
| 8 | `use_generated` → `accum_<OP>` (`default: break;`) | `n == INT_MIN` (extreme negative, no `case`) | `default: break;` → returns `INIT` | `errors.rs::use_generated_int_min` | [x] |
| 9 | `use_generated` → `accum_<OP>` | `n` in-range boundary `0` — `case 0: REP0` expands to *nothing* | returns `INIT` unchanged, indistinguishable from the `default` arm | `errors.rs::use_generated_zero_boundary` | [x] |
| 10 | `use_generated` → `accum_<OP>` | `n == 6`, the **last** valid `case` | performs all 6 steps (add→`15`, sub→`-15`, mul→`720`) — must *not* fall through to `default` | `errors.rs::use_generated_last_valid_case` | [x] |
| 11 | `op_add` (`mdcore.c:28`) | signed overflow: `INT_MAX + 1`, `INT_MIN + (-1)`, `INT_MAX+INT_MAX` | C signed overflow is UB; the flags CMake uses (no `-ftrapv`, no `-fsanitize`, no optimisation) make gcc emit a plain `add` that **wraps** two's-complement. Rust must use `wrapping_add`, never panic | `errors.rs::op_overflow_boundaries` | [x] |
| 12 | `op_sub` (`mdcore.c:29`) | signed overflow: `INT_MIN - 1`, `INT_MAX - INT_MIN` | wraps two's-complement (as above) | `errors.rs::op_overflow_boundaries` | [x] |
| 13 | `op_mul` (`mdcore.c:30`) | signed overflow: `INT_MAX * INT_MAX`, `INT_MIN * -1`, `INT_MIN * INT_MIN` | wraps two's-complement (as above) | `errors.rs::op_overflow_boundaries` | [x] |
| 14 | `helper_call` | overflow of `return r + acc` when `r` is near `INT_MAX` (e.g. `a=INT_MAX,b=0`, `REPEAT>=2`) | wrapping sum; also `printf` must still report the un-wrapped `r`/`acc` | `errors.rs::helper_call_return_overflow` | [x] |
| 15 | `helper_call` (OP=mul) | `INIT_mul=1` repeatedly multiplied — `REPEAT=7` gives `5040`, and `r` overflow via `a=b=INT_MAX` | wrapping arithmetic throughout | `errors.rs::helper_call_return_overflow` | [x] |
| 16 | `G_OP` | reassignment through the exported **writable** global (`G_OP = op_sub`), then call | store succeeds (symbol is in `.data`); subsequent call dispatches to the new function. Must **not** fault | `globals.rs::g_op_is_writable_like_c` | [x] |
| 17 | `G_OP_NAME` | reassignment through the exported writable global | store succeeds; must **not** fault | `globals.rs::g_op_name_is_writable_like_c` | [x] |
| 18 | `main` → `atoi` (`mdmain.c:35-36`) | non-numeric argument (`"abc"`, `""`, `"+"`, `"-"`, `"x1"`) | `atoi` returns **0**; no rejection, computation proceeds with `0`, exit **0** | `driver_bin.rs::atoi_non_numeric` | [x] |
| 19 | `main` → `atoi` | trailing garbage (`"12abc"`, `"3.9"`, `"7 8"`) | parses the leading digits only (`12`, `3`, `7`), ignores the rest | `driver_bin.rs::atoi_trailing_garbage` | [x] |
| 20 | `main` → `atoi` | leading whitespace / explicit sign (`"  42"`, `"\t-7"`, `"+5"`) | whitespace skipped, sign honoured | `driver_bin.rs::atoi_whitespace_and_sign` | [x] |
| 21 | `main` → `atoi` | value exceeding `int` (`"2147483648"`, `"-2147483649"`, `"4294967296"`) | glibc `atoi` = `(int)strtol(...,10)`: fits in `long`, then **truncated** to `int` (→ `-2147483648`, `2147483647`, `0`) | `driver_bin.rs::atoi_int_overflow` | [x] |
| 22 | `main` → `atoi` | value exceeding `long` (`"99999999999999999999"`, `"-99999999999999999999"`) | `strtol` saturates at `LONG_MAX`/`LONG_MIN`, then truncates to `int` (→ `-1`, `0`) | `driver_bin.rs::atoi_long_overflow` | [x] |
| 23 | `main` → `atoi` | `int` boundary values passed exactly (`"2147483647"`, `"-2147483648"`) | parsed exactly; downstream arithmetic wraps | `driver_bin.rs::atoi_int_boundaries` | [x] |

## Generic FFI boundary conditions (not in the C's own check list)

Covered by `errors.rs` even though the C code performs no such validation, because
they are inputs a real caller can produce across the FFI boundary:

| # | condition | expected behaviour | test | ✔ |
|---|-----------|--------------------|------|---|
| G1 | every public function called with `0`, `1`, `-1`, `INT_MIN`, `INT_MAX` in each argument position (full 5×5 cross-product for the binary functions) | identical `int` return **and** identical `printf` bytes from both libraries | `errors.rs::exhaustive_boundary_cross_product` | [x] |
| G2 | "out-of-range enum" analogue: `use_generated` receives an `int` with no matching `switch` label. C `switch` accepts any `int`, so this is a real input — swept over all of `-8..=15`, plus `INT_MIN`, `INT_MIN+1`, `INT_MAX-1`, `INT_MAX` | rows 5–8: `default: break;` → `INIT` | `errors.rs::use_generated_full_sweep` | [x] |
| G3 | `G_OP` used as a function pointer *after* being overwritten with each of the three `op_*` implementations taken from the **other** library (cross-library function pointer) | dispatch follows the stored pointer; both libraries agree | `globals.rs::g_op_cross_library_dispatch` | [x] |
| G4 | NUL-terminated string read through `G_OP_NAME` (pointer deref across FFI) | exactly `"add"` / `"sub"` / `"mul"` for the configured `OP`, 3 bytes + NUL | `globals.rs::g_op_name_matches_c` | [x] |
| G5 | repeated / interleaved calls to all six functions in one process (shared global state, `printf` buffering) | no cross-call state; byte-identical interleaved stdout | `configs.rs::interleaved_call_sequence` | [x] |
| G6 | `argc == 0` (bare `execve` with an empty `argv`), so `argv[0]` is absent | both print exactly `usage:  A B\n` to stderr, nothing to stdout, exit **2** — byte-compared out-of-band with an `execve` probe, and the reachable `argv[0] == ""` variant is pinned by a test | `driver_bin.rs::empty_argv0_is_formatted_identically` | [x] |
| G7 | arguments that are not valid UTF-8 (`0xff`, `0x80`, `0xc3` bytes) | `atoi` sees raw bytes; a `String`-based translation would panic or mangle them. Identical stdout + exit status | `driver_bin.rs::non_utf8_arguments_are_handled_like_c` | [x] |
| G8 | randomized `atoi` fuzz: 120 random strings over `0-9 + - space tab nl cr x a-f . , e`, digit runs of every length 1..21, and 5000/3000-digit strings | identical stdout + exit status for every case (`atoi` is the only libc function the translation reimplements by hand) | `driver_bin.rs::atoi_randomized_fuzz` | [x] |

**Null pointers:** no public function in `mdcore.c` takes a pointer parameter, so
there is no null-pointer input to test at the function level. The only pointer
values in the API are the two exported globals, covered by rows 16–17 and G3–G4.

## Result

All 23 numbered rows and all 8 generic rows have a passing differential test, in
**both** the debug and the release profile, under **all 45 verified Cargo feature
combinations** (`./check_all.sh test` and `./check_all.sh release`).

Note that the *debug* profile is the stricter one for rows 11–15: Rust's debug
build panics on `+`/`-`/`*` overflow, so the `INT_MIN`/`INT_MAX` rows passing
there proves the translation genuinely uses `wrapping_*` rather than accidentally
matching gcc's wrap-around.
