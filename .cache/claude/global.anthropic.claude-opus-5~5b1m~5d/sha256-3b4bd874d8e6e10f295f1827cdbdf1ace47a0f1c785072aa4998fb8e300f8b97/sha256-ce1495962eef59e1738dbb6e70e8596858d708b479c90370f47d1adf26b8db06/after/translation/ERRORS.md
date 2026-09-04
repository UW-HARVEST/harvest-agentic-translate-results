# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every line of `c_src/src/lib.c` and `c_src/include/lib.h` was grepped for
rejection machinery. Result of each grep over the whole C subtree:

| pattern searched | hits |
|---|---|
| `return -1`, `return 0;` as sentinel, `return NULL` | 0 |
| `RETURN_ERROR`, `GOTO_ERROR`, `CHECK`, `_ASSERT`, `FAIL` macros | 0 |
| `assert(` / `<assert.h>` | 0 |
| `errno`, `perror`, `strerror` | 0 |
| `enum` (error enums or any enum) | 0 |
| `if (`, `switch (`, `while (`, `for (`, `?:` — any branch at all | 0 |
| explicit range / bounds check (`<=`, `>=`, or `<`/`>` used as a comparison) | 0 — the 3 `<` and 8 `>` characters in the C are all `#include <stdint.h>`, the shift operators `<<` / `>>`, and the member arrow `->` |
| null check (`== NULL`, `!ptr`, `if (rnd)`) | 0 |
| `#define` min/max constants / limits | 0 |
| `malloc` / `free` / allocation failure path | 0 |

**The C library has NO error surface.** `next_double` is straight-line code:
it unconditionally dereferences `rnd`, mutates `rnd->state`, and returns a
`double`. Every `uint64_t` bit pattern in `state[0]`/`state[1]` is a *valid*
input (those are covered by `CONFIGS.md`, not here), there is no return-code
channel, and there is no input the C code rejects.

## Table

There is exactly one input the C code cannot handle, and it is UB rather than a
defined rejection. It is listed for completeness, with the generic-boundary
rows the task requires.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `next_double` | `rnd == NULL` | Undefined behaviour: unconditional `rnd->state[0]` load with no null check ⇒ SIGSEGV on a null page. No error code is returned; there is no channel to return one. | `phase_c_e1_null_pointer_both_segfault` (forks a child per library, asserts BOTH die on the same fatal signal) | [x] |
| E2 | `next_double` | pointer to a *misaligned* `cn_rnd_t` (odd address) | No alignment check exists. On x86-64 the `mov`/`xor` sequence tolerates unaligned 8-byte access, so the call succeeds and must return the same value as the aligned call. | `phase_c_e2_misaligned_state` | [x] |
| E3 | `next_double` | pointer to a `cn_rnd_t` at the very end of a mapped page (no readable slack past `state[1]`) | Reads exactly 16 bytes, writes exactly 16 bytes, never past the struct ⇒ succeeds; no over-read. Rust must not over-read either. | `phase_c_e3_no_overread_past_struct` | [x] |
| E4 | `next_double` | "out-of-range enum value across the FFI boundary" | **Not applicable / vacuous:** the C API declares no `enum`, no flag, no mode, and no integer selector parameter. The only parameter is `cn_rnd_t *`. There is no int-typed input whose domain could be exceeded, because every `uint64_t` state is valid. Documented so the requirement is discharged explicitly rather than silently skipped. | `phase_c_e4_no_enum_parameters_exist` (asserts the C header contains no `enum`) | [x] |
| E5 | `next_double` | "zero and oversized lengths" | **Not applicable / vacuous:** the API takes no length, count, size, or buffer parameter — the struct is fixed at `uint64_t state[2]`. The degenerate *value* case (all-zero state, which makes the generator absorb at 0) is a VALID input and is covered as row C1 of `CONFIGS.md`. | `phase_c_e5_no_length_parameters_exist` | [x] |

## Divergence found and fixed (row E2)

Row **E2 initially FAILED**, and it was a genuine translation bug, not a test
artifact:

- The original Rust did `let rnd = unsafe { &mut *rnd };` and then indexed
  `rnd.state[..]`. Forming a `&mut cn_rnd_t` **requires 8-byte alignment**, so
  a misaligned `cn_rnd_t *` aborted the process
  (`misaligned pointer dereference: address must be a multiple of 0x8`).
- The C performs plain `uint64_t` loads/stores through the pointer with no
  alignment check and, on x86-64, happily accepts a misaligned struct.
- **Fix (in Rust only):** `cn_rnd_next` now takes a `*mut cn_rnd_t` and touches
  the state exclusively via `addr_of_mut!` + `read_unaligned` /
  `write_unaligned`, never forming a reference. The store order (`state[0]`
  then `state[1]`) and the single-read-per-word structure of the C are
  preserved, so aligned behaviour is bit-identical while misaligned input is now
  tolerated exactly as the C tolerates it.

## Note on row E1 and build profiles

`debug_assertions` builds enable Rust's `ub_checks`, which intentionally trap a
null dereference and `abort()` (SIGABRT) rather than faulting (SIGSEGV). That is
a development diagnostic, not an ABI difference. The E1 test therefore makes its
exact signal-for-signal comparison against the **release** cdylib — the shipped
artifact, built like the C `.so` with no UB instrumentation — where both die
with SIGSEGV. It additionally asserts the test-profile `.so` still dies fatally
and never silently returns a value.

## Completion

- [x] Every row above has a passing differential test (or an explicit,
      source-verified not-applicable justification plus a test asserting the
      justification still holds).
