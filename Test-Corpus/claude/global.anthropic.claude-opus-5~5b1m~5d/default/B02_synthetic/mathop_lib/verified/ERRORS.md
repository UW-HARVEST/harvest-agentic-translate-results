# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. There are **no** `RETURN_ERROR`
macros, no `assert`s, no `errno` use and no error-enum returns in this library:
every rejection is either an early `return 0`, a `default:` fall-through, a
`NULL`-check guard, a bounds guard, or a `calloc` failure. `StatusCode` declares
`STATUS_ERROR = -1` / `STATUS_WARNING = 1` but **no code path ever assigns
them** — only `STATUS_SUCCESS` (line 130) is ever written. That itself is a row
(#14) because a translation that "helpfully" reports errors would diverge.

Every row below is a distinct rejection/guard the C source actually contains,
with the exact `c_src/src/lib.c` line that implements it.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|----------------------------------------------|-------------------|------|---|
| 1 | `is_valid_operation` (line 53) | `op_char == 0` (the `op_char &&` short-circuit rejects NUL) | returns `false` | `err_01_is_valid_nul` | [x] |
| 2 | `is_valid_operation` (line 53) | `op_char < '1'` (0x01..0x30, incl. `'0'`) | returns `false` | `err_02_is_valid_below_range` | [x] |
| 3 | `is_valid_operation` (line 53) | `op_char > '5'` (`'6'`..0x7f) | returns `false` | `err_03_is_valid_above_range` | [x] |
| 4 | `is_valid_operation` (line 53) | `op_char` negative — i.e. every byte 0x80..0xff reinterpreted as **signed** `char` on x86-64; fails `>= '1'` | returns `false` for all 128 high bytes | `err_04_is_valid_negative_char` | [x] |
| 5 | `divide_operation` (lines 75-77) | `b == 0` (division-by-zero guard) | returns `0`, never traps | `err_05_divide_by_zero` | [x] |
| 6 | `modulo_operation` (lines 82-84) | `b == 0` (modulo-by-zero guard) | returns `0`, never traps | `err_06_modulo_by_zero` | [x] |
| 7 | `select_operation` (lines 100-101) | `op` matches no `case`: `0`, `6`, negative, `INT_MIN`, `INT_MAX` — a C enum accepts any `int` | `default:` → returns the **`add_operation`** pointer | `err_07_select_operation_out_of_range_enum` | [x] |
| 8 | `select_operation` (lines 100-101) | out-of-range `op` reached indirectly through `perform_computation_with_history` | behaves as addition (`a + b`) | `err_08_pcwh_out_of_range_enum_adds` | [x] |
| 9 | `allocate_results` (line 113) | `count == 0` → `calloc(0, 24)` | glibc returns a non-`NULL` unique pointer (not an error) | `err_09_allocate_zero_count` | [x] |
| 10 | `allocate_results` (line 113) | `count < 0` → sign-extended to a huge `size_t`, `calloc` overflow/OOM | returns `NULL` | `err_10_allocate_negative_count` | [x] |
| 11 | `allocate_results` (line 113) | `count == INT_MAX` → `calloc(2147483647, 24)` ≈ 48 GiB | returns `NULL` (allocation failure is *not* reported, just `NULL`) | `err_11_allocate_int_max` | [x] |
| 12 | `perform_computation_with_history` (lines 122-125) | `*history == NULL` on entry (uninitialised history) | not an error: allocates 10 slots and **resets `*history_count` to 0**, discarding the caller's count | `err_12_pcwh_null_history_resets_count` | [x] |
| 13 | `perform_computation_with_history` (line 127) | `*history_count >= 10` (history full, incl. values `10`, `11`, `1000`, `INT_MAX`) | the write **and** the increment are skipped; the arithmetic result is still returned; `*history_count` is left unchanged | `err_13_pcwh_history_full_skips_write` | [x] |
| 14 | `perform_computation_with_history` (line 127) | `*history_count < 0` (negative count, e.g. `-1`) | guard passes → **out-of-bounds write before the buffer** and count incremented; C does *not* reject this | `err_14_pcwh_negative_count_writes_oob` | [x] |
| 15 | `perform_computation_with_history` (line 130) | any successful record | `status` is always `STATUS_SUCCESS (0)`; `STATUS_ERROR`/`STATUS_WARNING` are unreachable | `err_15_status_always_success` | [x] |
| 16 | `mathop` (lines 144-146) | `is_valid_operation((char)(param1 % 128))` is false | `validation_char = '1'` is a **dead store** — the value is never read again, so an invalid `param1` has *no* observable effect on the result | `err_16_mathop_validation_is_dead_store` | [x] |
| 17 | `mathop` (line 148) | `param3 < 0` → `(param3 % 5) + 1` yields `0`, `-1`, `-2`, `-3` | out-of-range `Operation`; `select_operation` `default:`→ add, and `get_operation_priority` returns `op*10` ≤ 0 (e.g. `-30`) | `err_17_mathop_negative_param3_bad_enum` | [x] |
| 18 | `mathop` (line 156) | `param4 == -1` → `((-1+1) % 5) + 1 == 1`; `param4 == INT_MAX` → `INT_MAX+1` overflows | second op enum also escapes `1..5` for negative `param4` (e.g. `param4 = -3` → `-1`) | `err_18_mathop_negative_param4_bad_enum` | [x] |
| 19 | `mathop` (line 141) | `param1 == INT_MIN` → `INT_MIN % 128` | `0`, so `is_valid_operation(0)` → row #1 path | `err_19_mathop_int_min_param1` | [x] |
| 20 | `divide_operation` / `modulo_operation` (lines 78, 85) | `a == INT_MIN && b == -1` — C signed-division overflow is **undefined**; on x86-64 `idiv` raises `#DE` | process dies with **`SIGFPE` (signal 8)** | `err_20_*` (5 tests, fork-based) | [x] |
| 21 | generic FFI boundary | `perform_computation_with_history(a, b, op, NULL, cnt)` / `(.., h, NULL)` — the C **never** null-checks these out-params and dereferences them unconditionally (`*history` line 122, `*history_count` line 127) | process dies with **`SIGSEGV` (signal 11)** | `err_21_*` (5 tests, fork-based) | [x] |
| 22 | generic FFI boundary | signed overflow in `add_operation`/`subtract_operation`/`multiply_operation` (e.g. `INT_MAX + 1`) and in `get_operation_priority` (`op * 10` with `op = INT_MAX`) — UB in C, wraps in practice | two's-complement wraparound | `err_22_signed_overflow_wraps` | [x] |

## Rows 20 and 21 — how they are tested, and the bug row 20 found

These two rows are the only rejections whose C ground truth **kills the
process** instead of returning a value, so they cannot be compared in-process.
`tests/phase_c_trap.rs` runs each case in a **fresh child process** (the test
binary re-executes itself with `MATHOP_TRAP_SPEC` set) and compares the two
libraries on their **termination signal**, which is the observable behaviour.
A benign-input control case (`err_20_control_benign_input_survives`) and two
must-not-crash cases pin the tests down so they cannot pass for the wrong
reason (e.g. `dlopen` failing in both children).

### Row 20 was a REAL divergence, now fixed

This is the one genuine bug the verification found. The original Rust used
`wrapping_div` / `wrapping_rem`:

| input | C `.so` | Rust `.so` (before) | Rust `.so` (after) |
|-------|---------|---------------------|--------------------|
| `divide_operation(INT_MIN, -1, 0)` | `SIGFPE` (8) | returned `INT_MIN`, exit 0 | `SIGFPE` (8) |
| `modulo_operation(INT_MIN, -1, 0)` | `SIGFPE` (8) | returned `0`, exit 0 | `SIGFPE` (8) |
| `mathop(INT_MIN, -1, 3, 1)` | `SIGFPE` (8) | returned a value, exit 0 | `SIGFPE` (8) |

Crucially this is **reachable from `mathop`**, the only function declared in
`include/lib.h`: `param3 = 3` makes `(param3 % 5) + 1 == 4 == OP_DIVIDE` and
`param3 = 4` selects `OP_MODULO`, with `param1 = INT_MIN` and `param2 = -1`
flowing straight through as the dividend and divisor. The randomized `mathop`
sweep in Phase B hit it and killed the harness, which is how it was found.

The fix replaces `wrapping_div`/`wrapping_rem` with a `cdq; idiv` inline-asm
helper (`c_signed_divrem`) so the Rust emits exactly the instruction the C
compiler emits, and therefore traps identically. Neither alternative matched:
`wrapping_*` returns silently, and Rust's plain `/` / `%` panic, which under
`panic = "abort"` is `SIGABRT` (6), not `SIGFPE` (8).

Inputs that reach this trap are excluded from the in-process sweeps via the
`mathop_traps` / `is_div_trap` predicates in `tests/common/mod.rs` and are
covered exclusively by `phase_c_trap.rs`.

### Row 21 was ALSO a real divergence — but only under the `dev` profile

The release build looked correct, which is exactly why the "all feature
combinations / all profiles" gate matters. Under `cargo build` (dev profile,
`debug-assertions = on`) the results were:

| input | C `.so` | Rust `.so` (before, dev) | Rust `.so` (before, release) | Rust `.so` (after, both) |
|-------|---------|--------------------------|------------------------------|--------------------------|
| `pcwh(1, 2, 1, NULL, &cnt)` | `SIGSEGV` (11) | `SIGABRT` (6) | `SIGSEGV` (11) | `SIGSEGV` (11) |
| `pcwh(1, 2, 1, &hist, NULL)` | `SIGSEGV` (11) | `SIGABRT` (6) | `SIGSEGV` (11) | `SIGSEGV` (11) |
| `pcwh(1, 2, 1, NULL, NULL)` | `SIGSEGV` (11) | `SIGABRT` (6) | `SIGSEGV` (11) | `SIGSEGV` (11) |

Cause: with debug assertions on, rustc wraps raw-pointer dereferences in an
`assert_unsafe_precondition!` null check. It fires *before* the hardware fault,
panics with `null pointer dereference occurred`, and — because the panic crosses
an `extern "C"` boundary — becomes a non-unwinding abort, i.e. `SIGABRT` (6).
The C, which emits a bare `mov`, dies with `SIGSEGV` (11).

Notably `ptr::read_volatile` / `ptr::write_volatile` do **not** avoid this: they
carry the same precondition check. The fix is the `raw` module in
`src/lib.rs`, which performs the four accesses with a single inline-asm `mov`
each, so there is no check under any profile and the fault is the same hardware
fault the C takes.

Five cases now cover the row: four `SIGSEGV` parity cases plus
`null_buffer_recovered`, which proves that a NULL *buffer* behind a *valid*
pointer is **recovered** (allocate + reset count), not an error — the guard is
on `*history`, not on `history`.

## Profile / feature coverage

Every row above was verified under all four build configurations
(`release` × `debug` and `default` × `--no-default-features`) by `verify.sh`.
Two of the three divergences found were profile-sensitive, so a single-profile
run would have missed them.
