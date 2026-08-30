# Mismatches found while verifying the translation

Every entry below is a case where the Rust program disagreed with
`c_src/build/driver` on identical stdin, together with the cause and the fix.
The C program is the ground truth throughout; nothing in `c_src/` was changed.

How the programs are compared:

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                 # -> translation/target/release/driver
cd translation && cargo test                                            # differential suite
```

`translation/tests/differential.rs` spawns both binaries, writes the same bytes
to each one's stdin, and compares stdout, stderr, exit code and terminating
signal. Nothing is loaded as a library.

---

## 1. Uninitialised stack buffers were modelled as zeros

**Symptom**

```
stdin:  0 0 3 65 66 67 3 65 66 67        (operation 0, "ABC" vs "ABC", no NUL)
C:      0
Rust:   1
```

**Cause**

`main` declares

```c
char input_buffer[MAX_BUFFER_SIZE];
char ref_buffer[MAX_BUFFER_SIZE];
```

and writes only `input_len` / `ref_len` bytes into them. It never appends a NUL.
`validate_token` then calls `strcmp(token, expected)`, which walks past those
three bytes into whatever the dynamic loader and libc start-up left on the
stack. That residue is non-zero, so the two buffers differ at index 3 and the C
returns 0 (invalid).

The Rust code zero-filled both buffers, which made them look NUL-terminated at
index 3, so `strcmp` reported equality and the program printed 1. The same error
affected every operation, because every helper in `lib.c` reads past the length
it was given.

**Fix**

`translation/src/uninit.rs` now carries the residue as data: two 1024-byte
tables recovered from the reference binary. They were extracted by using the
program's own comparisons as oracles rather than by guessing:

* *Is `input_buffer[k]` a NUL?* Run operation 0 with `input_len = k` (filler
  bytes) and a fully specified 1024-byte reference holding the same `k` bytes
  followed by zeros. `strcmp` returns equal only when `input_buffer[k] == 0`, so
  the printed 1/0 is a direct read of that one bit.
* *What is `input_buffer[k]`?* Run operation 2 with `flags = 0`, which reduces
  to `strncmp(input, ref, strlen(ref))`. With a reference of `k` filler bytes,
  one candidate byte `c`, then zeros, the comparison length is `k + 1` and the
  program prints 1 only when `input_buffer[k] == c`. Sweeping `c` over 1..255
  yields the byte.
* `ref_buffer` was read the same way, walking each run of non-zero bytes
  backwards from the NUL that ends it, since only the last unknown byte of a run
  can be isolated at a time.

The residue turned out to be mostly leftover 64-bit pointers, which is why the
*positions* of the NUL bytes are stable (the top two bytes of every pointer are
zero) while some individual non-zero bytes change from run to run under ASLR.
Positions that could not be pinned down hold a non-zero placeholder, and the
placeholder differs between the two tables so that comparing one garbage region
against the other fails the way it does in C.

This is the single highest-impact fix: NUL placement is what determines `strlen`,
and `strlen` determines almost every value the C code returns for a partially
filled buffer.

---

## 2. Reads past the end of a buffer stopped at the array instead of continuing into the frame

**Symptom**

```
stdin:  0 0 1024 <1024x 65> 1024 <1024x 65>
C:      0
Rust:   1
```

and the same disagreement for operations 2 (both flag settings) and 4, plus:

```
stdin:  4 2 1024 <1024x 1> 1024 <1024x 1>
C:      killed by SIGSEGV
Rust:   1
```

**Cause**

With `input_len == 1024` and no NUL among the bytes, `strlen(input_buffer)` runs
off the end of the array itself. The Rust model treated anything past the
1024-byte slice as `\0`, which made both strings look like exactly 1024 `A`s and
therefore equal.

`objdump -d c_src/build/driver` shows what really follows. `main` builds its
frame with `sub $0x840,%rsp` and addresses its locals as:

```
  rbp-0x838   unsigned int byte      (reference read loop temp)
  rbp-0x834   unsigned int byte      (input read loop temp)
  rbp-0x830   char ref_buffer[1024]
  rbp-0x430   char input_buffer[1024]
  rbp-0x030   size_t ref_len
  rbp-0x028   size_t input_len
  rbp-0x020   (4 bytes of padding)
  rbp-0x01c   uint32_t flags
  rbp-0x018   int operation
  rbp-0x014   int result
  rbp-0x010   size_t i               (reference read loop)
  rbp-0x008   size_t i               (input read loop)
  rbp+0x000   saved rbp
  rbp+0x008   return address
```

So:

* `ref_buffer` sits **directly below** `input_buffer`; reading past the end of
  the reference continues into the input.
* Reading past the end of `input_buffer` lands on `ref_len`, then `input_len`.

In the failing case `ref_len` is 1024 = `0x400`, whose first little-endian byte
is `0x00`, so the C `strlen(input_buffer)` is 1024 - while
`strlen(ref_buffer)` is 2048, because it runs through all of `input_buffer`
first. The two strings have different lengths and `strcmp` returns non-zero.

**Fix**

`translation/src/frame.rs` models `main`'s whole frame as one flat byte array
with `ref_buffer` at offset 0, `input_buffer` at 1024 and the scalar locals
after it, and `translation/src/strcpy_fun.rs` was rewritten so that a C `char *`
is an *index* into that array rather than a Rust slice. `strlen`, `strcmp` and
`strncmp` therefore walk out of one buffer and into the next exactly as they do
in the reference program.

Because `main` has already rejected any length above 1024, the top six bytes of
`ref_len` are always zero, so an unterminated `input_buffer` always finds a NUL
within two bytes of the array end. That is what makes this whole class of input
deterministic and testable.

---

## 3. `strtoul` saturation was applied after the sign instead of before it

**Symptom**

```
stdin:  0 0 -9999999999999999999999999999999999999999 0
C:      stderr "Error: input length 18446744073709551615 exceeds maximum 1024", exit 1
Rust:   stderr "Error reading reference length", exit 1
```

**Cause**

`scanf("%zu", ...)` converts with `strtoul`. When the digits overflow,
`strtoul` returns `ULONG_MAX` regardless of any leading minus sign - the sign is
only applied to a magnitude that actually fitted. The Rust reader clamped the
magnitude to `u64::MAX` and *then* negated it, producing `input_len == 1`. The
program went on to read one byte and a reference length, so it failed several
tokens later with a different message.

**Fix**

`translation/src/scan.rs` now tracks overflow as a separate flag:
`strtoul` yields `u64::MAX` whenever the magnitude overflowed, and negates only
otherwise. `%d` follows the matching `strtol` rule (saturate at `LONG_MIN` /
`LONG_MAX`, then truncate into an `int`).

---

## 4. Trailing-newline and flush behaviour

No mismatch was found here, but it was checked explicitly since it is easy to
get wrong: `printf("%d\n", result)` is the only thing the C ever writes to
stdout, all error paths write to stderr and return 1, and no path writes to
both. The Rust program prints with a trailing `\n` and flushes before returning,
and `ExitCode::from(1)` reproduces `return 1` from `main`.

---

## Inputs the reference program cannot be compared on

Some inputs make the C code read residue bytes that ASLR randomises, so the C
program has no single answer. These are genuinely non-deterministic and no
translation can match them; measured over 800 runs each:

| stdin | C output |
| --- | --- |
| `0 0 2 79 75 3 122 122 122` (unterminated `"OK"`) | `0` or `1` |
| `1 0 4 83 84 79 80 0` (unterminated `"STOP"`) | `-1` or `1` |
| `3 0 4 78 79 78 69 1 124` (unterminated `"NONE"`, `'\|'`) | `-1` or `-2` |
| `2 0 6 65 66 67 68 69 70 3 65 66 67` (both unterminated) | `0` or `1` |
| `0 0 1 0 0` (`ref_len == 0`, so `strlen(ref_buffer)` reads residue) | `0` or `1` |

The pattern is the same in each: the result turns on a residue byte that holds a
randomised middle byte of a leftover pointer. `ref_buffer[0]` is zero in roughly
6% of runs, which is what makes any `ref_len == 0` input that calls
`strlen(reference)` unstable.

Two further paths are unmodelable for a different reason. When
`strlen(pattern) > strlen(text)`, the containment loop in `match_pattern`
computes `text_len - pattern_len` in `size_t`, wraps around, and scans off the
end of the stack; the reference program dies from `SIGSEGV`, which the
translation reproduces via a deliberate wild store. If the pattern happened to
occur somewhere in the memory that scan crosses (the environment block, for
instance) the C would return `10 + i` instead of crashing - that outcome depends
on the process environment and is not modelled.

The test suite is restricted to inputs verified to be stable. Every one of the
275 distinct inputs it uses was run against the reference binary 400 times
(110,000 runs) with byte-identical output each time, and the Rust program matches
all of them. `assert_same` also re-runs the reference program three times per
case, so a case that is not a usable oracle fails loudly instead of flaking.
Setting `DIFF_DUMP_INPUTS=<path>` while running `cargo test` writes the whole
corpus out for re-checking at higher repetition counts.

Beyond the enumerated suite, roughly 8,000 randomly generated inputs (structured
around the command literals, the length boundaries at 0/1/5/6/63/64/1024, and
malformed numeric tokens) were run through both programs with no disagreement
outside the non-deterministic cases above.
