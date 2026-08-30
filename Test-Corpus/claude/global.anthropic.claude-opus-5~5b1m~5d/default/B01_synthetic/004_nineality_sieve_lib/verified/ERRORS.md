# ERRORS.md — Error / rejection surface table (Phase C)

## How this table was derived

Mechanical grep of the *entire* C source (`c_src/src/sieve.c`,
`c_src/include/sieve.h`), excluding the license header comment:

```
grep -nE 'return|assert|NULL|errno|exit|abort|RETURN_ERROR|==|!=|if|switch|#if|malloc|free|\[' \
     c_src/src/sieve.c c_src/include/sieve.h
```

Total matches in code (non-comment) lines:

| construct | occurrences | where |
|-----------|-------------|-------|
| `return` statement | **0** | — (`sieve` returns `void`) |
| error-return macro (`RETURN_ERROR`, …) | **0** | — |
| `assert` | **0** | — |
| `NULL` check | **0** | — (no pointer parameters exist) |
| `errno` / `exit` / `abort` | **0** | — |
| explicit range / bounds check | **0** | — |
| min/max constant (`INT_MAX`, `#define …MAX`) | **0** | — |
| enum type | **0** | — |
| array indexing / allocation | **0** | — |
| `if` | **1** | `sieve.c:35` — `if (val % 10 == 9) { break; }` (loop-exit test, **not** an error path) |
| `#if*` | **1** | `sieve.h:24` — `SIEVE_H_` include guard |

**Conclusion: the C library has an empty explicit error surface.** `sieve` has
no return value, no out-parameters, no sentinel, and no rejection branch. It
accepts every one of the 2^32 possible `int` bit patterns. Consequently, the
"same error code / same sentinel" comparison degenerates to "same observable
behaviour (the exact stdout byte stream) and same termination status".

The rows below enumerate every *distinct* way the C code can behave
anomalously or hit an implicit/undefined boundary — i.e. the real
rejection-adjacent surface — plus the generic C-API boundaries the task
mandates covering even when absent from the source.

## The table

| # | function | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|------------------------------------------|-------------------|------|---|
| 1 | `sieve` | No `return`/error code exists anywhere: `void sieve(int)`. Any `int` is accepted. | No error can ever be signalled; only stdout bytes observable. Rust must likewise never reject. | `err_01_no_error_return_channel_exists` | [x] |
| 2 | `sieve` | `val < 0` — C's `%` truncates toward zero, so `val % 10 ∈ {-9,…,0}` and **can never equal 9**. The documented "stops when it ends in 9" contract is violated for all negative inputs. | Does **not** stop at the negative value ending in 9; keeps incrementing through `0` up to `+9`, printing every intermediate value, then stops. Emits `10 - val` lines. | `err_02_negative_never_matches_mod9` | [x] |
| 3 | `sieve` | `val = -9` specifically (input literally "ends in 9" in the human sense, but `-9 % 10 == -9 != 9`). | Prints `-9,-8,…,8,9` = 19 lines. Does *not* stop at `-9`. | `err_03_negative_nine_does_not_terminate_early` | [x] |
| 4 | `sieve` | `val` negative and ending in `0` (`val % 10 == 0`, e.g. `-10`, `-1000`): the "0" remainder case for negatives. | Same as row 2 — runs up to `+9`. | `err_04_negative_multiple_of_ten` | [x] |
| 5 | `sieve` | `val ∈ [INT_MAX-7, INT_MAX] = [2147483640, 2147483647]` — no representable value ≥ `val` ends in 9, so `val++` **signed-overflows**: undefined behaviour in C. The project compiles with no `-O` flag (`CMakeLists.txt` sets none ⇒ `-O0`), and the emitted code is a plain `addl $1, -0x4(%rbp)`, i.e. it *wraps* to `INT_MIN`. | Prints `val … 2147483647`, then wraps and continues from `-2147483648` upward (≈2^31 further lines) until it finally reaches `+9`. Effectively non-terminating for a test. Rust must produce a byte-identical **prefix**. | `err_05_int_max_overflow_wraps` (bounded prefix, child process killed after 1 MiB) | [x] |
| 6 | `sieve` | `val = INT_MAX = 2147483647` exactly (the extreme of row 5; also the widest positive `%d` rendering). | `2147483647\n` then `-2147483648\n`, `-2147483647\n`, … | `err_06_int_max_exact_prefix` | [x] |
| 7 | `sieve` | `val = INT_MIN = -2147483648` (extreme of row 2; `INT_MIN % 10 == -8`; also the only `%d` value whose negation is unrepresentable). | Prints `-2147483648\n-2147483647\n…`, ~2^31 lines up to `+9`. Non-terminating for a test ⇒ prefix comparison. | `err_07_int_min_prefix` | [x] |
| 8 | `sieve` | Out-of-range "enum" value across FFI: the C API declares **no enum**, so the mandated out-of-range-enum probe becomes "an `int` argument with no distinguished meaning". Every 32-bit pattern is a valid `c_int`. | Accepted unconditionally; behaviour determined solely by `val % 10` and sign. Verified for hostile bit patterns (`0x80000000`, `0x7FFFFFFF`, `0xFFFFFFFF`, `0xAAAAAAAA`, …) reinterpreted as `int`. | `err_08_arbitrary_bit_patterns` | [x] |
| 9 | `sieve` | Null pointer / zero length / oversized length: **no pointer or length parameter exists** in the ABI (`void sieve(int)`), so there is no null check to diverge on. Probe instead: the boundary scalars `0`, `-1`, `1`. | `0` ⇒ 10 lines `0..9`; `-1` ⇒ 11 lines `-1..9`; `1` ⇒ 9 lines `1..9`. No rejection. | `err_09_generic_scalar_boundaries` | [x] |
| 10 | `sieve` | Calling convention abuse: extra/garbage in unused argument registers, and calling `sieve` through a mismatched-but-compatible prototype (`int` vs `unsigned` reinterpretation at the boundary). | Only the low 32 bits of `edi` are read (`mov %edi,-0x4(%rbp)`); upper bits of `rdi` ignored. Rust `extern "C" fn(c_int)` must behave identically. | `err_10_upper_register_bits_ignored` | [x] |
| 11 | `sieve` | Repeated invocation / no reset: the C function keeps **no static or global state**, so an "uninitialised/stale state" error class cannot exist. | N-th call is independent; output = concatenation of individual calls. | `err_11_no_hidden_state_between_calls` | [x] |
| 12 | `sieve` | `printf` return value ignored — the C code never checks it, so a write error (e.g. stdout closed / `EBADF` / full pipe) is **silently swallowed** and the loop still runs to completion. | Loop still terminates normally; no error propagated; no crash. Rust must also ignore the `printf` result. | `err_12_ignores_printf_failure_on_closed_stdout` | [x] |

## Notes on rows 5–7 (unbounded loops)

Rows 5, 6 and 7 describe inputs for which the C function performs on the order
of 2^31 `printf` calls (tens of gigabytes of output) before returning. They
cannot be run to completion inside a test budget. They are verified
**differentially but boundedly**: each side is run in a *separate child
process* whose `stdout` is redirected to a file; the parent waits until 1 MiB
has been written, then `SIGKILL`s the child and compares the first 1 MiB of
each side byte-for-byte. Because the byte stream is produced strictly in
order, prefix equality over a 1 MiB window covers the wrap point and the
first ~100 000 post-wrap values, which is exactly the interesting region.

Because row 5 rests on undefined behaviour, the wrap was additionally checked
to be *stable across optimisation levels*, so the Rust `wrapping_add(1)` is
correct no matter how the C is built:

| C build | first bytes of `sieve(INT_MAX)` |
|---------|---------------------------------|
| `-O0` (what `CMakeLists.txt` produces) | `2147483647\n-2147483648\n-2147483647\n…` |
| `-O2` | `2147483647\n-2147483648\n-2147483647\n…` |
| `-O3` | `2147483647\n-2147483648\n-2147483647\n…` |

GCC 11.5 does not exploit the overflow to remove the loop at any of these
levels; it always wraps, matching the Rust translation.
