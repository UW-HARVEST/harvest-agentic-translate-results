# Differential findings: `c_src` vs. `translation`

Everything below was found by building both programs and running them side by
side (`translation/tests/differential.rs`, 178 tests) plus roughly 6 500
additional throw-away fuzz inputs.  The C program is the ground truth; every fix
was made in the Rust code.

Build / run commands used throughout:

```
cd c_src && cmake -S . -B build && cmake --build build   # -> c_src/build/driver
cd translation && cargo build --release                  # -> target/release/driver
cd translation && cargo test                             # 178 differential tests
```

---

## Why this program is hard to translate

`c_src/src/main.c` declares

```c
char input_buffer[MAX_BUFFER_SIZE];   /* 1024 bytes, never initialised */
char ref_buffer[MAX_BUFFER_SIZE];     /* 1024 bytes, never initialised */
```

and fills only `input_len` / `ref_len` bytes of them.  `lib.c` then runs
`strlen()`, `strcmp()` and `strncmp()` over those buffers, so as soon as the
data it was given contains no NUL byte the *leftover stack contents* decide what
the program prints.  Reproducing that in Rust means reproducing the C stack
frame: both its residue and its layout.  The layout was read off `gcc`'s
prologue (`objdump -d c_src/build/driver`):

| address     | object                     | offset in the modelled frame |
| ----------- | -------------------------- | ---------------------------- |
| `%rbp-0x830`| `ref_buffer[1024]`         | `0 .. 1024`                  |
| `%rbp-0x430`| `input_buffer[1024]`       | `1024 .. 2048`               |
| `%rbp-0x30` | `ref_len`   (`size_t`)     | `2048 .. 2056`               |
| `%rbp-0x28` | `input_len` (`size_t`)     | `2056 .. 2064`               |
| `%rbp-0x20` | padding (uninitialised)    | `2064 .. 2068`               |
| `%rbp-0x1c` | `flags`     (`uint32_t`)   | `2068 .. 2072`               |
| `%rbp-0x18` | `operation` (`int`)        | `2072 .. 2076`               |
| `%rbp-0x14` | `result`    (uninitialised)| `2076 .. 2080`               |
| `%rbp-0x10` | `i` of the reference loop  | `2080 .. 2088`               |
| `%rbp-0x08` | `i` of the input loop      | `2088 .. 2096`               |

Two facts follow, and both are *observable*:

* `ref_buffer` is immediately followed by `input_buffer`, so a `strlen()` /
  `strcmp()` that walks off the end of the reference data continues inside the
  input data;
* `input_buffer` is immediately followed by `main()`'s locals, so a `strlen()`
  that walks off the end of a completely filled input buffer reads the
  little-endian bytes of `ref_len`.

`src/main.rs` therefore models the whole frame as one `Vec<u8>` and hands
`process_strings()` two overlapping sub-slices of it.

---

## Mismatch 1 - invented stack residue instead of the real one

**Symptom.** Any input whose payload is not NUL terminated could produce a
different number.  Examples (all reproducible, `C` = `c_src/build/driver`):

| stdin                                     | C   | Rust before |
| ----------------------------------------- | --- | ----------- |
| `4 7 5 83 84 79 80 88 5 83 84 79 80 88`   | `1` | `0`         |
| `4 1 5 65 68 77 73 78 5 65 68 77 73 78`   | `1` | `0`         |
| `2 1 5 80 65 85 83 69 5 80 65 85 83 69`   | `1` | `0`         |
| `4 0 1024 <1024x65> 8 <8x65>`             | `5` | `0`         |
| `4 0 1024 <1024x65> 257 <257x65>`         | `5` | `0`         |

**Cause.** The first translation filled both buffers with a *synthetic* pattern
("six pseudo-random non-zero bytes followed by two zero bytes", seeded
differently per buffer).  That guess is wrong in two independent ways:

1. the real residue is not periodic - e.g. `ref_buffer[8 .. 32]` is all zero in
   the real program, but non-zero in the synthetic model, which is why
   `4 0 1024 … 8 …` printed `0` instead of `5` (the reference string was 14 bytes
   long in the model and 8 bytes long in reality);
2. the two buffers were seeded *differently*, so residue never compared equal
   between them.  In the real program both buffers hold leftover 48-bit
   addresses, so byte 5 of each is `0x7f` and bytes 6 and 7 are `0x00`.  Two
   unterminated five byte payloads with equal content therefore compare *equal*
   in C (`…"ADMIN" vs "ADMIN" -> 1`) but unequal in the model.

**Fix.** `src/residue.rs` now contains the measured residue:

* the bytes were read out of the real program's stack at exactly
  `%rbp-0x830`, `%rbp-0x430` and `%rbp-0x30` (see
  `tools/dump_stack_residue.c`);
* independently, the compiled C program itself was probed for **each of the
  2048 buffer bytes**, nine times per byte, with inputs of the shape
  `0 0 k <k bytes> k+1 <same k bytes> 0` - operation 0 returns `1` exactly when
  `input_buffer[k] == 0`, and the mirrored input tests `ref_buffer[k]`.  The
  captured dump agreed with the probed program on 2046 of 2048 bytes; the two
  disagreements were resolved in favour of the probe (majority of 9 runs).

## Mismatch 2 - the two buffers were not adjacent

**Symptom.** `4 2 1024 <1024x65> 1024 <1024x65>` (operation 4, case sensitive,
both buffers completely filled with `'A'`):

* C: no output, killed by **SIGSEGV** (exit status `139`);
* Rust before: printed `0`, exit status `0`.

**Cause.** In C `strlen(ref_buffer)` does not stop at the end of `ref_buffer`;
it continues into `input_buffer` and only stops at the low byte of `ref_len`,
giving `pattern_len == 2048` while `text_len == 1024`.  `match_pattern()` then
computes `text_len - pattern_len` in `size_t` arithmetic, which underflows to
`0xffff_fffe_0000_0000`, and the following `strncmp()` loop walks up the stack
until it hits an unmapped page.  The old model kept the buffers in two separate
`Vec`s, so `pattern_len` stayed below `text_len` and the loop terminated
normally.

**Fix.** One contiguous frame (`src/main.rs::new_frame`), `reference` =
`&frame[0..]`, `input` = `&frame[1024..]`.  `cstr::segfault()` reproduces the
crash with a volatile read of address 1, so the exit status is a real SIGSEGV
rather than a Rust panic.

## Mismatch 3 - `main()`'s locals behind `input_buffer` were not modelled

**Symptom.** `4 2 1024 <1024x65> 8 65 8 0 1 1 1 1 1`

* C: `1033`
* Rust before: `0`

**Cause.** The reference string here is the two bytes `'A', 0x08`.  That
sequence exists exactly once in the C program's memory: at `input_buffer[1023]`
(the last `'A'`) followed by the first byte of the `ref_len` local, which is
`8`.  `match_pattern()`'s substring loop finds it at index 1023 and returns
`10 + 1023 == 1033`.  The old model padded the input buffer with 16 bytes of
synthetic residue instead of the real locals, so the pattern was not found.

**Fix.** After parsing, `main()` writes `ref_len`, `input_len`, `flags`,
`operation` and both loop counters into the modelled frame at the offsets listed
in the table above, exactly as the compiled C code leaves them at the moment
`process_strings()` is called.  Only offsets `2048 .. 2051` are actually
reachable, because `ref_len <= 1024` always leaves a zero byte at `2050`.

## Fixed for fidelity (no observable mismatch found)

* `match_pattern()`'s case-insensitive loop indexed the slices directly
  (`text[i]`), which would have aborted with a Rust panic message on stderr
  instead of dying from SIGSEGV had the index ever left the frame.  It now goes
  through `cstr::byte_at()` like every other memory read.
* `eprintln!` was replaced by `eprint!("…\n")` and `println!` by
  `print!("{}\n")` so that the emitted bytes are literally what `fprintf` /
  `printf` write, independent of the platform's line ending conventions.

## Checked and found already correct

* `scanf` emulation (`src/scanf.rs`): `%d`, `%u`, `%zu` skip leading
  whitespace, accept `+`/`-`, stop at the first non-digit, treat EOF and a
  matching failure identically, clamp to `LONG_MAX` / `ULONG_MAX` on overflow
  and then truncate to the destination width.  Verified with 2 500 random token
  streams (`0x10`, `1e3`, `--3`, `18446744073709551615`, `-0`, `007`, …) and
  with the dedicated tests `byte_written_as_hex_literal`,
  `operation_overflow_wraps_to_minus_1`, `input_len_negative`,
  `byte_value_321_truncated`, `flags_2_pow_32_plus_1`.
* All eight `main()` error messages, byte for byte, including the `%zu`
  formatting of `SIZE_MAX` and the `%d` formatting of `MAX_BUFFER_SIZE`.
* `strncpy` / `strncat` / `snprintf` truncation inside `compare_prefix()` and
  `match_pattern()` (tests `op2_exact_prefix_60_plus_tmp`,
  `op2_exact_prefix_64_bytes`, `op4_cs_wildcard_62_bytes`).
* `find_delimiter()` returning `(int)i` for `i` up to 1023, the `'\0'`
  delimiter, and both special cases (`-2`, `-3`).

## Unreachable C code (kept, but no test can reach it)

* `process_strings()` returns `-1` for `input == NULL` and `-2` for
  `reference == NULL`; `main()` always passes two arrays, so neither is
  reachable.
* `parse_command()`'s fallback `strcmp(buffer, cmd_list[i]) == 0` can only fire
  when `buf_size < cmd_len`, i.e. when the *stack residue* completes a command
  name; the measured residue starts with `0xb0 0x49 0xdc …`, so it never does.
* `compare_prefix()` computes `prefix_len` and never uses it on the
  `exact_match` path.

---

## Inputs whose behaviour the C program itself does not reproduce

Some of the residue bytes are the low bytes of stack addresses, and Linux
randomises the initial stack pointer in multiples of 16 bytes.  21 of the 2048
buffer bytes are therefore NUL in roughly one run out of sixteen:

```
input_buffer: 72 74 88 91 161 322 412 416
ref_buffer:   0 64 128 160 216 232 235 264 674 682 834 980 994
```

When a single one of those bytes decides the answer, the C program prints
different numbers on different runs.  Measured over 200 runs each:

| stdin                                   | C output distribution | this translation |
| --------------------------------------- | --------------------- | ---------------- |
| `0 0 1 0 0`                             | `0` 187x, `1` 13x     | `0`              |
| `2 0 6 72 69 76 76 79 0 0`              | `0` 186x, `1` 14x     | `0`              |
| `0 0 5 80 65 85 83 69 5 80 65 85 83 69` | `1` 196x, `0` 4x      | `1`              |
| `1 0 4 83 84 79 80 0`                   | `-1` 199x, `1` 1x     | `-1`             |
| `4 1 2 79 75 3 79 75 0`                 | `5` / `6`             | `5`              |
| `3 0 4 78 79 78 69 2 124 0`             | `-1` / `-2`           | `-1`             |

The translation reproduces the majority outcome of every one of them, but they
are *not* used as test inputs - a test that the reference implementation fails
one run in sixteen tests nothing.  The same applies to the operation 4
case-sensitive path when the pattern is longer than the text: after the
`size_t` underflow the C program scans the entire stack, so whether it finds a
match before dying is not reproducible either.  Only underflow inputs whose
outcome was verified stable over 400 runs (SIGSEGV) are used as tests.

Every remaining test input was executed **400 times** against the C program
without a single differing outcome, and matches this translation exactly.
