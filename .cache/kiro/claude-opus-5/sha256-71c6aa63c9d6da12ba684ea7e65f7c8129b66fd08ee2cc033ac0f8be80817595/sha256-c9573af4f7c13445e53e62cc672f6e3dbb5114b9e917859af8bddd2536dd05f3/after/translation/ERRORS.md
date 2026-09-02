# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping every `return`, every
`== NULL` / `!= NULL` test, every explicit range check, every clamp, and every
`#define`d limit. There are **no** `assert`s, no `errno` use, and no error enums in
this library; rejection is expressed as a NULL return, a `0` / `-1` sentinel, an
early `void` return, or a printed diagnostic.

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `get_operation` | `opcode < 0` — fails `opcode >= 0` (L76). e.g. `-1`, `-4`, `INT_MIN` | returns `NULL`; nothing printed | `err01_get_operation_negative` | [x] |
| 2 | `get_operation` | `opcode >= 4` — fails `opcode < 4` (L76). e.g. `4`, `5`, `INT_MAX` | returns `NULL`; nothing printed | `err02_get_operation_too_large` | [x] |
| 3 | `get_operation` | `opcode == OP_SHIFT` (`0x04`, L34) — the `OP_*` macros run `1..4` but the table is indexed `0..3`, so the highest documented opcode is itself out of range | returns `NULL` | `err03_get_operation_op_macros` | [x] |
| 4 | `get_operation` | one step past each end of the valid range: `-1` and `4` | `NULL` on both; `0` and `3` non-`NULL` | `err04_get_operation_boundary` | [x] |
| 5 | `execute_operation` | `func == NULL` (L84) | prints `Error: Operation function pointer is NULL for %s\n` with `op_name`; returns `0`; `func` never called | `err05_execute_operation_null_func` | [x] |
| 6 | `execute_operation` | `func == NULL` **and** `op_name == NULL` — `%s` receives a null pointer | glibc prints `…NULL for (null)`; returns `0` | `err06_execute_operation_null_func_null_name` | [x] |
| 7 | `execute_operation` | `func != NULL` but `op_name == NULL` (success path, `%s` null) | prints `Result of (null): <n>`; returns `func(a,b)` | `err07_execute_operation_null_name_success` | [x] |
| 8 | `compute_checksum` | `values == NULL` (L102), with `count > 0` | `checksum` stays `0`, `MAGIC_NUMBER` **not** applied; returns `0` | `err08_compute_checksum_null_values` | [x] |
| 9 | `compute_checksum` | `count == 0` — fails `count > 0` (L102) | returns `0` (no `MAGIC_NUMBER` xor) | `err09_compute_checksum_zero_count` | [x] |
| 10 | `compute_checksum` | `count < 0` (e.g. `-1`, `INT_MIN`) — fails `count > 0` (L102) | returns `0` | `err10_compute_checksum_negative_count` | [x] |
| 11 | `compute_checksum` | `values == NULL` **and** `count == 0` / `count < 0` | returns `0` | `err11_compute_checksum_null_and_bad_count` | [x] |
| 12 | `compute_checksum` | `count > 4` (L105 clamp `copy_count = (count > 4) ? 4 : count`), incl. `5`, `16`, `INT_MAX` | oversized length is **not** an error: reads only the first 4 ints, result identical to `count == 4` | `err12_compute_checksum_oversized_count` | [x] |
| 13 | `compute_checksum` | result range bound by `MASK_LOWER` (`0x0000FFFF`, L36) | return value is always `<= 0xFFFF` for every input | `err13_compute_checksum_mask_bound` | [x] |
| 14 | `init_state` | `state == NULL` (L117) | prints `Error: state pointer is NULL in init_state\n`; returns `void`; writes nothing | `err14_init_state_null` | [x] |
| 15 | `apply_operation` | `state == NULL` (L130) | prints `Error: state pointer is NULL in apply_operation\n`; returns; `func` never called | `err15_apply_operation_null_state` | [x] |
| 16 | `apply_operation` | `state != NULL`, `func == NULL` (L135) | prints `Error: operation function pointer is NULL in apply_operation\n`; returns; **`operation_count` is NOT incremented** and `accumulator` is unchanged | `err16_apply_operation_null_func` | [x] |
| 17 | `apply_operation` | `state == NULL` **and** `func == NULL` — check order matters (state is tested first) | only the *state* message is printed, not the *func* one | `err17_apply_operation_both_null` | [x] |
| 18 | `checkshift` | `malloc(sizeof(ComputeState))` returns `NULL` (L150) | prints `Error: Failed to allocate memory for state\n`; returns `-1`; no further output | `err18_checkshift_malloc_failure` (LD_PRELOAD malloc interposer) | [x] |
| 19 | `compute_checksum` | misaligned `int*` — the C reaches `values` only through `memcpy` (L106), which has no alignment requirement, so a misaligned pointer is *accepted*, not rejected | reads the same 4·count bytes; no alignment fault | `err_misaligned_values_pointer` | [x] |

## Finding: row 18 initially DIVERGED

Row 18 caught a real translation defect. LLVM recognises `malloc`/`free` by name and
had promoted the non-escaping 12-byte `ComputeState` block to registers, deleting
the allocation from the Rust `.so` entirely — confirmed with an `LD_PRELOAD`
interposer (0 `malloc` calls from Rust vs 1 from C) and in the disassembly (no
`malloc`/`free` call in Rust's `checkshift`). Consequences:

* `state == NULL` became unreachable, so under allocation failure the C returned
  `-1` after printing the diagnostic while the Rust returned a normal result. No
  happy-path test could see this.
* the `malloc`/`free` pair was invisible to any external allocator interposer.

Fixed in `src/lib.rs` by routing both calls through `#[inline(never)]`
trampolines that load the function pointer with `read_volatile`, which hides the
callee's identity from the optimiser so a real indirect call is always emitted.
`err18b_allocator_call_parity` is the regression guard: it asserts that N
`checkshift` calls produce the same number of `malloc(12)` and `free` calls on
both sides.

## Notes on non-rows

* L69 `if (ops[0] == NULL)` is a lazy-initialisation guard for a function-`static`
  table, not an input rejection: it is unobservable from outside because the table
  is always fully populated before the range check runs. It is covered implicitly by
  rows 1–4 (repeated calls must keep returning the same behaviour), by
  `cfg10_get_operation_repeat_calls`, and by `cfg10b_concurrent_dispatch_and_leaf_ops`
  (the lazy fill is a benign race in C; the Rust must not abort where C proceeds).
* There is no path in which `checkshift` returns anything other than `final_result`
  or `-1`. Note that `-1` is *also* a legal `final_result` value, so the sentinel is
  ambiguous in C; the Rust reproduces that ambiguity rather than distinguishing it.
* `free(state)` (L185) is unconditional after the null check, so there is no
  double-free or null-free path.
* **`memcpy` call counts differ and this is deliberate.** The C `.so` calls
  `memcpy` for the 12-byte struct copy in `init_state` and the ≤16-byte copy in
  `compute_checksum`; the Rust inlines the small fixed-size copies (verified with a
  `memcpy` interposer: C emits `[12]`+`[16]` from `checkshift`, Rust emits none).
  Unlike `malloc`, `memcpy` cannot fail and no C branch is keyed on it, so nothing
  observable — return values, state bytes, or emitted output — changes. The
  variable-length copy inside a directly-called `compute_checksum` *is* a real
  `memcpy` on both sides. Row 19 covers the one input class this could have
  affected (alignment).
