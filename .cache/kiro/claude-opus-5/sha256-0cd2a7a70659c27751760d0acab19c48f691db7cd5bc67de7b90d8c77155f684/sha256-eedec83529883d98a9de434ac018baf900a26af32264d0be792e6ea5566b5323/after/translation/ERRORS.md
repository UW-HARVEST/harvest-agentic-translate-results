# Differential verification log

Comparison method: build both executables, feed each the same bytes on stdin,
and require byte-identical stdout, byte-identical stderr, and an identical exit
status. The Rust code is never loaded as a library.

```
# C reference
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
#   -> c_src/build/driver

# Rust
cd translation && cargo build --release
#   -> translation/target/release/driver

# the suite
cd translation && cargo test
#   (add RUST_DRIVER=target/release/driver to test the release binary instead
#    of the one cargo builds for the test profile)
```

The suite is `translation/tests/differential.rs` (51 tests). It locates the C
binary at `c_src/build/driver`, and if that is absent configures and builds it
*out of source* into `translation/target/c_ref` so `c_src/` is never written to.

---

## Mismatches found

### 1. Exit status on a closed stdout pipe: C died on SIGPIPE, Rust exited 0

**Symptom**

```
$ ./c_src/build/driver   < big_input | head -c 10 >/dev/null ; echo ${PIPESTATUS[0]}
141
$ ./translation/target/release/driver < big_input | head -c 10 >/dev/null ; echo ${PIPESTATUS[0]}
0        # <-- mismatch
```

Reproduce with an input that makes the program emit more than a pipe buffer's
worth of stdout (e.g. 100 distinct words analyzed, then menu option `3` a few
hundred times) and a reader that closes early.

**Cause**

The Rust standard library sets `SIGPIPE` to `SIG_IGN` before `main` runs. A
write to a closed pipe therefore returns `EPIPE`, the error is discarded, and
the program runs to completion and exits 0. The C program keeps the default
disposition and is killed by the signal, so the shell reports 141
(128 + SIGPIPE). stdout and stderr agreed; only the exit status differed.

**Fix** (`translation/src/cio.rs`, called first thing in `main`)

```rust
pub fn restore_default_sigpipe() {
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

Both now report 141. This was the only mismatch found.

---

## C behaviour that looks like a bug and was deliberately kept

Each of these was confirmed against the C binary and is asserted by the suite.
They are listed because they are exactly what a "tidying" edit would break —
see the harness-validation section at the end.

| # | C behaviour | Where |
|---|---|---|
| 1 | `if (c == '/' && (peek_char() == '/' \|\| peek_char() == '*'))` calls `peek_char()` *before* consuming, so it still returns `c`. The condition collapses to `c == '/'`, and **every** slash starts a comment — `a / b` tokenizes the `/` as `COMMENT`, and division is never an `OPERATOR`. | `tokenizer.c` `tokenizer_next_token` |
| 2 | `analyze_text` counts `TOKEN_NEWLINE` into `result.line_count`, then immediately overwrites it with the tokenizer's process-wide `total_lines_processed`. `char_count` is likewise cumulative. Neither is ever reset, so repeated analyses report growing totals. | `analyzer.c` `analyze_text` |
| 3 | `token.column = current_column - token.length` mixes `int` and `size_t`. For a block comment spanning newlines `current_column` has been reset to 1, so the reported column is **negative** (e.g. `Line 2, Column -2`). | `tokenizer.c` `create_token` |
| 4 | `count++` happens before `if (count > 100)`, so 101 tokens are printed before `... (truncated, too many tokens)`. | `main.c` `interactive_tokenizer` |
| 5 | `strstr(token.value, "")` is never NULL, so an empty search pattern matches every token. | `analyzer.c` `find_patterns` |
| 6 | A file of exactly `MAX_BUFFER_SIZE` (8192) bytes passes `read_file`'s `size > MAX_BUFFER_SIZE` check, then fails `tokenizer_load_text`'s `length >= MAX_BUFFER_SIZE` check — producing *two* stderr lines (`Error: Input text too large`, `Error: Failed to load text`) and an all-zero result. 8193 bytes instead produces only `Error: File too large`. | `main.c` / `tokenizer.c` |
| 7 | On EOF at the "Enter filename:" / "Enter pattern to search:" prompt, `break` leaves only the `switch`, so the menu is printed **once more** before the loop exits. | `main.c` cases 2 and 5 |
| 8 | `scan_number` breaks on a second `.`, so `1.2.3` is `1.2` then `.` then `3`; and `.` alone is punctuation. | `tokenizer.c` `scan_number` |
| 9 | The bubble sort in `print_token_distribution` swaps on `<`, not `<=`, and mutates the stored static arrays — so calling option 3 twice re-sorts an already sorted array, and equal counts keep insertion order. | `analyzer.c` |
| 10 | `scan_string`'s loop bound is `MAX_TOKEN_LENGTH - 2` but the escape branch appends two bytes per iteration, so the assembled buffer can reach 256 (a one-byte overrun of `char buffer[256]` in C) before `create_token` truncates the stored value to 255. The Rust version reproduces the observable 255-byte truncation. | `tokenizer.c` `scan_string` |
| 11 | `sscanf(input, "%d", &choice)` converts via `strtol` (saturating at `LONG_MIN`/`LONG_MAX`) and then stores the low 32 bits into an `int`. So `4294967296` becomes `0` (→ "Invalid choice") and `4294967303` becomes `7` — it actually **exits**. `9223372036854775808` saturates to `LONG_MAX` and truncates to `-1`. | `main.c` |
| 12 | `input[strcspn(input, "\n")] = 0` only trims the newline; leading/trailing spaces stay part of the filename, and a NUL earlier in the buffer already ends the string. | `main.c` cases 2 and 5 |
| 13 | Menu-choice and text lines are read with `fgets` into 256-byte buffers, so a longer line is split and its remainder is consumed as the *next* menu choice. | `main.c` |
| 14 | `strncat(text, line, MAX_INPUT_SIZE - strlen(text) - 1)` caps the accumulated text at 4095 bytes, truncating mid-token and silently dropping every later line. | `main.c` cases 1 and 6 |
| 15 | No `setlocale`, so `isalpha`/`isdigit`/`isspace` are ASCII-only and bytes ≥ 0x80 become `ERROR` tokens. Non-UTF-8 bytes must be echoed through unchanged. | `tokenizer.c` |
| 16 | `token_type_counts` is only updated by `analyze_text`; menu options 5 and 6 tokenize without touching it, but they *do* advance the tokenizer's cumulative char/line counters and replace the loaded buffer. | `analyzer.c` / `main.c` |

## Unreachable code, verified as unreachable rather than tested

- `tokenizer_peek_token` / the `lookahead_valid` path — `get_tokenizer_ops`
  exposes it, but no caller in `main.c` or `analyzer.c` ever invokes it.
- `analyze_text`'s `if (!initialized)` — `analyzer_init` is called
  unconditionally at the top of `main`.
- `tokenizer_load_text`'s `if (!text)` and `interactive_tokenizer`'s
  `"Failed to load text"` — the caller always passes a 4096-byte stack buffer,
  whose `strlen` is at most 4095, well under `MAX_BUFFER_SIZE`.
- `read_file`'s `malloc` failure branch.

## Path deliberately not asserted

`read_file` on an unseekable file (e.g. `/dev/stdin`): `fseek` fails, `ftell`
returns `-1`, so C reaches `malloc(0)` followed by
`fread(content, 1, (size_t)-1, file)`. That is undefined behaviour whose outcome
depends on how much data happens to be left in the pipe. It was observed to
agree in practice (stdin is already drained, so `fread` returns 0), but it is
not asserted, because the C program has no well-defined behaviour to match.

## Harness validation

To confirm the suite is not vacuous, bugs were injected into the Rust source and
the suite was re-run:

| Injected change | Result |
|---|---|
| truncation threshold `count > 100` → `count > 99` | 3 tests fail |
| "fixing" quirk 1 so a lone `/` is an operator, "fixing" quirk 2 so `line_count` keeps the newline tally, and "fixing" quirk 5 so an empty pattern matches nothing | 14 tests fail |

All injected changes were reverted; the suite passes 51/51 in both the debug and
release profiles.

Beyond the suite, roughly 1,000 generated inputs (structured menu sessions plus
raw random byte streams over a pool including `\0`, `\xff`, quotes, slashes and
newlines) were run through both binaries with no remaining differences.
