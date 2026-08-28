# Differential verification of the Rust translation against `c_src`

The C program is the ground truth. Both programs were built and then run as
subprocesses on the same inputs; **stdout, stderr and exit status were compared
byte for byte / value for value** (the exit status is compared as the raw
`wait(2)` status, so a death by signal would also show up).

```
# C  (ground truth)
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
# Rust
cd translation && cargo build --release                                # -> translation/target/release/driver
# the suite (builds the C binary itself if it is missing)
cd translation && cargo test --release
```

`translation/tests/differential.rs` drives the two **binaries** (never the Rust
code as a library) through `std::process::Command`, with raw-byte `argv` and
raw-byte stdin.

## Result

**One real mismatch was found and fixed** (the `SIGPIPE` exit status, §2 below);
**no mismatch remains in the translated logic.** 24 test functions covering
roughly 5,500 distinct input cases (≈11,000 process launches) all agree — in
both the `--release` and the default (overflow-checked) profile — and additional
throw-away sweeps run during the investigation (≈40,000 more cases, including
40,000 pseudo-random `i32` values folded through `classify`/`process_stream`)
also agree.

Branch enumeration was checked mechanically: a `gcov`-instrumented copy of the
C sources (compiled *outside* `c_src`, which was never modified) reports
**100 % of the 234 executable C lines executed** by the enumerated case list.
The only never-taken branches left are unreachable ones:

* `if (e && ...)` in `main.c` — `strtol` never leaves `e == NULL`;
* `iv_reserve`'s `nc > SIZE_MAX/2` and `!p` (realloc failure) paths;
* `if (acc > 0x7fffffffLL)` in `process_a_stream` — `acc` grows by at most ~21
  per element, so this needs on the order of 10⁸ stream elements (and see trap 3
  below: the *second* clamp fires unconditionally anyway, so the result would be
  `INT_MIN` either way).

## Mismatches actually observed

### 1. `--help` prints `argv[0]`, so the two binaries print their own paths

`usage()` does `fprintf(stderr, "Usage: %s [--stdin] ...", argv[0])`. Run from
their build locations, the C binary prints
`Usage: .../c_src/build/driver ...` and the Rust binary prints
`Usage: .../translation/target/release/driver ...`.

* Cause: the value of `argv[0]`, i.e. where each executable lives — **not** a
  translation defect. There is nothing in the Rust to fix: it faithfully echoes
  `argv[0]`.
* Consequence for testing: a naive comparison of `--help` "fails" for a reason
  that has nothing to do with the translation. The harness therefore forces
  `argv[0]` to the literal `driver` for **both** programs
  (`CommandExt::arg0("driver")`), after which `--help` is byte-identical. This
  was confirmed independently with `exec -a driver ...` in a shell.

### 2. Wrong exit status when stdout is a pipe with no reader (FIXED)

* Symptom: with a program whose output exceeds the 64 KiB pipe buffer and a
  reader that goes away early, e.g.
  `driver 0 1 7 300000 3 | head -c 4`, the **C is killed by `SIGPIPE`**
  (raw wait status 13, shell exit 141) while the Rust build **exited 0**.
  Reproduced with a reader that reads 4 bytes and closes the pipe:
  C `-13`, Rust `0`.
* Cause: the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, and
  every write in the translation discards its `Result` (`let _ = write!(...)`,
  matching C's habit of ignoring `printf`'s return). A C program starts with the
  default `SIGPIPE` disposition and therefore dies. Nothing in the translated
  *logic* was wrong — the difference came from the Rust runtime's startup.
* Fix: `restore_default_sigpipe()` in `src/main.rs` calls
  `signal(SIGPIPE, SIG_DFL)` as the first thing `main` does. Normal runs (stdout
  captured to a file or a fully-read pipe) are unaffected; only the
  closed-reader case changes, and it now matches C exactly.
* Regression test: `stdout_pipe_closed_early_dies_the_same_way`. Verified that
  the test fails (C 13 vs Rust 0) when the fix is reverted, and passes with it.

Apart from §1 (which is `argv[0]`, not behaviour) and §2 (now fixed), no input
produced a byte of difference on stdout or stderr or a different exit status.

## Traps in the C that the translation gets right (checked, not assumed)

Each of these is a place where a plausible "clean-up" of the C would change
observable output. They are called out because they are the mismatches a next
reader should re-check first; every one has a dedicated test.

1. **`engine.c` case 9 pops the stack twice into the same buffer.**
   ```c
   int tmp[m];
   for (int i=m-1;i>=0;i--) iv_pop(&vm->stack, &tmp[i]);
   for (int i=m-1;i>=0;i--) iv_pop(&vm->stack, &tmp[i]);   /* again! */
   ```
   The range check only validates `m <= stack.len`, so the second loop runs off
   an empty stack; `iv_pop` then returns false and **leaves `tmp[i]` holding the
   value written by the first loop**. Up to `2*m` values are consumed.
   Reproduced in `IntVec::pop_into` (which must not touch `*out` when empty).
   Test: `opcode_stream` sweeps stack depth 0…8 × `m` 0…depth+2.
2. **`vm_print` indexes the alphabet with `& 25`, not `% 26`.**
   `"abcdefghijklmnopqrstuvwxyz"[(trace)&25]` maps the trace codes 0…14 onto
   only four letters: `a` (0,2,4,6), `b` (1,3,5,7), `i` (8,10,12,14),
   `j` (9,11,13). Every printed `TRACE=` string therefore uses only `a b i j`.
3. **`process_a_stream` always returns `INT_MIN`.** `acc` is a `size_t`; the
   clamps compare it with `long long` constants, so the usual arithmetic
   conversions make both comparisons *unsigned*: `-0x80000000LL` becomes
   `0xFFFFFFFF80000000`, which is larger than any clamped `acc`, so
   `if (acc < -0x80000000LL) acc = -0x80000000LL;` fires unconditionally and
   `(int)acc` is `-2147483648`. The loop still matters — it mutates `state_a`.
   Test: `stream_sweep_folded`, `wide_random_value_sweeps` (impl `A:` stack tops
   are always `-2147483648`).
4. **Switch fallthroughs.** `case 3:;` before `case 4:` in the bucket switch of
   `engine.c` (buckets 3 *and* 4 trace 8) and `case 5:;` before `case 6:` in
   `a.c`'s `target` (`k` 5 and 6 both return 5).
5. **The inline/macro call wrappers change the argument.**
   `classify` is `call_a_once(x)` for impl 0, `call_b_once(x+1)` for impl 1
   (`MAC_CALL` adds 1) and `target(target(x+1))` for impl 2. Inside `a.c`,
   `a_bias_call` passes `(x ^ 0x55) + 7` and `wrap` passes `x - 5`; inside
   `b.c`, `b_twist_call` passes `((x + 9) ^ 0x2222) - 17` and `w2` passes
   `x + 9`. Off-by-one here would be invisible on many inputs, which is why the
   sweeps fold thousands of values into one printed number.
6. **Three different `target` functions.** `a.c` and `b.c` each define a
   file-static `target` that shadows the global one in `lib.c`; `engine.c` sees
   the `lib.c` one. Their `static` state (`state_a`, `flipflop`) persists for
   the whole process and is shared between `call_*_once` and
   `process_*_stream`, so results depend on how many times each has been called
   (`main` runs the same program three times, once per impl).
7. **`case 6`: `(size_t)k` sign-extends.** A negative `k` becomes a huge
   `size_t`, so `(size_t)k > p.n - p.ip` is true and the opcode returns 7. A
   jump to exactly the end (`k == p.n - p.ip`) is *allowed*.
8. **`case 7` loop semantics.** `times` is fetched first (missing → rc 8), then
   `p.ip >= p.n` (→ rc 9); the body is a **recursive `run_engine` over a
   one-instruction program**, so a body that needs an operand (0, 6, 7, 9) always
   fails and produces trace 12 on the first iteration; `p.ip` ends at
   `saved_ip + 1` either way; `times <= 0` executes nothing.
9. **Order of validation.** `case 6` fetches `k` *before* popping `cond`
   (rc 5 beats rc 6); `case 9` fetches `m` before range-checking it (rc 10 beats
   rc 11); `case 1`/`case 2` pop `b` before `a` and short-circuit.
10. **`iv_peek` defaults:** `-777` in `vm_print`, `0` for opcodes 3/5/8 on an
    empty stack (so `3` on an empty stack pushes 0 and does *not* fail).
11. **`strtol` quirks in `main.c`.** `if (e && *e=='\0')` accepts an **empty
    argument** (no conversion, but `e` already points at the terminator → pushes
    0), accepts leading `isspace` (including `\v` and `\f`) and a sign, rejects
    any trailing character, and on overflow saturates to `LONG_MAX`/`LONG_MIN`
    which the `(int)` cast then truncates (`"9223372036854775808"` → `-1`,
    `"-9223372036854775809"` → `0`, `"4294967296"` → `0`). Rejected arguments
    print `skip '<raw bytes>'` on stderr — including non-UTF-8 bytes, so the
    Rust must handle `argv` as bytes (`OsStrExt`), not `String`.
12. **`fgets`, not `scanf`.** `read_stdin` reads at most 4095 bytes per call, so
    a line longer than that is tokenised in pieces **and a number can be split
    in half**: `"0 3 "×1023 + "0 " + "1234 5\n"` is seen as `… 0 1` then
    `234 5`. Also, the chunk is treated as a C string, so an embedded NUL
    discards the rest of that chunk while the following line is still read.
    Tests: `stdin_fgets_chunk_boundary`, `stdin_tokenising`.
13. **Integer overflow wraps as gcc `-O0` does it.** `add`/`mul` on `INT_MAX`,
    `code<<1` in `a.c`, `x+1` in `MAC_CALL` at `INT_MAX`, and `vm->steps++`
    past `INT_MAX` (verified with `driver 0 5 7 2147483647 10`: both print
    `STEPS=-2147483647`). The Rust uses `wrapping_*` throughout.
14. **Stream ordering / streams used.** All diagnostics (`skip '…'`,
    `no program`, usage) go to stderr, the three `RC:`/`A:`/`B:`/`EXT:` lines to
    stdout; exit codes are 0 (normal and `--help`) and 2 (empty program).

## Remaining divergences, and why they are not fixed

* **`argv[0]`** — see above; inherent to where each binary lives.
* **Allocation failure.** `iv_reserve`/`iv_push` return `false` in C and the
  push is silently dropped; Rust's `Vec` aborts on allocation failure. Only
  reachable under memory exhaustion, and the C behaviour there (a silently
  short program) is not observable otherwise.
* **`int tmp[m]` in `engine.c` case 9 is a VLA**, so the C puts `4*m` bytes on
  the stack while the Rust uses the heap. Measured on this host
  (`ulimit -s` = 8192 KiB) with `driver 0 1 7 <m> 3 9 <m+1>` (opcode 7 builds an
  `m+1`-deep stack cheaply):

  | `m` | C | Rust |
  | --- | --- | --- |
  | 1,000,000 / 2,000,000 / 2,090,000 | exit 0 | exit 0 (identical output) |
  | 2,097,000 / 2,500,000 | killed by `SIGSEGV` | exit 0, prints all four lines |

  Not emulated, deliberately: stack exhaustion is undefined behaviour whose
  threshold depends on `ulimit -s`, on how much stack the caller has already
  used and on the compiler's frame layout. Rust has no `alloca` in `std`, so the
  only way to imitate it would be to guess the limit and raise `SIGSEGV`
  ourselves — which would introduce *new* mismatches near the guessed boundary
  (Rust crashing on inputs where the C succeeds). Reaching this needs a VM stack
  of ≥ ~2.1 million entries. A 100,000-element case is in the suite
  (`large_inputs`) and matches.

## Case classes in `translation/tests/differential.rs`

| test | input class |
| --- | --- |
| `empty_input_no_program` | no argv, `--stdin` with empty/blank/whitespace-only input, all-skipped argv, stdin ignored without the flag → `no program`, exit 2 |
| `single_item_programs` | one bytecode: every opcode alone, i.e. rc 1,2,3,4,5,8,10,99 and the peek-default paths |
| `help_flag` | `--help` alone / before / after bytecodes / after a skip / with `--stdin` (argv[0] normalised) |
| `argument_parsing_strtol_quirks` | 44 argument spellings × (alone, inside a program) |
| `argument_parsing_non_utf8` | raw non-UTF-8 argv bytes echoed by `skip '…'` |
| `stdin_tokenising` | 23 stdin spellings (separators, CRLF, missing trailing newline, NUL, `\v`, unparsable tokens) × 3 argv shapes |
| `stdin_fgets_chunk_boundary` | the 4095-byte `fgets` split, boundaries at 4093…4097, several over-long lines |
| `opcode_push_add_mul_dup_pop` | ops 0–4 incl. `INT_MAX` add/mul wrap, underflows, halt, invalid op |
| `opcode_classify_buckets` | 49 values × {op5, op8, repeated classify} — every bucket of all three impls |
| `opcode_jump` | op 6: missing operand, empty stack, taken/untaken, k = 0/1/2/exact end/past end/negative/`INT_MIN` |
| `opcode_loop` | op 7: missing times/body, times 0/negative/`INT_MIN`, times × 13 body opcodes, loop over halt, back-to-back loops |
| `opcode_stream` | op 9: missing m, m<0, m>len, m huge, plus stack-depth × m × tail matrix (the double-pop) |
| `classify_sweep_folded`, `stream_sweep_folded`, `wide_random_value_sweeps` | ~900 boundary values and 8,000 pseudo-random values folded into one printed result, m = 1…4 |
| `exhaustive_short_programs`, `exhaustive_three_token_programs`, `exhaustive_four_token_programs` | every 1-/2-token program over 13 symbols, every 3-token program over 11 opcodes, every 4-token program over the 7 operand-consuming opcodes |
| `randomised_programs`, `randomised_programs_via_stdin` | 400 + 300 deterministic pseudo-random programs, incl. argv/stdin splits |
| `deep_valid_programs` | 24 long programs constructed to avoid every error path, so they run to the last instruction: ~700 steps and ~650 trace characters per run, all of which depend on every intermediate value |
| `loop_step_counter_at_scale`, `large_inputs` | 5M-iteration loop, 20,000 argv bytecodes, 10,000 stdin bytecodes, 600-deep stack, 100,000-element VLA stream |
| `stdout_pipe_closed_early_dies_the_same_way` | stdout is a pipe whose reader closes after 4 bytes → both must die from `SIGPIPE` (regression test for §2) |

No test is `#[ignore]`d, skipped or disabled.
