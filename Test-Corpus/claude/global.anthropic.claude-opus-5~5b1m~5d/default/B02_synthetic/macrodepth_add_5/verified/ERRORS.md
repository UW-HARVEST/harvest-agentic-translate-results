# ERRORS.md — error-surface table (Phase A / gate for Phase C)

Mechanically derived from the C sources. Every place the C code rejects,
short-circuits, or otherwise refuses an input gets one row. The complete grep of
`c_src/src/*.{c,h}` for `return` / `if (` / `switch` / `case` / `default` /
`assert` / `NULL` / comparisons yields exactly **two** in-source rejection
points, plus the library-level (`atoi`, `printf %s`) and pointer-level rejections
that the public entry points inherit:

| C location | construct | kind |
|------------|-----------|------|
| `mdmain.c:29-32` | `if (argc < 3) { fprintf(stderr, "usage: %s A B\n", argv[0]); return 2; }` | explicit rejection, sentinel return `2` |
| `mdmacros.h:91` (`DISPATCH_REP`, instantiated by `DEFINE_ACCUM(OP)` → `accum_<OP>`, reached through `use_generated`) | `default: break;` — `switch (n)` only has `case 0..6` | silent rejection: accumulator stays at `INIT_FOR(OP)` |

There are **no** `assert`s, no `return -1`, no `return NULL`, no error enums, no
`errno` use, no min/max constants and no null checks anywhere in the C sources.
There are also **no C enums at all**, so the "out-of-range enum value across
FFI" class degenerates to (a) out-of-range `int` arguments — rows E05–E10 — and
(b) out-of-range values written into the exported `G_OP` / `G_OP_NAME` globals,
which are ordinary writable `.data` objects that accept any bit pattern — rows
E17–E19. `FOR_EACH`/`DO_LOOP` (`mdmacros.h:77-78`) are never instantiated by any
translation unit, so they contribute no reachable behavior.

`OP` below is the build-time operation (`add`/`sub`/`mul`) and
`INIT = INIT_FOR(OP)` (`0` for add/sub, `1` for mul). Every row is checked for
**all 24 `(OP, REPEAT)` configurations**.

| # | function | trigger (the exact invalid input/condition) | expected C result | ✔ |
|---|----------|---------------------------------------------|-------------------|---|
| E01 | `main` | `argc == 2`, valid `argv` (one step below the required 3) | prints `usage: <argv[0]> A B\n` **to stderr**, nothing to stdout, returns `2` | [x] |
| E02 | `main` | `argc == 1` | same as E01, returns `2` | [x] |
| E03 | `main` | `argc == 0` (zero length) | same as E01, returns `2` | [x] |
| E04 | `main` | `argc < 0` (`-1`, `-5`, `INT_MIN`) | same as E01, returns `2` (the check is `argc < 3`, so negatives are rejected too) | [x] |
| E05 | `use_generated` / `accum_<OP>` | `n == 7` — one step past the last `case` (`case 6`) | `default: break` → returns `INIT`, prints `gen.acc=<INIT>\n` | [x] |
| E06 | `use_generated` / `accum_<OP>` | `n == -1` — one step below `case 0` | returns `INIT`, prints `gen.acc=<INIT>\n` | [x] |
| E07 | `use_generated` / `accum_<OP>` | `n == INT_MAX` (oversized) | returns `INIT` | [x] |
| E08 | `use_generated` / `accum_<OP>` | `n == INT_MIN` (oversized negative) | returns `INIT` | [x] |
| E09 | `use_generated` / `accum_<OP>` | `n == 8 … 64`, `n == 1000`, random `n` outside `0..=6` | returns `INIT` | [x] |
| E10 | `use_generated` / `accum_<OP>` | `REPEAT == 7` build: `main` calls `use_generated(7)`, i.e. the in-range unrolled loop and the `switch` disagree | `x3 == INIT` even though `acc` used 7 steps; `summary` reflects that | [x] |
| E11 | `main` | `argv[1]` is not numeric text (`"abc"`, `""`, `"+"`, `"-"`, `"0x10"`, `"1e3"`) | `atoi` returns `0` / the longest numeric prefix; **no** error is reported, returns `0` | [x] |
| E12 | `main` | `argv[1]`/`argv[2]` numerically out of `int` range (`"99999999999999"`, `"-99999999999999"`, `"9223372036854775808"`) | `atoi` = `(int)strtol` → clamp to `LONG_MAX`/`LONG_MIN` then truncate to 32 bits; no error, returns `0` | [x] |
| E13 | `main` | `argv[0] == NULL` while `argc < 3` | glibc `%s` prints the literal `(null)`: `usage: (null) A B\n` on stderr, returns `2` | [x] |
| E14 | `main` | `argv == NULL` while `argc < 3` (null pointer) | dereferences `argv[0]` → **SIGSEGV** | [x] |
| E15 | `main` | `argv == NULL` while `argc >= 3` (null pointer) | dereferences `argv[1]` → **SIGSEGV** | [x] |
| E16 | `main` | `argc >= 3` but `argv[1] == NULL` (null string pointer) | `atoi(NULL)` → **SIGSEGV** | [x] |
| E17 | `main` (via the exported `G_OP` global) | caller stores `NULL` into `G_OP`, then calls `main(3, …)` | `helper_call`/`helper_ptr`/`use_generated` still run and print, then the indirect call through `G_OP` → **SIGSEGV** | [x] |
| E18 | `main` (via the exported `G_OP_NAME` global) | caller stores `NULL` into `G_OP_NAME`, then calls `main(3, …)` | glibc `%s` prints `op=(null) …`, returns `0` | [x] |
| E19 | `main`, `helper_ptr` (via `G_OP`) | caller stores a *different* op (`op_mul`) into `G_OP` | `main`'s `g.call` uses the new pointer; `r_call`, `helper_call`, `helper_ptr` keep using the build-time `OP_FN(OP)` (they are macro-expanded, not global reads) | [x] |
| E20 | `op_add` / `op_sub` / `op_mul` | signed-overflow inputs (`INT_MAX+1`, `INT_MIN-1`, `INT_MIN * -1`, `INT_MIN*INT_MIN`) — no range check exists, so these are accepted | two's-complement wrap-around of the 32-bit result | [x] |
| E21 | `helper_call` | operand values that make `r + acc` overflow (`a = INT_MAX`, `REPEAT > 0`) | wrapped 32-bit sum; `printf` shows the wrapped `r`/`acc` | [x] |
| E22 | `main` | operand values that make `r_call + acc + x1 + x2 + x3 + g` overflow | wrapped 32-bit `summary=`, returns `0` | [x] |
| E23 | `main` (via both globals) | `G_OP` set to each of the three op addresses **and** `G_OP_NAME` set to a caller-owned string of every length class (empty, 1 byte, 10 bytes, 300 bytes) — arbitrary values crossing the FFI boundary into data exports | accepted, no validation: `op=%s` prints the caller's bytes and `g.call` uses the caller's pointer; returns `0` | [x] |
| E24 | all entry points | `stdout`/`stderr` closed (fd 1 and fd 2 unwritable) — `printf`'s return value is ignored by the C code | every function still returns its normal value and the process survives (a `println!`-based translation must not abort here) | [x] |

Every row is exercised by one `#[test]` of the same name in
`translation/tests/errors.rs` (`e01_…` … `e24_…`), run against both `.so`s for
all 42 build configurations. Rows E14–E17 and E24 run the call in a `fork()`ed
child so the expected fatal signal / survival is compared as a `waitpid` status
for both libraries instead of taking the test process down.

Notes on the two divergences these rows caught (both fixed in the Rust):

* E19/C16 — the Rust `helper_ptr` used to read the `G_OP` global, while the C
  `fp` is macro-expanded from `OP_FN(OP)`. With `G_OP` overwritten by the caller
  the two disagreed (`C=1` vs `Rust=-1` for `helper_ptr(0,1)`); confirmed by
  re-introducing the old code and watching `c16`/`c31`/`c40`/`c42` fail.
* E17/C32 — `G_OP`/`G_OP_NAME` were plain Rust `static`s and thus landed in
  `.data.rel.ro` (read-only after RELRO) instead of the C's writable `.data`;
  storing into them killed the process with SIGSEGV. Confirmed by rebuilding
  with `static` instead of `static mut` and watching the suite die at `c16` with
  `signal: 11, SIGSEGV`.
