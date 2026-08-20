# ERRORS.md — Phase A: error / rejection surface table

## How this table was derived

Mechanical grep of the only translation unit, `c_src/src/main.c`:

```
$ grep -nE 'return|assert|RETURN|NULL|-1|if|switch|error|errno|exit|perror|fprintf|stderr' c_src/src/main.c
33:    return 0;
$ grep -nE '#\s*(ifdef|ifndef|if |define)' c_src/src/main.c      -> NONE
$ grep -nE 'argc|argv|getenv|getopt|option'  c_src/src/main.c      -> NONE
$ grep -oE '[0-9]+' c_src/src/main.c                              -> 0, 128 (and the 2025 copyright year)
```

The entire program is:

```c
int main() {
    char text[128];
    while (fgets(text, 128, stdin)) {
        fputs(text, stdout);
    }
    return 0;
}
```

So there are **no** `RETURN_ERROR` macros, no `assert`s, no error enums, no
explicit range/null checks, no `errno` inspection and no diagnostics on stderr.
`return 0` is the only `return`, and it is unconditional: **the program has
exactly one exit code, 0** — every "error" is absorbed silently.

That makes the real rejection surface implicit, and it lives in three places:

1. the conditions under which `fgets` returns `NULL` (the only thing that ends
   the loop),
2. the truncation `fputs` performs at the first NUL byte (input the program
   silently drops),
3. the `128` buffer bound (the only numeric limit in the program), and
4. failures of `fputs`, whose return value is **discarded**, so they are
   *not* rejections at all — the loop keeps running and stdin is still drained.

Every row below is one distinct such condition, with the C result stated as the
externally observable outcome (exit status + bytes on stdout), because that is
the program's whole interface.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `fgets` | stdin is at EOF immediately (empty input) — `fgets` returns `NULL` on the first call, loop body never runs | exit 0, zero bytes on stdout | `err_01_empty_stdin_immediate_eof` | [x] |
| 2 | `fgets` | stdin closed before exec (`fd 0` closed) — `read` fails `EBADF`, `fgets` returns `NULL` | exit 0, zero bytes on stdout, nothing on stderr | `err_02_stdin_closed_ebadf` | [x] |
| 3 | `fgets` | stdin is a **directory** — `read` fails `EISDIR`, `fgets` returns `NULL` | exit 0, zero bytes on stdout | `err_03_stdin_is_directory_eisdir` | [x] |
| 4 | `fgets` | stdin is `/dev/null` — immediate EOF, same as row 1 via a different stream kind | exit 0, zero bytes on stdout | `err_04_stdin_dev_null` | [x] |
| 5 | `fgets` | EOF reached *after* ≥1 byte with **no trailing newline** — `fgets` must return the partial chunk (NOT `NULL`), then `NULL` on the next call | exit 0, the partial last line echoed verbatim, no newline added | `err_05_eof_without_trailing_newline` | [x] |
| 6 | `fgets` | a "line" longer than the `128` buffer — `fgets` may store at most 127 bytes, so the over-long line is **split**, not rejected or truncated | exit 0, output byte-identical to input (reassembled from 127-byte chunks) | `err_06_line_exceeds_buffer_127_split` | [x] |
| 7 | `fputs` | input chunk contains a NUL byte in the middle — `fputs` stops at the NUL, silently dropping the rest of that chunk | exit 0, chunk truncated at the first NUL (input bytes are lost) | `err_07_embedded_nul_truncates_chunk` | [x] |
| 8 | `fputs` | input chunk *starts* with a NUL byte — `fputs` writes nothing for that chunk, but the loop continues | exit 0, that whole chunk produces zero output | `err_08_leading_nul_writes_nothing` | [x] |
| 9 | `fputs` | input is **only** NUL bytes and no newline — `fgets` succeeds (non-`NULL`), `fputs` writes nothing, next `fgets` returns `NULL` | exit 0, zero bytes on stdout even though input was non-empty | `err_09_all_nul_input_no_output` | [x] |
| 10 | `fputs` | NUL as the byte right at the 127-byte chunk boundary — truncation point interacts with the buffer bound | exit 0, first chunk emits 126 bytes; the chunk that *begins* with the NUL emits nothing | `err_10_nul_at_chunk_boundary` | [x] |
| 11 | `fputs` | stdout closed before exec (`fd 1` closed) — every `fputs` fails `EBADF`; **the return value is ignored**, so this is not a rejection: the loop still consumes all of stdin | exit 0, no output, no stderr, stdin fully drained | `err_11_stdout_closed_ebadf_still_drains` | [x] |
| 12 | `fputs` | stdout is a pipe whose reader has exited — the write raises **SIGPIPE**, whose default disposition kills the process | killed by signal 13 (`SIGPIPE`), i.e. wait-status `-13` / shell status 141 — *not* a clean exit 0 | `err_12_broken_pipe_dies_with_sigpipe` | [x] |
| 13 | `fputs` | stdout is `/dev/full` — every flush fails `ENOSPC`, ignored exactly like row 11 | exit 0, stdin fully drained, no stderr | `err_13_stdout_dev_full_enospc` | [x] |
| 14 | `main` | command-line arguments supplied — `int main()` takes no parameters and nothing greps `argc`/`argv`, so arguments cannot be rejected | exit 0, arguments ignored entirely, output depends only on stdin | `err_14_arguments_ignored` | [x] |
| 15 | `main` (FFI) | the exported `main` is called with a garbage/out-of-range `int` return expectation — the C function takes **no parameters**, so there is no argument to put out of range; it must return exactly `0` every time, for every input, including all rows above | return value `0` from the `.so` on every invocation | `so_differential_all` (asserts `rc == 0` for every case) | [x] |

### Notes on the generic FFI boundary checks the prompt also asks for

* **Null pointers / oversized lengths / one-past-range values**: `main` has the
  signature `int main(void)` — it accepts **no pointer, length, or enum
  arguments**, so there is no in-band argument to fuzz. The equivalent
  boundary surface is the *stream* state, which rows 1–13 cover (closed fd,
  unreadable fd, unwritable fd, zero-length input, oversized input).
* **Out-of-range enum values across FFI**: the C source declares no `enum` and
  its only function takes no arguments, so this class does not exist here. The
  nearest analogue — a return value with no valid variant — is pinned by row 15
  (`main` must always return exactly `0`).
* **The `128` bound**: exercised at 126 / 127 / 128 / 129 bytes and at multiples
  of 127 in `CONFIGS.md` rows 7–11, plus row 6 above.
