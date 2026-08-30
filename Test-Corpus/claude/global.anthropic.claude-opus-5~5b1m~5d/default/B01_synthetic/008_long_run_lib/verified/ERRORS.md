# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every error-ish construct in the whole C tree was grepped for:

```
$ grep -n 'return\|assert\|NULL\|errno\|exit\|abort\|if *(\|else\|switch\|case\|#if\|<=\|>=\|!=\|==' \
      c_src/src/long.c c_src/include/long.h
c_src/src/long.c:29:#define ARRAY_SIZE (256 * 1024) // 1MB assuming sizeof(int) = 4
c_src/src/long.c:66:    return;
c_src/include/long.h:24:#ifndef ECHO_H_
```

Results of the sweep:

* error-return macros (`RETURN_ERROR`, …): **0**
* `return <error value>` statements: **0** — the only `return` is the bare
  `return;` at `long.c:66`, falling off the end of a `void` function
* `return NULL` / null-pointer checks: **0** — neither public function takes a
  pointer argument, so there is no pointer to validate
* `assert` / `abort` / `exit` / `errno` use: **0**
* explicit range checks, `if`/`switch`/`else`: **0** — the only conditionals in
  the file are the three `for` loop bounds (`i < ARRAY_SIZE`, `j < 100`,
  `i < ITERATIONS`)
* error enums / status codes: **0** — both public functions return `void`
* `#ifdef` feature branches: **0** (the only `#if` is the `ECHO_H_` include guard)

**Conclusion: `liblong` has no error surface in the conventional sense.** Both
public functions are infallible-by-signature (`void` return), accept no
pointers, and validate nothing. Every possible input is "accepted".

That is precisely why this table must be written anyway: the rejection surface
being empty is itself a behavioural contract the Rust must reproduce. The Rust
must **not** invent rejections that the C does not perform — no bounds-check
panic, no overflow panic, no assertion, no early return. A Rust `panic!` where
C silently computes a wrapped value is a divergence of exactly the kind this
table exists to catch. Under `panic = "abort"` (set in `Cargo.toml`) any such
panic is an immediate `SIGABRT` of the calling process, which is trivially
distinguishable from the C behaviour.

Rows below therefore enumerate, one per distinct condition, every input or
state that *could plausibly* be rejected — including the implicit-UB conditions
the C executes anyway, the loop-bound constants, and the generic FFI boundaries
required by the task (null pointers, zero/oversized lengths, one-past-range
values, out-of-range enum values).

## Table

Legend for "expected C result": `NO-REJECT` = C accepts and completes normally,
returning `void` and leaving well-defined state. `N/A-by-signature` = the
condition is not expressible through this entry point's ABI.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `long_exec` | `seed = 0` (boundary low of `unsigned int`) | `NO-REJECT`: `srand(0)`; glibc treats seed 0 like seed 1; prints an `int` + `\n` | [x] |
| 2 | `long_exec` | `seed = UINT_MAX` (`4294967295`, boundary high, one past `INT_MAX` positive range) | `NO-REJECT`: no truncation/rejection, `srand` takes the full 32-bit value | [x] |
| 3 | `long_exec` | `seed = 0x80000000` (`2147483648`) — first value that is negative when reinterpreted as `int`; catches a signed/unsigned mix-up in the FFI signature | `NO-REJECT`: identical stream to `srand((unsigned)INT_MIN)` | [x] |
| 4 | `long_exec` | `seed` passed as a *negative* `int` from the caller (`-1`), i.e. an out-of-range value for the declared `unsigned int` parameter | `NO-REJECT`: two's-complement reinterpretation to `4294967295`; must equal row 2 exactly | [x] |
| 5 | `long_exec` | out-of-range "enum-like" `int` passed across FFI: `seed` = value with no distinguished meaning (`0x7FFFFFFF`, `0xFFFFFFFE`, `12345678`). C enums/`unsigned int` accept any bit pattern; there is no valid-variant table to fall outside of | `NO-REJECT`: every one of the 2^32 bit patterns is a legal seed; no default/fallback branch exists | [x] |
| 6 | `long_exec` | extra/garbage arguments supplied by the caller (`long_exec(seed, junk...)`) — the C prototype is `(unsigned int)` and is called through a mismatched pointer type | `NO-REJECT`: SysV AMD64 ignores surplus register args; only `edi` is read | [x] |
| 7 | `perform_expensive_operations` | called with a *null* pointer argument, i.e. the "null check" boundary. The C prototype `void perform_expensive_operations()` takes **no** parameters and dereferences no caller pointer — it only touches the module-global `array` | `N/A-by-signature`, and therefore `NO-REJECT`: passing junk args changes nothing | [x] |
| 8 | `perform_expensive_operations` | called **before** `long_exec`, i.e. on the zero-initialised `.bss` `array` ("zero length / uninitialised state" boundary) | `NO-REJECT`: no lazy-init guard, no "not seeded" error — the worker just runs. Ground truth measured from gcc: `0` is **not** a fixed point (`step(0) == -3`), so one call maps every element to `-626538949` | [x] |
| 9 | `perform_expensive_operations` | called repeatedly with no re-seeding (0, 1, 2, 3, … 40 back-to-back calls) — no call-count limit or one-shot guard exists | `NO-REJECT`: pure idempotent-composition, `f^(100n)` applied elementwise | [x] |
| 10 | `perform_expensive_operations` | `array` element `= INT_MAX` — `x * 3 + 7` **signed overflow (UB)**; the only "range check" a defensive implementation would add | `NO-REJECT`: gcc emits `imul`/`add`; result wraps mod 2^32. Rust must use `wrapping_*`, must **not** panic | [x] |
| 11 | `perform_expensive_operations` | `array` element `= INT_MIN` — `x * 3 + 7` overflow **and** `x / 2` at the most-negative value **and** `x % 7` with negative dividend, all in one input | `NO-REJECT`: `INT_MIN/2 == -1073741824` (truncation toward zero), `INT_MIN % 7 == -2` (verified against gcc); no `SIGFPE`, no panic | [x] |
| 12 | `perform_expensive_operations` | `array` element negative and `x << 1` overflows the sign bit (e.g. `INT_MIN`, `-1073741825`, `0x40000000`) — left-shift of a negative / overflowing signed value is UB | `NO-REJECT`: gcc emits a plain `shl`; must match `((x as u32) << 1) as i32` | [x] |
| 13 | `perform_expensive_operations` | `array` element negative feeding `x >> 3` — right shift of a negative signed value is *implementation-defined*, not an error | `NO-REJECT`: gcc emits `sar` (arithmetic, sign-extending). Rust `i32 >> 3` must match, **not** a logical shift | [x] |
| 14 | `perform_expensive_operations` | divisor boundary: is `x / 2` or `x % 7` ever a division by zero (`SIGFPE`)? Both divisors are non-zero literals | `NO-REJECT`: unreachable by construction; no divide-by-zero possible for any of the 2^32 inputs | [x] |
| 15 | `perform_expensive_operations` | index boundary: `for (size_t i = 0; i < ARRAY_SIZE; i++)` at `i = ARRAY_SIZE - 1` (last in-bounds) and the loop's refusal to touch `i = ARRAY_SIZE` (`262144`, one past the documented valid range) | `NO-REJECT`: exactly `[0, 262144)` written; byte `array[262144]` (one past the end) is never modified — verified by a guard-canary test | [x] |
| 16 | `perform_expensive_operations` | "oversized length": caller writes past `array`'s 1 MiB `st_size` and expects the callee to clamp | `NO-REJECT`: no length parameter exists; the size is the compile-time constant `ARRAY_SIZE`. Both `.so`s must publish the identical `st_size` (`0x100000`) so the same bytes are in range | [x] |
| 17 | `long_exec` | reentrancy / state boundary: `long_exec` called a second time in the same process after `array` was left dirty by row 9 | `NO-REJECT`: the `array[i] = rand()` loop fully overwrites all state first, so output depends only on `seed`, never on prior `array` contents | [x] |
| 18 | `long_exec` | `stdout` unavailable/closed when `printf("%d\n", …)` runs — the one library call whose return value the C **discards** | `NO-REJECT`: return of `printf` is ignored; no error propagation to the caller either way | [x] |
| 19 | `perform_expensive_operations` / `long_exec` | the `int xor_result = 0; xor_result ^= array[i]` accumulator overflowing | `NO-REJECT`: `^` on `int` cannot overflow/trap; sign bit is just another bit | [x] |

All 19 rows are covered by `tests/error_paths.rs`; each test asserts the C and
Rust `.so`s agree on the *same* outcome (same array bytes / same printed text /
both completing without a signal), not merely that "both did something".
