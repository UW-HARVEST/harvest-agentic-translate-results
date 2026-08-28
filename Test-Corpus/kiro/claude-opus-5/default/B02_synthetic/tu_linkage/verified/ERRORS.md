# Differential verification of the C → Rust translation

Ground truth: `c_src/` (never modified). Subject: `translation/`.

Both programs are compared by **execution only** — spawned as subprocesses,
fed identical argv and identical stdin bytes, then stdout, stderr and exit
status are compared byte for byte. See `tests/differential.rs`.

## How to reproduce

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                  # -> translation/target/release/driver
cd translation && cargo test                                             # builds the C side automatically if absent
```

## Result

**No behavioural mismatch was found.** 43 test functions covering roughly 1,600
distinct (argv, stdin) pairs pass, plus ~40,000 additional randomized pairs run
as an out-of-tree sweep. Every enumerated input produces identical stdout,
identical stderr and an identical exit status.

The only difference between the two programs' output streams is `argv[0]`,
which `usage()` prints verbatim and which is necessarily the path of whichever
binary was invoked. The test harness substitutes each program's own path with
`<PROG>` and compares every other byte exactly (`normalize()` in
`tests/differential.rs`). This is a property of the invocation, not of the
translation.

## Coverage evidence

A `gcc --coverage` copy of the C sources (built outside `c_src/`, in `/tmp`, so
that `c_src/` stayed untouched) was driven with the same corpus. Result:
**100.00% line coverage and 100% of branches evaluated in all six C files**
(`main.c`, `engine.c`, `a.c`, `b.c`, `util.c`, `lib.c`).

Two input classes were found *only* because of this coverage pass, and were then
added to the suite:

| Gap found by gcov | Why random programs missed it | Test added |
|---|---|---|
| `lib.c:32-33` — `if (m == 7) return 3;` and the final `return 4;` (i.e. `code % 10` in {7, 8, 9}) | Only reachable through the EXT engine with specific immediates such as `0 7 5` or `0 8 8`; opcode-shaped random programs rarely push 7/8/9 and then classify | `lib_target_covers_every_modulo_bucket` |
| `engine.c:101` — `case 2: vm_trace(vm, 7);` in the classify-bucket switch | Requires `classify` to return exactly `2`, which for EXT needs `target(target(x+1)) == 2`, i.e. `x+1 ≡ 8 or 9 (mod 10)` | `classify_bucket_switch_covers_every_arm` |

Both matched the C on the first run, so they are recorded here as coverage gaps
in the *test suite*, not as defects in the translation.

## C behaviours that are bugs, and are reproduced deliberately

These are the places where a "sensible" translation would diverge. Each was
checked against the C binary; the Rust already reproduces all of them.

1. **`process_a_stream` always returns `INT_MIN`** (`c_src/src/a.c:69-73`).
   `acc` is `size_t`, so both clamp comparisons are *unsigned*:

   ```c
   if (acc > 0x7fffffffLL)  acc = 0x7fffffffLL;   /* leaves acc <= 2147483647 */
   if (acc < -0x80000000LL) acc = -0x80000000LL;  /* -0x80000000LL -> 0xffffffff80000000 */
   ```

   The second condition is therefore *always* true, and `acc` is overwritten with
   `0xffffffff80000000` before the `(int)` truncation. Every `REDUCE` on the A
   engine yields `-2147483648`, regardless of the data. `translation/src/a.rs`
   keeps `acc: u64` and performs the same two unsigned comparisons.
   Observable: `driver 9 0` prints `A:STACK_TOP=-2147483648`.

2. **`REDUCE` pops its operands twice** (`c_src/src/engine.c`, `case 9`). The
   validation only guarantees `m <= stack.len`, but the body contains the same
   pop loop twice. The second pass can run the stack dry, and `iv_pop` leaves
   `*out` untouched on failure, so those `tmp[i]` slots keep the value the first
   pass wrote. With `m = 3` and four stack entries, only `tmp[2]` is replaced.
   Reproduced in `engine.rs` by ignoring a `None` from `iv_pop` rather than
   assigning. Covered by `reduce_pops_its_operands_twice`, which walks stack
   depths `m` through `2m` so the fully-failing, partially-failing and
   fully-succeeding second passes are all exercised.

3. **`vm_print` masks the trace code with `& 25`, not `& 31`**
   (`c_src/src/util.c`). `25` is `0b11001`, so trace codes collapse onto the
   same letters: 0/2/4 all print `a`, 1/3/5 all print `b`. Kept verbatim in
   `util.rs`.

4. **`case 3:;` fall-through** in `engine.c`'s bucket switch and in `a.c`'s
   `case 5:;` — the empty statement does not stop the fall-through, so buckets
   3 and 4 share a trace and `k` of 5 and 6 share a return. Mirrored as
   `3 | 4 => ...` and `5 | 6 => 5`.

5. **A negative jump distance is a huge unsigned value.** In `case 6`,
   `(size_t)k > p.n - p.ip` sign-extends `k`, so any negative `k` fails the
   bound check and returns `7` instead of jumping backwards. `engine.rs` uses
   `k as usize`, which sign-extends identically. Covered by
   `run_engine_return_code_7_jump_target_out_of_range`.

6. **`DUP` and `CLASSIFY` peek with a default instead of failing.**
   `iv_peek(&vm->stack, 0)` means `3`, `5` and `8` succeed on an empty stack and
   operate on `0`, while `vm_print` peeks with `-777`. Both defaults are kept.

7. **An empty argv entry parses as the bytecode `0`.** `strtol("")` performs no
   conversion and sets `endptr == nptr`, which already points at the
   terminating NUL, so `*e == '\0'` holds and `0` is pushed — no `skip` message.
   `cstd::c_strtol` returns end index `0` for this case and `main.rs` compares
   it against `arg.len()`, which is also `0`. Covered by
   `empty_argument_parses_as_zero`.

8. **File-scope statics persist across all three engine runs.** `state_a`
   (`a.c`) and `flipflop` (`b.c`) are never reset, so the A, B and EXT results
   depend on how many `classify` calls preceded them and in what order. The
   Rust keeps them in `thread_local! { Cell<i32> }` and `main.rs` runs the three
   engines in the original order. Covered by
   `static_state_carries_across_the_three_engine_runs`.

9. **`a.c` and `b.c` each define their own `static int target`**, shadowing the
   global `target` from `lib.c` inside that translation unit only. Even
   `&target` inside `a.c` refers to the static one. The Rust keeps three
   separate functions (`a::target`, `b::target`, `lib_target::target`) and
   `classify` dispatches to the right one per `impl_id`.

10. **Signed overflow is wrapped, not trapped.** `state_a ^ (code<<1)`,
    `acc*3`, `a+b`, `a*b` and `(int)` truncation of a saturated `strtol` result
    are all undefined or implementation-defined in C but wrap under gcc. Every
    such site in the Rust uses `wrapping_*` / `as` casts, so the release build
    (and the debug build, where Rust's overflow checks are on) both match.

11. **`fgets` splits tokens at the buffer edge.** `read_stdin` uses
    `char buf[4096]`, so a number longer than 4095 bytes, or one that straddles
    the boundary, is tokenized as two separate numbers. `cstd::c_fgets` caps
    each read at `size - 1 == 4095` bytes and stops after a newline, keeping the
    newline. Covered by
    `stdin_tokens_straddling_the_fgets_buffer_boundary_are_split` at pads
    4090/4092/4093/4094/4095/4096/4097/4100.

12. **An embedded NUL discards the rest of the chunk.** The tokenizer walks a C
    string (`while (*p)`), so a `\0` byte in the middle of a line silently drops
    everything after it in that `fgets` buffer. `read_stdin` in `main.rs`
    truncates each chunk at the first `0` byte. Covered by
    `stdin_embedded_nul_truncates_the_rest_of_the_chunk`.

13. **stdin tokens fail silently; argv tokens print `skip '...'`.** Only the
    argv loop has the `fprintf(stderr, "skip '%s'\n", ...)`. Covered by
    `stdin_non_numeric_tokens_are_dropped_silently` and
    `non_numeric_arguments_are_skipped_with_a_message`.

14. **`--help` returns immediately, mid-scan.** Arguments before it have already
    been parsed and their `skip` messages already printed; arguments after it
    are never looked at, and stdin is never read even with `--stdin`. Exit
    status is `0`, and stdout stays empty because `usage` writes to stderr.
    Covered by `help_wins_immediately_wherever_it_appears`.

## Branches that are unreachable, and why

These are the only C branches gcov reports as never taken. Each is dead code or
an out-of-memory path, so no input can distinguish the two programs through
them. They are listed for completeness rather than left unexplained.

| Location | Status |
|---|---|
| `a.c:33` — the `6` arm of `return (state_a & 1) ? 6 : 5;` | **Provably dead.** `state_a` starts at `0` and is only ever updated as `state_a ^= (code << 1)`; `code << 1` always has a zero low bit, so `state_a` is even forever and `state_a & 1` is always `0`. The Rust keeps the conditional anyway. |
| `a.c:71` — the `acc > 0x7fffffffLL` clamp | Needs `acc > 2^31`, but `acc` grows by at most `21` per stream element, so it would take ~10^8 elements (a multi-gigabyte stack). Also unobservable: as shown in item 1 the function returns `INT_MIN` whether or not this clamp fires. |
| `main.c:49`, `main.c:67` — the `e &&` half of `if (e && *e=='\0')` | **Dead.** `strtol` always stores a non-NULL `endptr`. |
| `util.c:34` — `if (need <= v->cap) return true;` | Dead via its only caller: `iv_push` calls `iv_reserve(v, cap ? cap*2 : 8)`, so `need > cap` always holds. |
| `util.c:37`, `util.c:41` — `nc > SIZE_MAX/2` and `!p` | Allocation-failure paths. Unreachable without OOM; Rust's `Vec` aborts instead of silently dropping a push. |
| `util.c:45` — the `if (out)` NULL check in `iv_pop` | Dead; every call site passes a real pointer. |

## Test inventory

`tests/differential.rs`, 43 tests, none `#[ignore]`d, skipped or disabled:

- **argv**: no arguments (exit 2, `no program`), `--help` in every position,
  non-numeric arguments, `strtol` acceptance (`+5`, leading space, trailing
  space, `0x10`, `5abc`, `007`, `-0`), the empty argument, `strtol`
  saturation then `(int)` truncation (`2147483648`, `4294967296`,
  `±9223372036854775807`, 20-digit values), repeated `--stdin`.
- **stdin**: absent flag, empty, newline-only, whitespace-only, one token, many
  tokens, tab/CR/CRLF separators, no trailing newline, argv-then-stdin
  ordering, non-numeric tokens, embedded NUL in four positions, buffer-boundary
  straddling at eight pad lengths, 3,000-line and 6,000-token inputs.
- **VM**: every opcode alone (`-3..=13`), every unknown-opcode value, and one
  test per `run_engine` return code — `0` through `11` and `99` are all
  reached and asserted.
- **VM boundaries**: `PUSH` with no immediate; `ADD`/`MUL` with zero and one
  operand; `DROP` on empty; `JMP-IF` with no `k`, no condition, `k == 0`,
  `k == remaining`, `k == remaining + 1`, negative `k`, and condition zero;
  `REPEAT` with no count, nothing to repeat, counts `-3/0/1/2/5/40/1000`,
  every body opcode, and a nested `REPEAT`; `REDUCE` with no `m`, `m < 0`,
  `m > stack.len`, `m == 0`, and stack depths `m..2m`; `HALT` before a bad
  opcode.
- **Numeric edges**: `INT_MIN`/`INT_MAX` immediates, negative codes `-1..-25`
  through all three `target` implementations, and `code % 10` buckets `0..9`.
- **Bulk**: all 225 two-opcode programs over `-2..=12`, and four seeded
  xorshift fuzzers (1,200 programs total) covering opcode-shaped, small-integer,
  long (20-60 instruction) and stdin-delivered inputs.
