# ERRORS.md — Error-surface table (Phase A → gates Phase C)

Mechanically derived from every rejection / early-return / sentinel /
implicit-failure site in `c_src/src/lib.c`. There are no `assert`s, no error
enums, no `RETURN_ERROR`-style macros and no `#define`d limits in this file, so
the table is built from every `return NULL`, every `return -1`, every
"fall-through `return 0`", every null-guard `if`, and every unguarded
dereference (whose "expected result" is a fault that both sides must reproduce).

Line numbers refer to `c_src/src/lib.c`.

| # | function | trigger (exact invalid input / condition) | expected C result | test |
|---|----------|-------------------------------------------|-------------------|------|
| E1 | `create_buffer` | L36 `if (!buffer)` — `malloc(sizeof(StringBuffer))` (16 B) returns NULL | returns `NULL` | not forceable without an allocator hook; covered indirectly by E2 (same sentinel). Documented, see notes. |
| E2 | `create_buffer` | L41 `if (!buffer->data)` — `malloc(initial_capacity)` fails because `initial_capacity < 0` sign-extends to a huge `size_t` (`-1`, `-2`, `INT_MIN`, `-32`, random negatives) | frees the struct, returns `NULL` | `e2_create_buffer_negative_capacity_returns_null` |
| E3 | `create_buffer` | L41 same guard, but with a huge *positive* capacity that glibc refuses (`INT_MAX`, `INT_MAX-1`, `2^30·2`…) | returns `NULL` **iff** the host `malloc` fails; C and Rust must agree bit-for-bit on the same host | `e3_create_buffer_huge_positive_capacity_agrees` |
| E4 | `create_buffer` | `initial_capacity == 0` → `malloc(0)` succeeds (glibc) then L48 `buffer->data[0] = '\0'` is a 1-byte heap **overflow** (C UB, benign on glibc) | returns non-NULL with `capacity == 0`, `length == 0`; the OOB store happens | `e4_create_buffer_zero_capacity` |
| E5 | `append_to_buffer` | L61 `if (!new_data)` — `realloc` fails. Reached by forcing `new_capacity = required_capacity * 2` to overflow `int` and become negative (e.g. `buffer->length = 1_500_000_000`, 1-byte string ⇒ `required = 1500000002`, `*2` wraps to `-1294967292`, sign-extends to ~1.8e19 bytes) | returns `-1`, buffer left **unmodified** (`data`, `capacity`, `length` unchanged) | `e5_append_realloc_failure_returns_minus_1` |
| E6 | `append_to_buffer` | Same site, but with a huge-but-**positive** `new_capacity`: `length = 1_073_741_820` + 2-byte string ⇒ `required = 1_073_741_823`, `*2 = 2_147_483_646` (no overflow) ⇒ `realloc` to ~2 GiB | host dependent. **Measured on this host: `realloc` SUCCEEDS** — `ret = 0`, `capacity = 2147483646`, `length = 1073741822`, and the 3 `strcpy`'d bytes land at offset 1073741820. C and Rust agree exactly (incl. the `size_t` conversion of a large positive `int`) | `e6_append_realloc_huge_positive_agrees` |
| E7 | `append_to_buffer` | `buffer == NULL` — **no null check** at L54/L55, `buffer->length` is dereferenced | SIGSEGV (C UB) | not differentially testable (both crash the process); asserted structurally instead: the Rust body has no null guard either — see notes |
| E8 | `append_to_buffer` | `str == NULL` — `strlen(NULL)` at L54 | SIGSEGV (C UB) | as E7 |
| E9 | `destroy_buffer` | L76 `if (buffer)` false — `buffer == NULL` | no-op, returns cleanly (must **not** crash) | `e9_destroy_buffer_null_is_noop` |
| E10 | `destroy_buffer` | L77 `if (buffer->data)` false — non-NULL struct whose `data` is `NULL` | skips `free(data)`, still `free`s the struct, returns cleanly | `e10_destroy_buffer_null_data` (note: the guard is *unobservable* — `free(NULL)` is itself a no-op — so removing it is a provably equivalent mutant; the Rust keeps it verbatim anyway) |
| E11 | `get_operation_name` | L90 `default:` — `op_code` outside `0..=3`. This is the "invalid enum value across FFI" case: the C `switch` accepts any `int`. Trigger with `4`, `-1`, `-2`, `-3`, `-4`, `5`, `INT_MIN`, `INT_MAX`, and randomized ints | returns the literal `"unknown"` | `e11_get_operation_name_out_of_range` |
| E12 | `perform_operation` | L107 final `return 0` — `operation` matches none of `"add"`/`"subtract"`/`"multiply"`/`"divide"`. Trigger with `""`, `"unknown"`, `"Add"` (case), `"add "`/`" add"` (whitespace), `"ad"`/`"adds"` (prefix/superstring), `"divide\0x"`, non-ASCII / high-bit bytes, 1 KiB junk | returns `0` (regardless of `a`, `b`) | `e12_perform_operation_unknown_op` |
| E13 | `perform_operation` | L105 `return 0` — `operation == "divide"` **and** `b == 0` | returns `0` (no division performed, no SIGFPE) | `e13_perform_operation_divide_by_zero` |
| E14 | `perform_operation` | L103 `a / b` with `a == INT_MIN && b == -1` | SIGFPE (`#DE`) — C UB, no defined value | not differentially testable (C crashes); Rust uses `wrapping_div` → `INT_MIN`. Excluded from randomized ranges. See notes. |
| E15 | `perform_operation` | `operation == NULL` — `strcmp(NULL, "add")` at L95 | SIGSEGV (C UB) | as E7 |
| E16 | `buffapp` | L112/L116 — `create_buffer(32)` returns NULL and `log_buffer->length = 0` dereferences NULL (**no** check) | SIGSEGV | unreachable in practice (32-byte `malloc` cannot be made to fail from the API); the Rust mirrors the missing check verbatim |
| E17 | `buffapp` | L119/L123/… — `append_to_buffer`'s `-1` return is **ignored** every time | log silently truncated/unchanged, `buffapp` still returns the computed `result` | unreachable from the public API (capacities stay small); mirrored verbatim in Rust |
| E18 | `buffapp` | L141 `if (intermediate3 != 0)` false → L144 fallback `result = param1+param2+param3+param4` (signed overflow wraps) | returns the wrapped 4-way sum instead of the quotient | `e18_buffapp_intermediate3_zero_fallback` (also the `buffapp` rows C38–C97 in `CONFIGS.md`) |
| E19 | `buffapp` | `param1 % 4` / `param3 % 4` **negative** (C `%` truncates toward zero ⇒ `-1/-2/-3` for negative params) → `get_operation_name` hits `default` ⇒ op is `"unknown"` ⇒ `perform_operation` returns `0` | intermediate is `0`; log prints `unknown(...)` | `e19_buffapp_negative_modulo_unknown_op` |

## Notes on the non-differentiable rows (E1, E7, E8, E14, E15, E16, E17)

These are rows where the C program's "result" is a process-fatal fault
(SIGSEGV/SIGFPE) or an unreachable allocator failure. Calling them in a
differential test would abort the test process rather than compare anything, so
they are **verified by inspection of the Rust source instead**, with the rule
"reproduce, do not fix":

* `append_to_buffer` (Rust) dereferences `(*buffer).length` with no null check
  and calls `strlen(str_)` unconditionally → same fault as E7/E8.
* `perform_operation` (Rust) calls `strcmp(operation, …)` unconditionally → E15.
* `buffapp` (Rust) writes `(*log_buffer).length = 0` with no null check and
  discards every `append_to_buffer` result → E16/E17.
* E1 shares its observable sentinel (`NULL`) with E2/E3, which *are* tested.
* E14 is the single knowingly-divergent case: C traps, Rust returns `INT_MIN`.
  Since C has no defined behaviour here there is nothing to match, and the
  randomized generators in Phase B/C explicitly exclude
  `(a, b) == (INT_MIN, -1)` for `"divide"`. `buffapp` can never reach it:
  `"divide"` requires `p % 4 == 3`, and `INT_MIN % 4 == 0`; the later
  `result / intermediate3` needs `intermediate3 == -1`, which forces
  `{i1,i2} == {1,-1}` and hence `result == 0`, not `INT_MIN`.

## Harness self-validation (mutation testing)

An error table is only as good as the tests that read it, so the Phase B + C
suites were validated by injecting 16 deliberate faults into `src/lib.rs`,
rebuilding, and re-running both suites. **14/16 were caught.** The two
survivors are provably *equivalent* mutants, i.e. no possible test can observe
them:

| injected fault | caught by |
|---|---|
| `param1 % 4` → `rem_euclid` | 25 Phase-B cells + E18/E19/residue-boundaries |
| `param3 % 4` → `rem_euclid` | 25 Phase-B cells + E18/E19/residue-boundaries |
| `create_buffer`: clamp negative capacity to 0 | **E2**, generic capacity band |
| `append`: drop `size_t` sign-extension of `new_capacity` | **E5** |
| `append`: `required > capacity` → `>=` | C13/C15 + boundary sweep + C7/C8-C10 |
| `append`: growth factor `*2` → `*3` | C14/C16/C17 + boundary sweep + chains |
| `append`: error sentinel `-1` → `-2` | **E5** |
| `perform_operation`: `wrapping_div` → `div_euclid` | C33/C35/C36/C37, **E12b**, generic extremes |
| `perform_operation`: `add`/`subtract` bodies swapped | C30 + 22 more |
| `get_operation_name`: `case 3` made unreachable | C28/C29, **E11**, generic op-code band |
| `get_operation_name`: `default` → `""` | C29 + 36 more, **E11**, **E19** |
| `buffapp`: `if intermediate3 != 0` → `== 0` | process aborts (divide-by-zero) |
| `create_buffer`: omit `data[0] = '\0'` | C1–C5, **E4**, **E9**, generic band |
| `buffapp`: `"Final result: "` → `"Final Result: "` | 59 Phase-B cells + E18/E19 (stdout bytes) |
| `destroy_buffer`: remove the `data != NULL` guard | *survives* — `free(NULL)` is a no-op, so the guard has no observable effect |
| `buffapp`: `create_buffer(32)` → `create_buffer(33)` | *survives* — the log buffer's capacity is never exposed; the printed string is content-determined and identical either way |

## Cross-compiler confirmation

Because `lib.c` contains signed-overflow UB (`a + b`, `a * b`,
`length + str_len + 1`, `required * 2`), the Rust's `wrapping_*` choices were
checked against **four** independently built C references, all byte-identical:

| C reference | result |
|---|---|
| `cmake` default (gcc, no `-O`) — the ground truth per `CMakeLists.txt` | 98 + 19 cases pass |
| `gcc -O2` | 98 + 19 cases pass |
| `gcc -O3` | 98 + 19 cases pass |
| `clang -O2` | 98 + 19 cases pass |

(Set `HARVEST_C_SO=/path/to/libX.so` to re-run against any other build.)
