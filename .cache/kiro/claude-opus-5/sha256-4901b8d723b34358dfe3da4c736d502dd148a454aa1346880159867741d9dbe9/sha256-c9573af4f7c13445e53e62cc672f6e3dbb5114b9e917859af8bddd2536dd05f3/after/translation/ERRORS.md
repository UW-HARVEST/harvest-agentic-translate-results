# ERRORS.md — Error / rejection surface table (Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping every `return`, every
`if (... == NULL)` / `!= NULL`, every conditional guard, every `assert` and
every named constant:

```sh
grep -n 'return'                    c_src/src/lib.c   # 7 hits
grep -n 'assert\|abort\|exit('      c_src/src/lib.c   # 0 hits
grep -n 'NULL\|if (\|switch\|#if'   c_src/src/lib.c   # 21 hits
grep -nE '0[xX][0-9a-fA-F]+|\b0[0-7]+\b|#define' c_src/src/lib.c
```

Findings that shape the table:

* There are **no** `assert`s, no `abort()`/`exit()`, no error enums, and no
  negative sentinel error codes anywhere in the library. `envy` returns an `int`
  whose entire range is a legal result, so "error" here means **the library's
  rejection/fallback branches**: an input is rejected and a documented default or
  backup value is substituted instead.
* The only rejecting function is `parse_env_numeric` (3 rejection branches, each
  returning `default_val`, two of them after a `stderr` warning).
* `init_config_from_env` rejects absent/unsatisfying environment values by
  storing `0` in the corresponding bitfield.
* `envy` has one true fallback branch: `result < 0` restores the `memcpy` backup
  and returns `state.base_value` (i.e. `param1`).
* The only size constant is `BUFFER_SIZE 256`; the only magic values are
  `03`, `0755`, `0x0F`, `0100`, `012`.

Every row below is asserted by a differential test in
`tests/phase_c_errors.rs` that builds that exact condition, calls **both** the C
`.so` and the Rust `.so` through `libloading`, and compares the returned value
**and** the captured `stdout`/`stderr` bytes.

| #  | function | trigger (the exact invalid input/condition) | expected C result | status |
|----|----------|----------------------------------------------|-------------------|--------|
| 1  | `parse_env_numeric` | `getenv(env_name)` returns `NULL` (variable not present in `environ`) — `lib.c:50` | returns `default_val` verbatim; no output written | [x] |
| 2  | `parse_env_numeric` | env value contains `,` (`strchr(env_value, ',') != NULL`) — `lib.c:55` | `fprintf(stderr, "Warning: Invalid character in %s\n", env_name)` then returns `default_val` | [x] |
| 3  | `parse_env_numeric` | env value contains `;` but **no** `,` (`strchr(env_value, ';') != NULL`) — `lib.c:61` | `fprintf(stderr, "Warning: Semicolon found in %s\n", env_name)` then returns `default_val` | [x] |
| 4  | `parse_env_numeric` | env value contains **both** `,` and `;` — comma is checked first | comma branch wins: only the "Invalid character" warning, returns `default_val` | [x] |
| 5  | `parse_env_numeric` | env value is the empty string `""` (not NULL, no `,`, no `;`) — falls through to `lib.c:66` | `atoi("") == 0`; **not** treated as an error, returns `0` not `default_val` | [x] |
| 6  | `parse_env_numeric` | env value is non-numeric junk (`"abc"`, `"++5"`, `"0x1f"`, `" \t-"`) | `atoi` semantics: leading whitespace skipped, parse stops at first non-digit, no error signalled (`0`, `0`, `0`, `0`) | [x] |
| 7  | `parse_env_numeric` | env value has trailing junk after digits (`"12abc"`, `"7 8"`, `"-3xyz"`) | `atoi` returns the leading integer (`12`, `7`, `-3`); no rejection | [x] |
| 8  | `parse_env_numeric` | env value numerically **out of `int` range** (`"2147483648"`, `"-2147483649"`, `"99999999999999999999"`) | UB per ISO C; glibc `atoi` = `strtol` truncated to `int`. Rust must reproduce byte-identically by calling the same libc `atoi` | [x] |
| 9  | `parse_env_numeric` | env value exactly at the `int` boundaries (`"2147483647"`, `"-2147483648"`) | returned unchanged | [x] |
| 10 | `parse_env_numeric` | `default_val` itself is an extreme (`INT_MIN`, `INT_MAX`, `0`, negative) while the variable is unset | the extreme `default_val` is returned unchanged (no clamping) | [x] |
| 11 | `parse_env_numeric` | `env_name` is a name that exists but whose *value* begins with `,`/`;` (rejection at position 0) | still rejected, warning emitted, `default_val` returned | [x] |
| 12 | `parse_env_numeric` | `env_name` is an empty string `""` | glibc `getenv("")` finds no match → `NULL` → returns `default_val`; no output | [x] |
| 13 | `init_config_from_env` | `PROG_VERBOSE` absent (`verbose_env == NULL`) — `lib.c:74` | `flags->verbose = 0` | [x] |
| 14 | `init_config_from_env` | `PROG_VERBOSE` present but contains no `'1'` (e.g. `""`, `"0"`, `"true"`, `"yes"`) | `flags->verbose = 0` — presence alone is **not** enough | [x] |
| 15 | `init_config_from_env` | `PROG_DEBUG` absent, or present without `'1'` — `lib.c:75` | `flags->debug = 0` | [x] |
| 16 | `init_config_from_env` | `PROG_OPTIMIZE` absent (`optimize_env == NULL`) — `lib.c:76` | `flags->optimize = 0`. Note the asymmetry: when *present* the content is **never** inspected, so even `""` and `"0"` set `optimize = 1` | [x] |
| 17 | `init_config_from_env` | called on a `ConfigFlags` whose 4-byte storage is pre-filled with arbitrary garbage | only the six bitfields (low byte) are overwritten; the 3 upper padding bytes are left byte-for-byte untouched | [x] |
| 18 | `perform_operation` / `apply_bit_operations` | **out-of-range "enum" value across the FFI boundary**: the caller supplies any of the 256 low-byte bit patterns for `struct ConfigFlags`, including `log_level` values `4..7` that `init_config_from_env` never produces and the `reserved` bit set. C enums/bitfields accept any int, so these are real inputs | no validation exists; the C masks `log_level` to 3 bits and tests each flag bit for non-zero. All 256 patterns must agree | [x] |
| 19 | `perform_operation` / `apply_bit_operations` | `ConfigFlags` storage with garbage in the **upper 24 padding bits** (e.g. `0xFFFFFF00 \| byte`) | the padding bits are never read; result depends only on the low byte | [x] |
| 20 | `perform_operation` | `optimize == 0` and `log_level == 0`, so the multiply term vanishes: `result = 0*val1 + val2/2` | `val2 / 2` with C truncation-toward-zero (so `-1/2 == 0`, `-3/2 == -1`), including `val2 == INT_MIN` | [x] |
| 21 | `perform_operation` | signed **overflow** of `val1 * log_level` and of the following `+` (e.g. `val1 = INT_MAX`, `log_level = 7`) | ISO C UB; gcc at `-O2` wraps two's-complement. Rust must wrap identically (`wrapping_mul`/`wrapping_add`) | [x] |
| 22 | `apply_bit_operations` | `verbose == 1` and `value` has its top bit set, so `value << 1` **overflows a signed int** (`lib.c:105`) | ISO C UB; gcc wraps. Rust must produce the same two's-complement wrap | [x] |
| 23 | `envy` | computed `result < 0` — `lib.c:171` | discards the computation, `memcpy`s the backup back over `state`, and returns `state.base_value`, i.e. **`param1`** | [x] |
| 24 | `envy` | `param3 == 0` — `lib.c:145` | the `param3 * multiplier` term is skipped entirely (so a huge `PROG_MULTIPLIER` cannot overflow) | [x] |
| 25 | `envy` | `param4 == 0` — `lib.c:149` | the `param4 >> 2` term is skipped | [x] |
| 26 | `envy` | `param4 < 0` (negative right-shift operand, `lib.c:150`) | implementation-defined per ISO C; gcc performs an **arithmetic** shift (`-1 >> 2 == -1`, `INT_MIN >> 2 == -536870912`) | [x] |
| 27 | `envy` | `param1 == INT_MIN` / `INT_MAX`, and the `result < 0` fallback returning `INT_MIN` | fallback returns `param1` unchanged even when it is itself negative | [x] |
| 28 | `envy` | `strchr(buffer, ':')` could be `NULL` — `lib.c:160` | unreachable in practice: `snprintf` always writes `"Result:<n>:Complete"` (max 27 bytes < `BUFFER_SIZE` 256), so a `:` is always at index 6 and `second_colon` is always found. Verified by asserting the `verbose` output line is always `"Found colon at position: 6"` | [x] |
| 29 | `envy` | every environment variable simultaneously invalid (`PROG_BASE_OFFSET=","`, `PROG_MULTIPLIER=";"`) | both `parse_env_numeric` calls reject, defaults `0100`/`012` are used, and **two** warning lines are written to `stderr` in call order (base offset first) | [x] |
| 30 | all five entry points | `NULL` pointer arguments (`env_name == NULL`, `flags == NULL`) | glibc `getenv(NULL)` dereferences its argument and `flags->…` dereferences `NULL`, so the C **crashes with SIGSEGV**. The Rust translation performs the identical raw dereference / identical libc call, so it faults identically. Not asserted in-process (a `SIGSEGV` would kill the harness); verified out-of-process by `tests/phase_c_errors.rs::row30_null_pointer_arguments_fault_identically`, which forks a child per library and compares the termination signal | [x] |

**Rows: 30. Unchecked rows: 0.**

## Supporting suites

* `tests/harness_selfcheck.rs` — negative controls proving the harness can
  actually fail: it asserts that `diff` panics on a return-value divergence and
  on a stdout divergence, that the capture really captures, and that the
  bitfield bit positions the tests encode match the compiled C.
* `tests/ffi_edge_inputs.rs` — generic FFI-boundary inputs beyond the table:
  misaligned `struct ConfigFlags*` at every offset `0..4`, out-of-bounds write
  detection with guard bytes, `env_name` shapes `getenv` treats specially
  (containing `=`, empty, prefix collisions), non-UTF-8 / high-byte env values,
  idempotence over 1000 repeated calls, and **merged** stdout+stderr capture
  (fd 1 and fd 2 pointed at one file) so the interleaving of the block-buffered
  stdout with the unbuffered stderr is compared too.
* `tests/stress_sweeps.rs` — denser sweeps of the same surface (~54M FFI calls),
  including a fully exhaustive 2^20-wide window around `0`, `INT_MIN`,
  `INT_MAX` and the `<< 1` sign-flip points.
