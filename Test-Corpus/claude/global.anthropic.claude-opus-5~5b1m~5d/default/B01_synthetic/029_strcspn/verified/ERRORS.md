# Differential verification log — `c_src/src/main.c` vs. `translation/`

## How the two programs are run

| | command |
|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` → `c_src/build/driver` |
| Rust | `cd translation && cargo build --release` → `translation/target/release/driver` |

Both read stdin only, take no arguments, write to stdout only and always exit `0`.
`translation/tests/differential.rs` spawns **both binaries as subprocesses**, pipes the
same bytes to stdin and asserts stdout, stderr and exit status all match.

## What the C program actually does

```c
char s1[100] = "", s2[100] = "";
fgets(s1, sizeof(s1), stdin);
fgets(s2, sizeof(s1), stdin);   // note: sizeof(s1), not sizeof(s2)
s1[strlen(s1)-1] = '\0';
s2[strlen(s2)-1] = '\0';
printf("%zu\n", strcspn(s1, s2));
```

Input classes enumerated from this (each has a test):

1. **Empty stdin** — both `fgets` return `NULL`, buffers stay `""`.
2. **One line only** — second `fgets` returns `NULL`, `s2` stays `""`.
3. **Line not newline-terminated** (EOF stop) — `strlen()-1` chops a *real character*,
   not a newline (`"abc"` → `"ab"`).
4. **Line ≥ 99 bytes** — `fgets` stores at most 99 bytes, so the `'\n'` is left in the
   stream and becomes the *start of `s2`*.
5. **`strlen() == 0`** — empty string, or a line whose first stored byte is `NUL`;
   `strlen(s)-1` wraps to `SIZE_MAX` and the store goes out of bounds (UB).
6. **Embedded `NUL`** — `fgets` stores it, but `strlen`/`strcspn` stop there.
7. **`strcspn` outcomes** — match at index 0, mid-string, at the last surviving byte,
   no match at all (returns `strlen(s1)`), empty `s1`, empty `s2`.
8. **Bytes ≥ 0x80 / invalid UTF-8** — `strcspn` compares raw bytes.

## Mismatches found

**None.** Every enumerated input class, plus a 400-case deterministic randomized
sweep in `cargo test` and a separate 3,000-case ad-hoc fuzz over the alphabet
`{a, b, NUL, \n, 0xff, space, z, \r}` with lengths 0–210, produced byte-identical
stdout, byte-identical stderr and identical exit status.

Recorded here instead are the C behaviours that are easy to get wrong and that the
Rust translation was checked against, so a future reader can re-verify them:

| C behaviour | Why it could mismatch | How the Rust matches it |
|---|---|---|
| `fgets` stops at `'\n'` and **keeps** it, and does **not** read across lines | Using `read_line`/`lines()` (which strips `\n`) or `scanf`-style whitespace skipping would change the byte count by one and drop leading spaces | `c_fgets` reads one byte at a time, stores the `'\n'`, and stops |
| `fgets` returns `NULL` at immediate EOF and **leaves the buffer untouched** | Writing a `NUL` or clearing the buffer would be indistinguishable here (buffers start zeroed) but is wrong in general | `c_fgets` returns `false` without touching `buf` when 0 bytes were read |
| `fgets` stores at most `size-1` = **99** bytes | An off-by-one would move the `'\n'` into or out of the next read, changing both `s1` and `s2` | `limit = buf.len() - 1` |
| Buffer is 100 bytes and the *second* call also passes `sizeof(s1)` | Looks like a bug; harmless because both arrays are 100 bytes | Same limit used for both reads; noted in a comment |
| `strlen(s)-1` with `strlen(s) == 0` → out-of-bounds store at `s[-1]` | Rust cannot do this; a naive `s[n-1]` would panic (`attempt to subtract with overflow` in debug) | Guarded with `if n > 0`. Verified unobservable: `s1[-1]` / `s2[-1]` land on stack padding or on the neighbouring buffer's already-`NUL` final byte, so the C output is unchanged. Confirmed against the real binary for empty stdin, `"\0\n"`, `"\0\n" + 99×'a'`, and 99-byte lines followed by EOF |
| `strlen` / `strcspn` stop at the first `NUL`, even one that `fgets` stored | Using the whole 100-byte buffer, or Rust `String` semantics, would give a larger answer | `c_strlen` finds the first `NUL`; `c_strcspn` restricts both operands to their `strlen` prefixes |
| `strcspn(s1, "")` returns `strlen(s1)` (empty set matches nothing) | An empty-needle special case that returns 0 would be wrong | Loop falls through and returns `n1` |
| `strcspn` compares `unsigned char` values | Signed-`char` handling would mishandle bytes ≥ 0x80 | Operates on `u8` throughout |
| `printf("%zu\n", ...)` — decimal, no padding, single trailing `'\n'` | Extra/missing newline or padding | `write!(out, "{}\n", ...)` |
| `printf` write failure is ignored and `main` still returns `0` | A Rust `unwrap()`/`println!` panic on a closed stdout would exit non-zero | Write and flush errors are discarded; verified with stdout closed (`>&-`): both exit `0` |
| Exit status is always `0`; stderr is always empty | — | No error paths in the Rust program |

## Result

`cargo test` in `translation/`: **36 tests, 36 passed, 0 failed, 0 ignored.**
Nothing in `c_src/` was modified (only the untracked `c_src/build/` output directory
was created by the build).
