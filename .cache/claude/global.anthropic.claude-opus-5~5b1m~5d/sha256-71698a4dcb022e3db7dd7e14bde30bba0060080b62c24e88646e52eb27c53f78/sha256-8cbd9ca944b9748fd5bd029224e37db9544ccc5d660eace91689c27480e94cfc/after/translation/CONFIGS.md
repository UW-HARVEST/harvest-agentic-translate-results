# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C code in `c_src/src/lib.c` actually
takes on valid inputs.

## Axes the C code branches on

**Cargo features:** none. `translation/Cargo.toml` has no `[features]` section,
so the only build configuration is the default one (`--no-default-features` is
equivalent). `#[cfg]` in `src/lib.rs` is limited to `target_arch = "x86_64"`
vs. not, which is a target axis, not a feature axis.

**Public entry points (all six exported symbols, low-level first):**
`create_buffer`, `append_to_buffer`, `destroy_buffer`, `get_operation_name`,
`perform_operation`, and the one-shot wrapper `buffapp`.

**A1 — `create_buffer(initial_capacity)` shape:** `0`, `1`, small (`< 32`),
exactly `32` (what `buffapp` uses), large (`1 << 20`).

**A2 — `append_to_buffer` growth branch (`lib.c:57`):** taken vs. not taken;
i.e. `required_capacity <= capacity` (in-place `strcpy`) vs.
`required_capacity > capacity` (`realloc` to `required * 2`).

**A3 — `append_to_buffer` string shape:** empty, 1 byte, many bytes, a string
longer than the whole current capacity, embedded high-bit / non-ASCII bytes
(`strlen` stops only at NUL), repeated appends accumulating `length`.

**A4 — number of appends:** zero, one, many (the accumulating-`length` and
repeated-`realloc` path), interleaved with reads of `capacity`/`length`.

**A5 — `get_operation_name(op_code)`:** each of the 5 switch arms
(`0 add`, `1 subtract`, `2 multiply`, `3 divide`, `default unknown`).

**A6 — `perform_operation(a, b, operation)` string dispatch (`lib.c:95-107`):**
each of the 4 recognised strings plus the fall-through; and for `"divide"` the
`b != 0` sub-branch.

**A7 — operand magnitude:** small, `0`, negative, `INT_MIN`/`INT_MAX`
(exercises the wrapping/UB arithmetic and truncating division).

**A8 — how the `operation` pointer is obtained:** a caller-owned C string vs.
the pointer returned by `get_operation_name` (composed pipeline, the way
`buffapp` does it).

**A9 — `buffapp` operation pair:** `param1 % 4` × `param3 % 4`, including the
negative-remainder values `-1, -2, -3` that hit `get_operation_name`'s default.

**A10 — `buffapp` final branch (`lib.c:141`):** `intermediate3 != 0`
(division) vs. `== 0` (sum fallback).

**A11 — `buffapp` observable stdout:** the `printf("Computation Log:\n%s\n")`
bytes must match, in addition to the return value.

## Table

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| C1 | `create_buffer` + `destroy_buffer` | A1 = each of `{0, 1, 7, 32, 1<<20}`: compare returned `capacity`, `length`, `data[0]`, NULL-ness; then destroy | [x] |
| C2 | `create_buffer` + `destroy_buffer` | A1 randomized valid capacities in `1..=4096`; struct fields must match | [x] |
| C3 | `create_buffer` + `append_to_buffer` | A2 = growth NOT taken, A3 = short string, A1 = large capacity (`256`) — single append | [x] |
| C4 | `create_buffer` + `append_to_buffer` | A2 = growth TAKEN, A3 = string longer than capacity, A1 = `1` — single append | [x] |
| C5 | `create_buffer` + `append_to_buffer` | A2 boundary: `required == capacity` exactly (no growth) — computed per capacity in `1..=64` | [x] |
| C6 | `create_buffer` + `append_to_buffer` | A2 boundary: `required == capacity + 1` exactly (growth to `required*2`) — per capacity in `1..=64` | [x] |
| C7 | `create_buffer` + `append_to_buffer` | A3 = empty string `""`, A4 = repeated 8× (`length` never advances, growth only if `capacity == 0`) | [x] |
| C8 | `create_buffer` + `append_to_buffer` | A3 = 1-byte string, A4 = many appends (grow repeatedly from `capacity = 1`) | [x] |
| C9 | `create_buffer` + `append_to_buffer` | A3 = randomized-length randomized-content ASCII strings, A4 = many appends, A1 randomized: compare full buffer bytes + `capacity` + `length` + return codes after every step | [x] |
| C10| `create_buffer` + `append_to_buffer` | A3 = bytes with the high bit set (non-UTF-8, `0x80..=0xFF`), randomized, A4 = many appends | [x] |
| C11| `create_buffer` + `append_to_buffer` | A1 = `0` (capacity 0) then append — forces growth on the very first append from a zero-size allocation | [x] |
| C12| `get_operation_name` | A5 = each of `0, 1, 2, 3` — compare returned string bytes | [x] |
| C13| `get_operation_name` | A5 = `default` arm via randomized ints outside `0..=3` (incl. `INT_MIN`, `INT_MAX`) | [x] |
| C14| `perform_operation` | A6 = `"add"`, A7 = randomized `a`, `b` incl. extremes/`0`/negatives | [x] |
| C15| `perform_operation` | A6 = `"subtract"`, A7 = randomized incl. extremes | [x] |
| C16| `perform_operation` | A6 = `"multiply"`, A7 = randomized incl. extremes and `0` | [x] |
| C17| `perform_operation` | A6 = `"divide"` with `b != 0`, A7 = randomized incl. negative dividends/divisors (truncation toward zero) | [x] |
| C18| `perform_operation` | A6 = `"divide"` with `b == 0` (valid input, guarded path → `0`) | [x] |
| C19| `perform_operation` | A8 = `operation` pointer taken from the OTHER library's `get_operation_name` (C name → Rust `perform_operation` and vice versa), A5×A6 cross-product | [x] |
| C20| `create_buffer`+`append_to_buffer`+`get_operation_name`+`perform_operation`+`destroy_buffer` | full hand-composed pipeline reproducing `buffapp` from the low-level entry points, randomized params; compare buffer bytes and every intermediate | [x] |
| C21| `buffapp` | A9 = every `(param1 % 4, param3 % 4)` combination over `{0,1,2,3}` with fixed small params; compare return value AND stdout bytes | [x] |
| C22| `buffapp` | A9 = negative remainders: `param1 % 4` and/or `param3 % 4` in `{-1,-2,-3}` (`unknown` operations); compare return value AND stdout | [x] |
| C23| `buffapp` | A10 = `intermediate3 == 0` fallback (sum path), constructed inputs; compare return value AND stdout | [x] |
| C24| `buffapp` | A10 = `intermediate3 != 0` division path; compare return value AND stdout | [x] |
| C25| `buffapp` | A7 = extreme params (`INT_MAX`, `INT_MIN`, `0`, `±1`) in all four positions, cross-product subset; compare return value AND stdout | [x] |
| C26| `buffapp` | A7/A9/A10 fully randomized 4-tuples (fixed seed, many iterations); compare return value AND stdout bytes | [x] |
| C27| `buffapp` | interleaved/repeated invocations alternating C and Rust in one process (checks no hidden global state and identical heap behaviour) | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED src/lib.c)` — there
is no driver executable, and the Rust crate is `crate-type = ["cdylib"]` only.
So the "compare C and Rust binary stdout" clause is covered instead by rows
C21–C27, which capture and compare the `printf` output of `buffapp` byte-for-byte
through the FFI boundary.

## Findings

All 27 rows pass against randomized inputs (fixed seeds) in every build
configuration. One real divergence was found and fixed during this phase:

- The **dev-profile** `cdylib` diverged on the NULL-dereference path in
  `append_to_buffer` (`src/lib.rs:126`): Rust's debug `ub_checks` turned the
  raw-pointer deref into a non-unwinding panic (`SIGABRT`, signal 6) where the
  C raises `SIGSEGV` (signal 11). Fixed by disabling `debug-assertions` and
  `overflow-checks` in `[profile.dev]`, since the C is compiled without UB
  instrumentation. Re-verified with `BUFFAPP_RUST_SO` pointed at the dev `.so`.

Harness note: `buffapp` writes to stdout, so every `buffapp` comparison runs the
call in a forked child with a private fd 1. That also makes fatal signals
(`SIGSEGV`, `SIGFPE`) first-class comparable results instead of aborting the
test process. Run the suite with `--test-threads=1` (see `run_tests.sh`).
