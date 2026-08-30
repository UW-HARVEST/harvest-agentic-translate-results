# ERRORS.md — Error / rejection surface (Phase A)

Mechanically derived from `c_src/src/staticalias.c` + `c_src/include/staticalias.h`.

## Mechanical grep result (what the C actually checks)

```
grep -nE 'return|assert|NULL|errno|-1|MAX|MIN|if *\(|else|for *\(|while' \
     src/staticalias.c include/staticalias.h
src/staticalias.c:30:  if(*outer >= inner) {
src/staticalias.c:32:    return &inner;
src/staticalias.c:33:  } else {
src/staticalias.c:35:    return outer;
src/staticalias.c:49:  return;

grep -nE 'typedef|enum|struct|#define' ...
include/staticalias.h:25:#define STATICALIAS_H_      <- include guard only
```

Findings, stated exactly as the source supports them:

* There is **no** error-return macro (`RETURN_ERROR` &c.), **no** `return -1`,
  **no** `return NULL`, **no** error enum, **no** `errno` use.
* There is **no** `assert`, **no** null check, **no** range check, and **no**
  min/max constant.
* There is **no** `enum` and **no** `struct` in the public header, therefore
  there is no "out-of-range enum value" input to this API. The only parameter
  types are `int *` and two `int`s. The out-of-range-enum class of bug is
  covered instead by the out-of-range *integer* rows (#5–#10) below, since every
  `int` bit pattern is a legal argument that the C accepts without validation.
* The only `if` in the library (`*outer >= inner`) is a *behaviour* branch, not a
  rejection: both arms return a valid non-NULL pointer.

So the library's rejection surface consists **solely of implicit / undefined
behaviour conditions**. Each distinct one gets a row. "Expected C result" is
what the shipped C `.so` (built by `c_src/CMakeLists.txt`, i.e. no
`CMAKE_BUILD_TYPE` ⇒ `-O0`, no `-fwrapv`, no `-ftrapv`) actually does; the Rust
must match that observed behaviour, not the abstract C standard.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|----------------------------------------------|-------------------|------|----|
| 1 | `static_alias` | `outer == NULL` — `*outer` read at `staticalias.c:30` with no null check | no error value is returned: the process dies dereferencing address 0 (`SIGSEGV`, signal 11). Rust must die with the same signal, not return a sentinel. | `err_01_static_alias_null_pointer_segv` | [x] |
| 2 | `driver` | `iterations <= 0` (`0`, `-1`, `INT_MIN`) — `for (i = 0; i < iterations; ...)` at `staticalias.c:45` is not a rejection but the loop body never runs | returns normally (`void`), prints **zero bytes**, and leaves `inner` completely unmodified. No error, no output. | `err_02_driver_non_positive_iterations` | [x] |
| 3 | `driver` | `iterations > 0` with any `initial_value` — `initial_value` is a by-value parameter whose address escapes into `static_alias`, which may write through it (`*outer += inner`) | no error: the write lands in `driver`'s own stack copy and is visible only via the subsequent `printf`. The caller's argument is never affected. | `err_03_driver_writes_only_its_own_parameter_copy` | [x] |
| 4 | `static_alias` | `outer == &inner`, i.e. the pointer returned by a previous call is fed back in (self-aliasing write at `staticalias.c:31`) | `*outer >= inner` is `inner >= inner` ⇒ true, so `inner += inner` (doubling) and `&inner` is returned again. Never takes the `else` arm. | `err_04_static_alias_self_alias` | [x] |
| 5 | `static_alias` | signed-overflow UB in the `if` arm: `inner += *outer` overflows `INT_MAX` (e.g. `inner = INT_MAX`, `*outer = INT_MAX`) | no trap and no diagnostic: two's-complement wrap-around (`INT_MAX + INT_MAX == -2`). Returns `&inner`. | `err_05_overflow_then_branch` | [x] |
| 6 | `static_alias` | signed-overflow UB in the `else` arm: `*outer += inner` overflows below `INT_MIN` (e.g. `inner = INT_MIN`, `*outer = -1`, reachable because `-1 < INT_MIN` is false — use `inner = -1`, `*outer = INT_MIN`) | wrap-around, no trap; returns `outer` (the caller's pointer). | `err_06_overflow_else_branch` | [x] |
| 7 | `static_alias` | extreme in-range boundary values `INT_MIN` / `INT_MAX` / `0` / `-1` / `1` for both `*outer` and `inner` (one step past the "documented" — i.e. nonexistent — valid range: there is none, all `2^32` patterns are accepted) | branch chosen strictly by the signed comparison `*outer >= inner`; both arms return non-NULL. No rejection at any boundary. | `err_07_boundary_value_cross_product` | [x] |
| 8 | `driver` | signed-overflow UB reached transitively: `iterations` large enough that the doubling of `inner` (row 4) overflows repeatedly | wraps every iteration; `printf("%d\n", ...)` prints the wrapped, possibly negative values; never traps, never stops early. | `err_08_driver_overflow_by_iteration` | [x] |
| 9 | `driver` | `initial_value == INT_MIN` with `inner` positive: the `else` arm computes `INT_MIN + inner`, then keeps re-entering it | wraps/creeps upward one `inner` at a time, printing each value; no rejection. | `err_09_driver_int_min_initial_value` | [x] |
| 10 | `driver` | `iterations == INT_MAX` (largest accepted count) | not a rejection — an unbounded-time loop. Behaviour is identical to any large count; verified with counts large enough to exercise the same wrapped steady state instead of running `2^31` `printf`s. | `err_10_driver_huge_iteration_count_prefix` | [x] |
| 11 | `static_alias` | `outer` pointing at unwritable memory while the `else` arm is taken (`*outer += inner` writes through the caller's pointer) | `SIGSEGV` on the store; no error return. Same in Rust. | `err_11_static_alias_readonly_outer_segv` | [x] |

All 11 rows have a passing differential test in
`translation/tests/error_paths.rs` (rows 1 and 11 compare the fatal **signal**
of a forked child for C vs Rust, so "both failed" is not accepted — the same
signal number is required).
