# ERRORS.md — error / rejection surface of `c_src/src/main.c`

## Mechanical derivation

The whole C translation unit is 17 lines of code:

```c
void driver(const char *s1, const char *s2) {
    printf("%zu\n", strcspn(s1, s2));
}

int main() {
    char s1[100] = "", s2[100] = "";
    fgets(s1, sizeof(s1), stdin);
    fgets(s2, sizeof(s1), stdin);

    s1[strlen(s1)-1] = '\0';
    s2[strlen(s2)-1] = '\0';

    driver(s1, s2);
    return 0;
}
```

Greps used to find every rejection/error construct:

```sh
grep -nE 'RETURN_ERROR|return +-|return +NULL|assert|errno|exit\(|abort\(|perror|fprintf\(stderr' c_src/src/main.c   # -> no matches
grep -nE 'if|switch|\?|&&|\|\||while|for'                                                                            c_src/src/main.c   # -> only line 7, a license comment ("modIFy")
grep -nE '#if|#ifdef|#define|getenv|argc|argv'                                                                       c_src/src/main.c   # -> no matches
grep -nE '[0-9]+|sizeof'                                                                                             c_src/src/main.c   # -> 100, 100, sizeof(s1) x2, "-1" x2
```

**The C code contains no `if`, no `assert`, no error return, no `errno` check and
no explicit range/NULL validation at all.** Every rejection is therefore
*implicit*: it comes from the libc calls (`fgets` returning `NULL`, `strlen`/
`strcspn` stopping at a NUL byte, the fixed 100-byte buffers) or from the
unconditional out-of-bounds write `s[strlen(s)-1]` when the string is empty.
The rows below enumerate each of those distinct paths; the "expected C result"
column is the *measured* behaviour of the reference build (never a guess).

Ignored return values are themselves part of the surface: both `fgets` results
are discarded, so an input failure is silently turned into an empty string.

## Error-surface table

| #   | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|-----|----------|------------------------------------------|-------------------|------|-----|
| E1  | `main` / `fgets(s1,…)` | stdin at EOF before any byte (empty stdin) | `fgets` returns NULL (ignored), `s1` stays `""`; prints `0\n`; exit 0 | `err_e1_empty_stdin` | [x] |
| E2  | `main` / `fgets(s2,…)` | only one line on stdin (EOF for 2nd read) | `fgets` returns NULL (ignored), `s2` stays `""`; prints `strlen(s1)`; exit 0 | `err_e2_single_line` | [x] |
| E3  | `main` / `fgets` | stdin closed (fd 0 not open → read fails `EBADF`) | both `fgets` return NULL; prints `0\n`; exit 0; **no stderr output** | `err_e3_stdin_closed` | [x] |
| E4  | `main` / `fgets` | stdin is a **directory** (read fails `EISDIR`) | both `fgets` return NULL; prints `0\n`; exit 0 | `err_e4_stdin_is_dir` | [x] |
| E5  | `main` | `s1[strlen(s1)-1]='\0'` with `strlen(s1)==0` → out-of-bounds write at index `(size_t)-1` (UB) | observably a no-op: `s1` stays `""`; prints normally; exit 0 (no crash) | `err_e5_oob_write_s1` | [x] |
| E6  | `main` | `s2[strlen(s2)-1]='\0'` with `strlen(s2)==0` → same OOB write for `s2` | observably a no-op; `s2` stays `""`; exit 0 | `err_e6_oob_write_s2` | [x] |
| E7  | `main` / `fgets` | line **longer than `sizeof(s1)-1 = 99`** bytes (truncation boundary) | only the first 99 bytes are stored (no `\n`), the chop then deletes a *real data* byte (98 left); the remainder of the line feeds the *second* `fgets` | `err_e7_line_over_99` | [x] |
| E8  | `main` | last line has **no trailing newline** | the chop deletes a real data byte instead of `\n` | `err_e8_no_trailing_newline` | [x] |
| E9  | `main` / `strlen` | line contains an **embedded NUL byte** | `strlen` stops at the NUL: chop cuts at `first_nul-1`, everything after the NUL is invisible to `strcspn` | `err_e9_embedded_nul` | [x] |
| E10 | `main` / `strlen` | line's **first byte is NUL** → `strlen==0` | drops into the E5/E6 OOB-write path; string stays `""` | `err_e10_leading_nul` | [x] |
| E11 | `driver` (FFI) | `s1 == NULL` | `strcspn` dereferences NULL → process killed by **SIGSEGV** (11), no output | `err_e11_null_s1` | [x] |
| E12 | `driver` (FFI) | `s2 == NULL` | `strcspn` dereferences NULL → **SIGSEGV** (11), no output | `err_e12_null_s2` | [x] |
| E13 | `driver` (FFI) | both `s1 == NULL` and `s2 == NULL` | **SIGSEGV** (11), no output | `err_e13_null_both` | [x] |
| E14 | `driver` (FFI) | zero-length `s1` (`""`) | `strcspn("", s2) == 0` → prints `0\n` | `err_e14_empty_s1` | [x] |
| E15 | `driver` (FFI) | zero-length `s2` (`""`) — empty reject set | `strcspn(s1, "") == strlen(s1)` → whole string accepted | `err_e15_empty_s2` | [x] |
| E16 | `driver` (FFI) | oversized inputs (far past the program's own 100-byte buffers: 4 KiB / 64 KiB strings) | no length limit in `driver`; prints the full `size_t` value | `err_e16_oversized` | [x] |
| E17 | `driver` (FFI) | bytes `>= 0x80` (`char` is signed on x86-64; a sign-extended translation would mis-index) | `strcspn` compares raw bytes → high bytes match normally | `err_e17_high_bytes` | [x] |
| E18 | `driver` (FFI) | `s1`/`s2` containing an **interior NUL** (string ends early) | both `strlen`/`strcspn` stop at the first NUL | `err_e18_interior_nul` | [x] |
| E19 | `main` | more than 2 lines on stdin (surplus input) | surplus is silently ignored; exit 0 | `err_e19_surplus_lines` | [x] |
| E20 | `main` | exit status: there is **no** failure path — `return 0` is unconditional, even for E3/E4 | exit code 0 for every input | `err_e20_exit_code_always_zero` | [x] |
| E21 | `driver` / `main` (`printf`) | stdout is a pipe **with no reader** → `printf` write gets `EPIPE` | the process is killed by **SIGPIPE** (13) — C keeps the default disposition. *(A Rust binary starts with `SIGPIPE` ignored; the translation restores `SIG_DFL` in `src/main.rs` to match. The `.so` never touches signal dispositions, exactly like the C `.so`.)* | `err_e21_broken_stdout_pipe_executables`, `err_e21b_broken_stdout_pipe_shared_objects` | [x] |
| E22 | `driver` (`printf`) | stdout is `/dev/full` → write fails with `ENOSPC` | `printf`'s return value is ignored → no output, exit 0, nothing on stderr | `err_e22_stdout_enospc` | [x] |
| E23 | `driver` (`printf`) | file descriptor 1 closed → write fails with `EBADF` | no output, exit 0, nothing on stderr | `err_e23_stdout_closed` | [x] |
| E24 | `driver` (FFI) | non-NULL but **invalid** pointer (address `0x1`) for `s1` / `s2` / both | `strcspn` faults → **SIGSEGV** (11), no output | `err_e13b_bogus_pointer` | [x] |
| E25 | `driver` (FFI) | pointer to a buffer whose **first byte is the terminator** (`""` and `"\0"`) — the zero-length boundary | accepted, `strcspn` returns 0 → prints `0\n` | `err_generic_zero_length_pointers` | [x] |
| E26 | `driver` (FFI) | byte-value edges one step past each interesting range: `0x01`, `0x7f`, `0x80`, `0xff`, plus a NUL inside the reject set | raw-byte comparison; a NUL terminates the reject set so nothing is rejected | `err_generic_byte_range_edges` | [x] |
| E27 | `main` | stdin length exactly at / one below / one above every internal size constant (0, 1, 97, 98, 99, 100, 101, 197…201) | truncation/chop behaviour changes at 99 only | `err_generic_size_constants` | [x] |

## Not applicable

* No enum type crosses the FFI boundary (`driver` takes two `const char *`), so
  there is no "out-of-range enum value" case. The pointer-shaped inputs are
  covered instead by rows E11–E13 (NULL) and E16 (oversized).
* No integer parameter, length argument, mode flag or output-buffer argument
  exists in the public API, so there are no range checks to violate.
