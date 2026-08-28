# ERRORS.md — Phase C error-surface table

Mechanically derived by grepping the **entire** C source for every rejection
mechanism. Commands run and their complete results:

```sh
cd c_src
grep -rn "return" src include
#   src/hello.c:30:    return 0;          <-- the ONLY return in the library

grep -rniE "RETURN_ERROR|assert|NULL|errno|return *-|E[A-Z]{3,}|enum|\
#ifdef|#if |switch|if *\(|for *\(|while *\(" src include
#   only license-comment lines, the HELLO_H_ include guard, and
#   `int helloworld() {` / `printf(...)` / `return 0;`

grep -rnE "\*|MAX|MIN|size_t|len|\[|\]" src include   # (excluding comments)
#   (NONE FOUND)
```

## Findings (the absence is the finding)

The complete library is:

```c
int helloworld() {
    printf("Hello World!\n");
    return 0;
}
```

* **0** error-return macros (`RETURN_ERROR` &c.) — the macro does not exist.
* **0** `return -1` / `return NULL` / error-enum returns. The single `return`
  statement is the unconditional `return 0`.
* **0** `assert`s.
* **0** `if` / `switch` / loop statements — the function is entirely branchless.
* **0** parameters ⇒ no pointer parameters, no length parameters, no enum
  parameters, no range checks, and no `MIN`/`MAX` constants.
* `printf`'s return value is **discarded**; the function returns `0` even when
  the write fails. Replicating this "swallow the I/O error" behaviour is the
  real error-path contract of this library.

So the error surface consists of the conditions under which the *only*
fallible operation (`printf`, lowered to `puts`) fails, plus the ABI-level
boundaries that exist for every C entry point. One row per distinct condition.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| E1 | `helloworld` | fd 1 **closed** before the call, stdout unbuffered ⇒ `puts` → `write(1,…)` fails `EBADF` | returns `0`; emits no bytes; stdout error indicator (`ferror`) set | `e1_fd1_closed_write_fails` | [x] |
| E2 | `helloworld` | fd 1 is a **read-only** descriptor (file opened `O_RDONLY`), stdout unbuffered ⇒ `write` fails `EBADF` | returns `0`; emits no bytes; `ferror` set | `e2_fd1_read_only_write_fails` | [x] |
| E3 | `helloworld` | fd 1 is the write end of a **pipe whose read end is closed**, stdout unbuffered ⇒ `write` fails `EPIPE` (SIGPIPE ignored) | returns `0`; `ferror` set | `e3_fd1_broken_pipe` | [x] |
| E4 | `helloworld` | stdout's **error indicator already set** before the call (no `clearerr`) — glibc refuses further output on the stream | returns `0`; C performs no check and reports nothing | `e4_error_flag_already_set` | [x] |
| E5 | `helloworld` | fd 1 is a **directory** fd (write always fails `EBADF`), stdout unbuffered | returns `0`; `ferror` set | `e5_fd1_is_directory` | [x] |
| E6 | `helloworld` | **error latching across a good → bad → good sequence** of calls: call 1 writes to a valid file, then fd 1 is closed under the stream and call 2 fails, then a valid fd is restored for call 3 while the stream's error flag is still latched | all three calls return `0`; `ferror` progression `false, true, true`; identical bytes reach the file | `e6_error_latching_across_calls` | [x] |
| E7 | `helloworld` | called through an **unprototyped (K&R) signature with extra arguments** — `int helloworld();` accepts any arity in C, so garbage in `rdi/rsi/rdx/rcx/xmm0` is a real input the C tolerates. This is the zero-parameter analogue of "out-of-range enum value across the FFI boundary": values with no valid meaning arrive in the argument registers and must be ignored identically. Driven with a seeded set of extreme `int` values (`0`, `-1`, `i32::MIN`, `i32::MAX`, random) | returns `0`; prints `Hello World!\n`; arguments ignored | `e7_extra_arguments_unprototyped` | [x] |
| E8 | `helloworld` | called with a **variadic** call signature (`extern "C" fn(c_int, ...)`) — another arity/ABI mismatch an external caller can produce through the unprototyped header (sets `al` to the SSE-register count) | returns `0`; prints `Hello World!\n` | `e8_variadic_call_signature` | [x] |

## Generic C-API boundaries: applicability

The task list requires covering null pointers, zero/oversized lengths,
one-past-range values, and out-of-range enum values. `helloworld` has **no
parameters at all** (`c_src/include/hello.h:27`), so these classes are
structurally inapplicable rather than untested. Recorded explicitly:

| generic boundary class | applicable? | why | covered by |
|------------------------|-------------|-----|-----------|
| null pointer argument | no | no pointer parameter exists in the API | — (E7/E8 pass garbage in the register a pointer would occupy, incl. `0` = `NULL` and `-1`) |
| zero length | no | no length/size parameter exists | — (E7 passes `0`) |
| oversized length | no | no length/size parameter exists | — (E7 passes `i32::MAX`, `u64::MAX`) |
| one past a documented valid range | no | no parameter has a documented range | — (E7 passes `i32::MIN`/`i32::MAX`) |
| out-of-range enum value | no | no enum parameter exists | E7/E8 — the ABI-level equivalent: meaningless ints crossing the FFI boundary |
| out parameter / return-buffer overflow | no | no out parameters; return type is a plain `int` | — |
| double-free / use-after-free of a handle | no | the API allocates nothing and returns no handle | — |
| uninitialised-context / wrong-order calls | no | the library holds no state; there is no init/destroy pair | B14 (idempotence over many calls) |

**All 8 applicable rows have a passing differential test — see
`tests/phase_c.rs` (harness in `tests/common/mod.rs`) and the evidence below.**

## Verification evidence

`./verify.sh` (debug and release × default / `--no-default-features` /
`--all-features`):

```
tests/phase_c.rs — test result: ok. 9 passed; 0 failed
  e1_fd1_closed_write_fails                    ... ok   (ret 0, ferror set, errno EBADF)
  e2_fd1_read_only_write_fails                 ... ok   (ret 0, ferror set, errno EBADF)
  e3_fd1_broken_pipe                           ... ok   (ret 0, ferror set, errno EPIPE)
  e4_error_flag_already_set                     ... ok
  e5_fd1_is_directory                           ... ok   (ret 0, ferror set, errno EBADF)
  e6_error_latching_across_calls                ... ok   (rets [0,0,0], ferror [f,t,t])
  e7_extra_arguments_unprototyped               ... ok   (5-arg, 8-arg, hostile)
  e8_variadic_call_signature                    ... ok   (happy + hostile)
  generic_boundaries_have_no_applicable_surface ... ok
```

Every row asserts the *same specific* outcome from both `.so`s — identical
return value, identical `ferror(stdout)` state and identical `errno` — not merely
"both failed somehow".

Two Phase C mutants confirm these tests have teeth (`./mutation_test.sh`):
making the Rust return `-1` on a failed write is caught by 7 tests, and making it
`panic!` on a failed write aborts the process. Both are KILLED.
