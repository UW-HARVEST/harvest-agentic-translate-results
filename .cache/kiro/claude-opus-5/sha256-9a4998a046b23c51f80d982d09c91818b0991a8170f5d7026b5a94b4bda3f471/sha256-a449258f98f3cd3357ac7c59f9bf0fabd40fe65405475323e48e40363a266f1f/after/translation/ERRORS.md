# Differential verification: mismatches found and fixed

The C program in `../c_src` is the ground truth. Both programs were built and
run as subprocesses over ~113,000 inputs, comparing stdout, stderr and exit
status byte for byte (`tests/differential.rs`, plus larger throwaway sweeps).

Every mismatch found was a **resource-exhaustion** divergence. The pure
arithmetic/control-flow translation was already faithful: the `& 25` alphabet
mask, the duplicated pop loop in opcode 9, the `case 3:`/`case 5:`
fall-throughs, the unconditional `INT_MIN` return from `process_a_stream`, the
`state_a`/`flipflop` statics persisting across all three engines, `strtol("")`
pushing 0, and the sign-extended `(size_t)k` guard in opcode 6 all matched from
the start and were left alone.

---

## 1. `iv_push` allocation failure: C drops the value, Rust aborted

**Found by** randomized sweep, input:

```
7 2147483647 3 99 0 -3 5 9 -3 -2147483648 6 9 99 2147483647 12 1000 4 1000 14
```

**C** (`rc=0`, 704 MB of stdout, empty stderr):

```
RC:A=99 B=99 EXT=99
A:STACK_TOP=0 STEPS=-2147483647 TRACE=bbbb…   (536870912 = 2^29 letters)
B:STACK_TOP=0 STEPS=-2147483647 TRACE=bbbb…   (134217728 = 2^27 letters)
EXT:STACK_TOP=0 STEPS=-2147483647 TRACE=bb…   ( 33554432 = 2^25 letters)
```

**Rust before the fix** (`rc=-6`, SIGABRT, empty stdout):

```
memory allocation of 4294967296 bytes failed
```

**Cause.** Opcode 7 repeats `DUP` 2147483647 times, so `vm->trace` grows without
bound. In C:

```c
bool iv_reserve(IntVec *v, size_t need){
    ...
    int *p = (int*)realloc(v->data, nc*sizeof(int));
    if (!p) return false;          /* <-- allocation failure is not fatal */
    ...
}
bool iv_push(IntVec *v,int x){
    if(v->len==v->cap && !iv_reserve(v, v->cap? v->cap*2:8)) return false;
    ...
}
void vm_trace(VM *vm, int t){ iv_push(&vm->trace, t); }   /* result discarded */
```

Once `realloc` returns NULL, `iv_push` returns false and the value is **silently
dropped** while the loop keeps running to completion. Every call site in the C
program ignores the `bool`. The translation used `pub type IntVec = Vec<i32>`,
and `Vec::push` *aborts* the process on allocation failure.

The trace lengths being exactly 2^29 / 2^27 / 2^25 confirm the mechanism: they
are the last power-of-two capacity `iv_reserve` managed to allocate. The three
values differ because `vmA`, `vmB` and `vmE` are all live at once, so each
successive engine has less memory available.

**Fix.** `src/util.rs` now implements `IntVec` explicitly: capacity doubles from
8 exactly as `iv_reserve` does, growth uses `Vec::try_reserve_exact`, and `push`
returns `bool`. All call sites discard that `bool` (`let _ = …`) to match C.

**Residual, documented limitation.** *Whether* a given push fails is decided by
the allocator, not by the program. Under an artificial `ulimit -v` the two
binaries start failing at slightly different points, because a Rust process and
a C process have different baseline allocations and allocation patterns. After
the fix both behave the same *way* — silent truncation at a power-of-two
capacity, `rc=0`, empty stderr — and they agree exactly whenever allocation is
not artificially constrained. Byte equality under induced OOM is not achievable
(two C builds with different `MALLOC_` tunables would not agree either).

---

## 2. Opcode 9's VLA overflowing the stack: C segfaults, Rust did not

**Found by** re-reading `engine.c` case 9 for untested paths.

```c
case 9: {
    int m; if(!prog_fetch(&p,&m)) return 10;
    if (m<0 || (size_t)m > vm->stack.len) return 11;
    int tmp[m];                       /* variable-length array */
```

`tmp` is a VLA in `run_engine`'s stack frame, and the pop loops write every
element, so a large `m` walks off the end of the stack. Measured with an 8 MiB
`RLIMIT_STACK` (`7 M 3 9 M` builds a stack of `M` entries, then reduces `M`):

| `m`       | C                          |
| --------- | -------------------------- |
| 1000000   | `rc=0`                     |
| 2093750   | `rc=0` (largest surviving) |
| 2094726   | SIGSEGV, `rc=139`          |
| 3000000   | SIGSEGV, `rc=139`          |

The translation allocated `tmp` with `vec![0; mc]` on the heap, so it **completed
successfully** where C dies.

**Two rounds of fixing were needed.**

1. First attempt: a recursive probe that consumed and touched `4 * m` bytes of
   real stack before the heap allocation. That produced a crash at the right
   threshold, but the *wrong* crash — Rust's std installs a guard-page SIGSEGV
   handler that prints and calls `abort()`:

   ```
   thread 'main' (891628) has overflowed its stack
   fatal runtime error: stack overflow, aborting
   ```

   giving `rc=134` (SIGABRT) and non-empty stderr, where C gives `rc=139`
   (SIGSEGV) and empty stderr.

2. Final approach (`src/stacklimit.rs`): compute the budget explicitly and raise
   a genuine fault. `RLIMIT_STACK` is read from `/proc/self/limits` (avoids a
   libc dependency), the stack base is the address of a local recorded at the top
   of `main`, and remaining space is `limit - (base - sp) - C_FRAME_OVERHEAD`.
   When `4 * m` does not fit, a volatile null write is issued. std's handler only
   claims faults near a guard page; for any other address it restores the default
   disposition and lets the fault re-raise, so the process dies with signal 11
   and an empty stderr — exactly like C.

`C_FRAME_OVERHEAD = 13180` is calibrated from the measurement above (C's largest
surviving VLA is 8375000 bytes against an 8388608-byte limit, i.e. 13608 bytes of
headroom for `main`'s and `run_engine`'s frames, the `alloca` alignment, the
`process_stream` call chain, and the kernel's argv/env block).

Result — thresholds now coincide within C's own measurement band, and the signal
and stderr match on both sides of the boundary:

| binary | largest surviving `m` | first crashing `m` | crash        |
| ------ | --------------------- | ------------------ | ------------ |
| C      | 2093750               | 2094726            | SIGSEGV, 139 |
| Rust   | 2093811               | 2093872            | SIGSEGV, 139 |

The exact boundary is environment-dependent for *both* binaries (it moves with
`ulimit -s` and with the size of the argv/env block), so it is tracked rather
than pinned. `tests/differential.rs` asserts agreement well inside each regime
(`m = 1000000` completes; `m = 3000000` and `m = 8000000` segfault).

---

## Non-mismatch: `argv[0]` in the usage text

`--help` runs `fprintf(stderr, "Usage: %s …", argv[0])`, so each binary correctly
prints its own path. The test harness replaces the binary's own path with
`<PROG>` before comparing; this is the only normalization applied to any output.

---

## Coverage

`tests/differential.rs` (31 tests, all enabled — nothing is `#[ignore]`d) covers,
per branch of the C source:

- **`main`**: no args (`rc=2`, `no program`); `--help` in first/middle/last
  position and after a skipped arg; args that fail `strtol` (`abc`, `0x10`,
  `12abc`, `1e5`, `-`, `+`, `1 2`, `5.5`, bare whitespace, …) producing
  `skip '…'`; the empty argument pushing `0`; `strtol`-accepted forms
  (leading whitespace, `+`/`-`, leading zeros); `LONG_MAX`/`LONG_MIN` clamping
  followed by `(int)` truncation; non-UTF-8 argument bytes.
- **`read_stdin`**: empty and whitespace-only input; `\t`/`\r`/`\n`/space
  delimiters; empty tokens; silently dropped unparsable tokens (no `skip`
  message here); embedded NUL truncating the rest of the chunk; the 4095-byte
  `fgets` boundary splitting a number in half; tokens longer than the buffer;
  `\v`/`\f`, which are `isspace()` for `strtol` but *not* delimiters here;
  arbitrary binary input.
- **Every opcode and every early return**: `rc` 1–11 and 99, `HALT`, opcode 5's
  five trace buckets including the `case 3:` fall-through, opcode 6's
  missing-`k` / failed-pop / out-of-range / negative-`k` / taken / not-taken
  paths, opcode 7's missing-`times` / `ip>=n` / zero-or-negative-`times` /
  inner-failure paths for every possible one-instruction body, opcode 9's
  missing-`m` / negative-`m` / `m>len` / `m==0` / duplicated-pop paths.
- **Cross-cutting**: `state_a` and `flipflop` persisting across all three
  engines; the `& 25` trace-letter mask; the VLA boundary in both regimes.
- **Sweeps**: all 1-opcode programs, all 2-opcode programs over 21 values, all
  3-opcode programs over 13 values, 1500 randomized argv programs, 600
  randomized stdin programs.

Additional throwaway sweeps (not in the suite, to keep it fast): all 4-opcode
programs over 13 values (28,561), 40,000 random 5-opcode programs, 20,000 random
programs of length 1–40, 8,000 negative-operand programs targeting the `code < 0`
branches in `a.c`/`b.c`, 8,000 random stdin payloads, 8,000 random argv
combinations. All passed.

Deliberately excluded from the suite: the 2147483647-iteration input from §1,
which takes C 321 minutes and writes 704 MB.
