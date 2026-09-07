# CONFIGS.md — Phase A: configuration surface table (VALID inputs)

## Axes the C code actually branches on

Derived from `c_src/src/lib.c` and `c_src/include/lib.h`:

* **Runtime options / modes / flags:** *none*. The public API is a single
  function with a single `const char *` parameter. There is no global state, no
  init/teardown, no setter, no `#ifdef`, and no `switch`. So the option axis is
  a single point.
* **Public entry points:** exactly one — `custom_strdup`. It IS the lowest-level
  entry point; there are no convenience wrappers layered over anything.
* **Input shapes the code distinguishes** (`lib.c:11`, `lib.c:14`, `lib.c:20`):
  * NULL vs non-NULL (→ `ERRORS.md` row 1)
  * `strlen(str)` = 0 / 1 / small / page-crossing / large (drives `len`,
    `malloc` size, and `memcpy` count)
  * byte content: ASCII, arbitrary binary `0x01..0xFF`, embedded NUL (only the
    prefix before the first NUL is copied)
  * pointer alignment of `str` (affects nothing semantically, but exercises the
    `memcpy`/`strlen` paths that a compiler may vectorize differently)
  * whether the caller reads the result as bytes and `free()`s it (allocator
    ownership parity)

## Configuration table

Every row is exercised with **many randomized inputs, fixed seed** (a
deterministic SplitMix64 PRNG in the test file), calling BOTH `.so`s through
`libloading` and comparing the returned bytes for the full `len` byte-for-byte,
plus comparing NULL-ness of the returned pointer.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `custom_strdup` | empty string `""` (len 0, the zero boundary) | [x] |
| 2 | `custom_strdup` | single byte, randomized over all 255 non-NUL byte values | [x] |
| 3 | `custom_strdup` | short random ASCII-printable strings, lengths 2..=16 | [x] |
| 4 | `custom_strdup` | short random **binary** strings (bytes `0x01..=0xFF`), lengths 2..=16 | [x] |
| 5 | `custom_strdup` | medium random binary strings, lengths 17..=1024 | [x] |
| 6 | `custom_strdup` | page-boundary lengths: exactly 4095, 4096, 4097, 8191, 8192, 8193 bytes | [x] |
| 7 | `custom_strdup` | large strings: 64 KiB, 1 MiB, 4 MiB of random bytes | [x] |
| 8 | `custom_strdup` | input with embedded NUL(s): copy must stop at the first NUL — only the prefix + terminator are compared | [x] |
| 9 | `custom_strdup` | high-bit / non-UTF-8 byte sequences (invalid UTF-8: lone continuation bytes, truncated sequences, `0xFF` runs) | [x] |
| 10 | `custom_strdup` | unaligned `str`: pointer offset 1..=7 bytes into an allocation, randomized lengths | [x] |
| 11 | `custom_strdup` | repeated/idempotent calls: same input strdup'd many times in a loop; every returned pointer distinct from the input and from each other, all contents equal | [x] |
| 12 | `custom_strdup` | result is `free()`d via libc after each call (allocator-ownership parity for both `.so`s), across a randomized mix of the shapes above | [x] |
| 13 | `custom_strdup` | randomized fuzz sweep: 20 000 random (length, byte-content) draws from the full cross-product of the axes above, fixed seed | [x] |

## Binary / driver executable

The CMake project builds **only** `add_library(driver SHARED src/lib.c)` — there
is no `add_executable`, and the Rust `Cargo.toml` declares
`crate-type = ["cdylib"]` with no `[[bin]]`. **No binary executable exists, so
the stdout-comparison requirement is N/A.**

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only feature
combination is the default (empty) one. Verified mechanically:

```
$ grep -n '\[features\]' translation/Cargo.toml   # no match
```

`cargo test`, `cargo test --no-default-features`, and
`cargo test --all-features` therefore all resolve to the same single
configuration; all three are run by `run_all.sh`.
