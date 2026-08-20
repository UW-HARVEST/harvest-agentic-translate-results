# ERRORS.md — Error-surface table (Phase C)

Mechanically derived from every rejection/error site in `c_src/src/goto.c`.
Grep basis (`grep -nE "return|goto|assert|NULL|error|cleanup" c_src/src/goto.c`):

* `goto.c:31` `goto error;`   guarded by `if (x < 0)`         → `error:` label at `:37`, `return -1` at `:39`
* `goto.c:45` `goto cleanup;` guarded by `if (!fp)`           → `cleanup:` label at `:59`, `return NULL` at `:62`
* `goto.c:54` `goto cleanup;` guarded by `if (ferror(fp))`    → `cleanup:` label at `:59`, `return NULL` at `:62`
* `goto.c:68` `return -1;`    guarded by `if (res == -1)`
* `goto.c:75` `return -2;`    guarded by `if (out == NULL)`

The cleanup label also runs `if (fp) fclose(fp);` — an observable *resource*
effect that no return value or output byte reveals, so it gets its own rows
(17–19), checked by counting `/proc/self/fd`.

There are **no** `assert`s, no error enums, no explicit range checks and no
min/max constants in the C source. The only "range" is that a C `int` accepts
every bit pattern, so the analogue of "an enum value with no valid variant" is
an arbitrary/extreme `int` across the FFI boundary (rows 2, 14 and
`ffi_all_int_bit_patterns_are_accepted_identically`).

Every row is implemented in `tests/phase_c_errors.rs`; each test calls **both**
`.so`s through `dlopen`/`dlsym` and compares the return value *and* the exact
bytes on fd 1 / fd 2.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `forward_goto_example` | `x < 0` → `goto error` (`x = -1`) | `stderr` = `"Error: negative input\n"`, `stdout` empty, returns `-1` | `err01_fge_negative` | [x] |
| 2 | `forward_goto_example` | `x == INT_MIN` / `INT_MIN+1` (extreme negative) | as row 1, returns `-1` | `err02_fge_int_min` | [x] |
| 3 | `forward_goto_example` | 500 seeded random `x < 0`, plus the `-1`/`0` boundary pair | `-1` + stderr message for every value | `err03_fge_random_negatives` | [x] |
| 4 | `open_with_cleanup` | `fopen` fails, `ENOENT`: path does not exist | `stderr` = `"Error: opening or processing file <path>\n"`, `fclose` **not** called (fp is NULL), returns `NULL` | `err04_owc_enoent` | [x] |
| 5 | `open_with_cleanup` | `fopen` fails, `ENOENT`: empty string path `""` | as row 4 with an empty `<path>` | `err05_owc_empty_path` | [x] |
| 6 | `open_with_cleanup` | `filename == NULL` (null pointer across FFI) | `fopen(NULL,"r")` → NULL; `fprintf("%s", NULL)` prints `(null)`; returns `NULL` | `err06_owc_null_pointer` | [x] |
| 7 | `open_with_cleanup` | `fopen` fails, `EACCES`: existing file with mode `000` | as row 4, returns `NULL` | `err07_owc_eacces` | [x] |
| 8 | `open_with_cleanup` | `fopen` fails, `ENAMETOOLONG`: 255/256/4096/5000-byte name | as row 4 (the whole oversized name is echoed), returns `NULL` | `err08_owc_enametoolong` | [x] |
| 9 | `open_with_cleanup` | `fopen` fails, `ENOTDIR`: a regular file used as a directory component | as row 4, returns `NULL` | `err09_owc_enotdir` | [x] |
|10 | `open_with_cleanup` | `fopen` **succeeds** but the read loop sets `ferror` (`EISDIR`: the path is a directory) — the *second* `goto cleanup` | stderr message, `fclose(fp)` executed, returns `NULL`; `stdout` empty | `err10_owc_ferror_directory` | [x] |
|11 | `driver` | `forward_goto_example` returned `-1` (`num < 0`) → early `return -1`; the file is never opened | returns `-1`, stderr only has the negative-input message, `stdout` empty | `err11_driver_negative_num` | [x] |
|12 | `driver` | `num < 0` **and** a bad filename (error precedence) | returns `-1` (not `-2`); no `"opening or processing"` message at all | `err12_driver_error_precedence` | [x] |
|13 | `driver` | `num >= 0` and `open_with_cleanup` returned NULL — all of rows 4–10 as filenames × `num ∈ {0,1,2,1000,0x3FFFFFFF,INT_MAX}` | `stdout` = `"Processing: n\nGoto output: 2n\n"`, then returns `-2` | `err13_driver_file_error_returns_minus2` | [x] |
|14 | `driver` | extremes: `num = INT_MIN, INT_MIN+1, -2, -1, 0, 1, INT_MAX-1, INT_MAX` with a bad file | `INT_MIN` → `-1`; `INT_MAX` → `-2` (`res` wraps to `-2`, which is *not* the `-1` sentinel) | `err14_driver_extreme_ints` | [x] |
|15 | `driver` | `filename == NULL` with `num >= 0` | returns `-2`, stderr contains `(null)` | `err15_driver_null_filename` | [x] |
|16 | `open_with_cleanup` | negative control: a zero-length but openable source (`/dev/null`, empty file) must **not** take cleanup | returns non-NULL `FILE*`, no stderr, no stdout; `driver` → `0` | `err16_empty_stream_is_not_an_error` | [x] |
|17 | `open_with_cleanup` | cleanup label's `if (fp) fclose(fp)` — 64 repetitions of each failing input (missing / directory / unreadable / NULL) | descriptor count unchanged; identical to C | `leak01_owc_error_paths_have_identical_fd_accounting` | [x] |
|18 | `open_with_cleanup` | success path ownership: the caller closes the returned stream | descriptor count unchanged; identical to C | `leak02_owc_success_path_fd_accounting` | [x] |
|19 | `driver` | `fclose(out)` on success, and no stray descriptor on the `-2` paths | descriptor count unchanged; identical to C | `leak03_driver_closes_the_stream_it_opened` | [x] |

## Generic FFI boundary cases (beyond the table)

| case | test | [x] |
|------|------|-----|
| every `int` bit pattern: 0, ±1, `INT_MIN`, `INT_MAX`, all powers of two ±1, 256 seeded randoms (stand-in for out-of-range enum values, since C `int` params accept any `int`) | `ffi_all_int_bit_patterns_are_accepted_identically` | [x] |
| NULL pointer for every pointer parameter of every entry point | `ffi_null_pointers` | [x] |
| zero-length path and oversized paths (1 … 8192 bytes, across `NAME_MAX` and `PATH_MAX`) | `ffi_zero_and_oversized_path_lengths` | [x] |
| paths whose bytes are not valid UTF-8 (`\xff`, lone continuation bytes, encoded surrogate) — success *and* failure paths | `ffi_non_utf8_paths` | [x] |
| 1000+ repeated failures, so any descriptor/memory drift diverges | `ffi_repeated_failures_do_not_drift` | [x] |

**Status: 19/19 rows + 5/5 generic boundary cases pass** under every feature
configuration (`./verify.sh`).
