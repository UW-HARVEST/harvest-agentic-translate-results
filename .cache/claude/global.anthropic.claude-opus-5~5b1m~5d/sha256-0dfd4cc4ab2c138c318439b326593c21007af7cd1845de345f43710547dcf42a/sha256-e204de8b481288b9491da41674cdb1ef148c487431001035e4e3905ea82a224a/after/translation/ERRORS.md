# Differential verification log (C ↔ Rust)

Ground truth: `c_src/` (never modified — source checksums unchanged).
Subject: the `driver` binary built from `translation/`.

## How both programs were run

```
# C  (Phase A)
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
# Rust (Phase A)
cd translation && cargo build --release                                 # -> translation/target/release/driver
```

`usage()` in `main.c` prints `argv[0]`, so a comparison is only meaningful when
both processes are invoked under the same name. Both binaries are therefore
staged as `<dir>/driver` and started as `./driver` with the matching working
directory (`translation/tests/differential.rs`, `harness()`).

## Result

**No output mismatch was found.** Over the whole campaign — 31 `cargo test`
cases (each covering many inputs) plus ~10,500 extra ad-hoc differential inputs
run against the C program built both unoptimised (CMake default) and with
`gcc -O2` — stdout, stderr and the exit status were byte-identical for every
input.

Two problems *were* found, both in the test harness rather than in the
translation; they are recorded here because they make a naive comparison look
wrong:

| # | Symptom | Cause | Fix |
|---|---------|-------|-----|
| 1 | `--help` output differed between the two programs | `usage()` prints `argv[0]`, and the two binaries live at different paths | stage both as `./driver` and run them with the matching `current_dir`, so `argv[0]` is the same string |
| 2 | A randomly generated program hung both programs (e.g. `7 1104791398 8`) | opcode 7 is a `for (i=0;i<times;i++)` loop with an attacker-supplied 32-bit count; the C program is just as slow | the random generators keep LOOP counts small; the LOOP paths are covered explicitly instead |

`translation/tests/differential.rs` compares stdout, stderr **and** the exit
status (plus the terminating signal on unix) for every case; no test is
`#[ignore]`d, skipped or disabled.

## C behaviours that had to be reproduced (all verified as already correct)

These are the places where a "reasonable" Rust rewrite would have diverged.
Each is asserted by at least one test.

1. **`process_a_stream` always returns `INT_MIN`** (`a.c:60`). `acc` is a
   `size_t`; the final clamp `if (acc < -0x80000000LL) acc = -0x80000000LL;`
   compares `unsigned long` against `long long`, so the constant is converted
   to `0xFFFFFFFF80000000` and the branch is *always* taken. The visible result
   is `A:STACK_TOP=-2147483648` after every opcode 9. `impl_a.rs` reproduces
   this with `u64` arithmetic instead of "fixing" the clamp. (`stream_opcode`,
   `stream_over_many_stack_depths`)
2. **`target`'s side effects still happen** even though the return value of
   `process_a_stream` is constant: `state_a` (a.c) and `flipflop` (b.c) are
   file-scope statics that persist for the whole process and are mutated by
   every `classify`/`process_*_stream` call, so later opcodes see the drifted
   state. (`state_is_shared_between_calls`)
3. **`a.c` and `b.c` each define their own `static target`**, shadowing the
   external `target` from `lib.c` only inside their translation unit; `engine.c`
   sees the `lib.c` one. Hence the three different `EXT:`/`A:`/`B:` results.
4. **`classify` argument fiddling**: impl 0 is `call_a_once(x)`, impl 1 is
   `call_b_once(x + 1)` (the `MAC_CALL` macro), impl 2 is
   `target(target(x + 1))`. `x + 1` is allowed to overflow (`i32::MAX` input).
5. **Opcode 9 pops twice into the same buffer** (`engine.c:148-149`). The bounds
   check only covers the first round, so the second round's `iv_pop` calls fail
   silently once the stack runs dry and leave the first round's values in
   `tmp`. `engine.rs` keeps both loops and ignores the failures.
6. **`case 3:;` / `case 5:;` fall-through** in `engine.c` (bucket 3 traces the
   same letter as bucket 4) and in `a.c`'s `target` (k = 5 and 6 both give 5).
7. **`(size_t)k` in the jump check** (`engine.c:113`): a negative displacement
   becomes a huge unsigned value, so `6` with `k < 0` always returns 7 rather
   than jumping backwards. (`conditional_jumps`, `jump_offset_sweep`)
8. **Opcode 7 recursion runs exactly one instruction** (`n == 1`), so any
   instruction needing an operand fails inside the recursion, yielding trace 12
   and `p.ip = saved_ip + 1`. `vm->steps` is still incremented by the nested
   call. (`loop_opcode`, `nested_loop_body_covers_every_opcode`)
9. **Trace letters are masked with `& 25`, not `% 26`** (`util.c:57`), so trace
   14 prints `i`, the same letter as trace 8. (`long_trace_wraps_...`)
10. **`iv_peek` defaults**: `-777` for the printed stack top, `0` for opcodes
    3/5/8 on an empty stack.
11. **`strtol` acceptance rules** in both `main`'s argv loop and `read_stdin`:
    the token must be consumed entirely (`*e == '\0'`), `""` is accepted and
    yields 0, leading `\v`/`\f` are whitespace even though the tokenizer does
    not split on them, out-of-range values saturate at `LONG_MAX`/`LONG_MIN`
    and are then narrowed with `(int)`. Rejected argv entries print
    `skip '<raw bytes>'` on stderr; rejected stdin tokens print nothing.
    (`whitespace_and_sign_forms`, `integer_narrowing_and_saturation`,
    `unparsable_arguments_are_skipped`, `non_utf8_arguments_are_echoed_verbatim`)
12. **`fgets`, not `scanf`**: at most 4095 bytes per call, so a long line is
    handed to the tokenizer in pieces and a number straddling the cut is parsed
    as two tokens; the tokenizer then treats each chunk as a C string, so an
    embedded NUL hides the rest of that chunk but not later lines.
    (`fgets_chunk_boundary_sweep`, `stdin_long_lines_are_split_by_fgets`,
    `stdin_embedded_nul_truncates_the_line`)
13. **Exit statuses**: 0 normally and for `--help` (which is honoured as soon as
    it is seen, after any preceding argument has already been parsed), 2 for
    "no program" on stderr. Every `return` in `run_engine` (1…11, 99) only ever
    shows up inside the `RC:` line, never in the process exit status.
    (`error_return_paths`, `no_arguments_is_no_program`, `help_flag`)
14. **Signed wraparound** in `+`/`*`/`<<` is reproduced with `wrapping_*`, so
    `0 2147483647 0 2 1` prints the same as the C program.

## Reproducing

```
cd translation && cargo test            # 31 tests, debug binary
cd translation && cargo test --release  # 31 tests, release binary
```

The C program is taken from `c_src/build/driver` when it exists, otherwise the
test suite configures and builds it out of source into
`translation/target/c_build` (so `c_src/` is never written to). Set `C_DRIVER`
to compare against a differently built C binary — e.g. a `gcc -O2` build, which
was also verified to match.
