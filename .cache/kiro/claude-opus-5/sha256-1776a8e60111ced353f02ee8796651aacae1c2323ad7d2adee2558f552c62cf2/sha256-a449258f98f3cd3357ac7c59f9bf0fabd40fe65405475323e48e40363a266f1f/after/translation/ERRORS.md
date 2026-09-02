# Differential verification: C ground truth vs. Rust translation

Both programs are the `driver` executable built from `c_src/` (CMake) and
`translation/` (Cargo). They are compared by *running* them, never by linking
the Rust code as a library.

## How to reproduce

```sh
# C
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
# -> c_src/build/driver

# Rust
cd translation && cargo build --release
# -> translation/target/release/driver

# Differential suite
cd translation && cargo test
```

`translation/tests/differential.rs` spawns both binaries for every input and
asserts stdout, stderr and exit status all match. 32 tests, roughly 2,000
program executions per binary. None are `#[ignore]`d.

## Mismatches found and fixed

### 1. SIGPIPE disposition — exit status differed (FIXED)

The only real mismatch found in the shipped translation.

The Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs. A write to a
closed stdout therefore returns `EPIPE`, which every write site in `main.rs`
discards with `let _ = ...`, and the process still exits `0`. The C program
inherits the default disposition and is killed by the signal.

Observed:

```
$ ./c_src/build/driver 7 1000000 3 | head -c 1 >/dev/null
# ${PIPESTATUS[0]} == 141   (killed by SIGPIPE)

$ ./translation/target/release/driver 7 1000000 3 | head -c 1 >/dev/null
# ${PIPESTATUS[0]} == 0     (WRONG)
```

Cause: Rust-runtime-imposed signal state that has no counterpart in the C
program. Fix: `restore_default_sigpipe()` in `translation/src/main.rs` calls
`signal(SIGPIPE, SIG_DFL)` as the first statement of `main`, via a direct
`extern "C"` declaration (no new dependency). Covered by
`sigpipe_kills_both_programs_the_same_way`.

Reaching this needs more output than a pipe buffer holds, so it only shows up
with a long `TRACE=` line — which is why happy-path testing misses it.

## C behavior that looks like a bug and was deliberately preserved

These were already correct in the translation. They are recorded because each
one is a place where "fixing" the Rust would break the comparison.

| C site | Behavior | Rust |
| --- | --- | --- |
| `util.c` `vm_print` | trace index is `& 25`, not `% 26`, so the 15 trace values collapse onto only `a`, `b`, `i`, `j` | `ALPHABET[(t & 25) as usize]` |
| `a.c` `process_a_stream` | `acc` is `size_t`; in `acc < -0x80000000LL` both sides promote to `unsigned long long`, so the guard is *always* true and the function unconditionally returns `INT_MIN` | `acc < (-0x80000000i64) as u64`, then `acc as u32 as i32` |
| `engine.c` case 9 | the pop loop is written twice, draining up to `2m` values while only `m` slots exist; where `iv_pop` fails it leaves `*out` alone, so `tmp[i]` keeps the first loop's value | both loops reproduced, `if let Some(v) = ... { tmp[i] = v }` |
| `engine.c` case 5 | `case 3:;` falls through into `case 4:`, so buckets 3 and 4 both emit trace 8 | `3 \| 4 => vm.trace(8)` |
| `a.c` `target` | `case 5:;` falls through into `case 6:` | `5 \| 6 => 5` |
| `engine.c` case 6 | `(size_t)k` sign-extends, so any negative `k` becomes a huge unsigned value and always returns 7 | `k as i64 as usize` |
| `main.c` argv loop | `strtol("")` performs no conversion and sets `endptr == nptr`, so `*e == '\0'` holds and an **empty** argv entry is pushed as the value `0` rather than skipped | `strtol10` returns `(0, 0)`; `end == arg.len()` is true for an empty arg |
| `main.c` `read_stdin` | the fgets buffer is walked as a C string, so an embedded NUL byte silently truncates that chunk; later lines are still read | chunk sliced at the first `0` byte |
| `main.c` `read_stdin` | `fgets` caps at 4095 bytes, so a long line arrives in pieces and a number straddling the boundary is split into two tokens | `fgets(&mut reader, 4096)` |
| `main.c` | argv rejects print `skip '...'` to stderr; identical rejects from stdin print nothing | matched |
| `b.c` `target` | `flipflop` is toggled on *every* call, including the `code < 0` early return | toggle precedes the sign check |
| `a.c` / `b.c` | `static int state_a` / `static int flipflop` persist for the whole process, so results depend on how many times each engine has classified so far | `thread_local! { Cell<i32> }`, single-threaded |
| `a.c` / `b.c` / `lib.c` | three *different* functions named `target`: two file-static, one global. `engine.c` includes `api.h` and binds to the global one for `impl_id == 2` only | `a::`, `b::`, `libtarget::` respectively |
| `engine.c` `classify` | `MAC_CALL` adds 1 but `inline_call` does not, so `impl 0` is `call_a_once(x)` while `impl 1` is `call_b_once(x + 1)` and `impl 2` is `target(target(x + 1))`. `a.c`'s `A_MAC_CALL` adds nothing | matched exactly |
| `engine.c` case 3 / case 5 / case 8 | peek with default `0` on an empty stack (so `DUP` pushes 0); `vm_print` peeks with default `-777` | matched |
| everywhere | signed `int` overflow and signed left shifts are UB in C but wrap in practice; `vm->steps++` wraps past `INT_MAX` | `wrapping_add` / `wrapping_mul` / `wrapping_shl`, verified to match including `STEPS=-2147483644` after 2^31 steps |

## Remaining known divergences (not fixed, with reasons)

### `--help` prints `argv[0]`

`usage()` prints the path the binary was invoked as, which is necessarily
different for the two executables. The tests normalize the binary's own path to
`$PROG` and compare the rest of the line byte for byte.

### Very large VLA in `engine.c` case 9 exhausts the C stack

`int tmp[m]` is a variable-length array sized from the program, so a large
enough `m` overflows the C process stack:

```
$ ./c_src/build/driver 7 3000000 3 9 3000000
# C:    killed by SIGSEGV (exit 139)
# Rust: exits 0 and prints normally
```

Not replicated. The threshold is C stack exhaustion — undefined behavior whose
trigger point depends on `ulimit -s` (8 MB here, so roughly `m > 2×10^6`), not
on program semantics, and there is no portable way to size a stack allocation
from a runtime value in Rust. Verified matching up to `m = 1,500,000`
(`7 1500000 3 9 1500000`).

### Runtime, on inputs that take minutes

`c_src/CMakeLists.txt` sets no optimization flags, so the C build is `-O0`
while the Rust build is `--release`. Output is identical; only wall time
differs, and the Rust program is the faster of the two.

```
7 20000000 5    ->  C 2.36 s, Rust 1.18 s
```

One fuzz case (`7 2147483647 10 ...`, ~2^31 iterations) exceeded a 20 s
per-case budget for C only. Run to completion without a timeout, stdout,
stderr and exit status were byte-identical, including the wrapped
`STEPS=-2147483644`.

## Dead files in `translation/src/`

`cstd.rs` and `lib_target.rs` are not declared as modules in `main.rs` (which
declares only `a`, `b`, `engine`, `libtarget`, `util`), so they are never
compiled. `cstd.rs` duplicates the `strtol`/`fgets` emulation that `main.rs`
implements inline, and `lib_target.rs` duplicates `libtarget.rs`. Left in place
because they affect nothing; noted so a reader does not mistake them for the
code under test.

Its `c_strtol` clamps the accumulated magnitude to `LONG_MAX` / `LONG_MAX + 1`,
whereas the live `strtol10` clamps only on `i64` overflow. The difference is
unobservable: the result is immediately truncated with `(int)`, and
`LONG_MAX`/`i64::MAX` both truncate to `-1` while `LONG_MIN`/`i64::MIN` both
truncate to `0`.

## Input classes enumerated from the C source

Argument handling (`main.c`):

- no arguments -> `no program` on stderr, exit 2
- `--help`, alone and mixed with other args before and after it -> usage on
  stderr, exit 0, remaining args never parsed
- unparsable arg -> `skip '<arg>'` on stderr (`abc`, `5abc`, `0x10`, `1e3`,
  `-`, `--`, `"5 "`, `"  -3  "`)
- empty arg `""` -> parses as 0
- accepted-by-`strtol` forms: `+5`, `" 5"`, `"\t7"`, `007`, `-0`
- `strtol` range errors then `(int)` truncation: `2147483648`, `4294967296`,
  `9223372036854775807/8`, `-9223372036854775808/9`, `99999...`
- non-UTF-8 argument bytes

`read_stdin`:

- `--stdin` with empty input, `"\n"`, `"   \n"`
- one token, several tokens, one token per line, no trailing newline
- delimiters ` `, `\t`, `\r`, `\n` — and `\v`/`\f`, which are *not* delimiters
  but which `strtol` does skip as leading whitespace
- unparsable tokens (dropped silently, unlike argv)
- embedded NUL in several positions
- 4096-byte `fgets` chunk boundary, including a number split across it
- 5000-byte token with no newline
- `--stdin` combined with argv values, repeated `--stdin`, stdin ignored when
  the flag is absent

`run_engine` — every opcode arm and every early `return`:

| opcode | ok path | error returns exercised |
| --- | --- | --- |
| 0 PUSH | `0 5` | 1 (immediate fetch fails) |
| 1 ADD | `0 5 0 6 1` | 2 (empty, and one operand) |
| 2 MUL | `0 5 0 6 2` | 3 (empty, and one operand) |
| 3 DUP | `3` on an empty stack pushes 0 | — |
| 4 DROP | `0 5 4` | 4 |
| 5 CLASSIFY | all five trace buckets, 18 immediates | — |
| 6 JMP-IF | taken (`k` = 0, 1, exactly-remaining), not taken | 5 (no `k`), 6 (no cond), 7 (`k` too large, `k` negative, `INT_MIN`) |
| 7 REPEAT | `times` = 0, negative, 1, 4, 200000, `INT_MAX`; body succeeds, body fails -> trace 12 | 8 (no `times`), 9 (nothing to repeat) |
| 8 CLASSIFY-2 | 18 immediates | — |
| 9 REDUCE | `m` = 0, `m` = stack len, `m` with >= `2m` on the stack | 10 (no `m`), 11 (`m < 0`, `INT_MIN`, `m >` stack len) |
| 10 HALT | stops, trailing words never run | — |
| default | `11`, `-1`, `100`, `INT_MAX`, `INT_MIN` -> 99 | — |

Sweeps: every 1-word program over `-3..=13`; all 289 2-word programs over
`-3..=13`; all 729 3-word programs over the 9 control-flow opcodes; 400
deterministic pseudorandom programs, each run both as argv and through stdin.

Additional out-of-band sweeps run during verification (not part of
`cargo test`, all zero mismatches): exhaustive 1-3 words over 15 values
(3,615 programs), exhaustive 1-4 words over 12 opcodes (22,620), exhaustive
1-5 words over 9 opcodes (66,429), and ~7,300 random programs of 5-30 words
with large immediates.
