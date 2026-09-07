# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/goto.c` (the only C source file). Every
rejection/error construct in the file was enumerated:

```sh
grep -n -E 'goto|return -1|return -2|return NULL|assert|if *\(!|< 0|== NULL|ferror' c_src/src/goto.c
```

Findings: 3 `goto` targets/branches (`x < 0`, `!fp`, `ferror(fp)`), 2 error
returns in `driver` (`res == -1`, `out == NULL`), 1 defensive null check
(`if (fp) fclose(fp)`), 0 `assert`s, 1 size constant (`char buffer[100]`,
`sizeof(buffer)` = 100 passed to `fgets`). One row per distinct rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `forward_goto_example` | `x < 0` (`goto error`) — e.g. `-1`, `INT_MIN`, random negatives | stderr gets exactly `Error: negative input\n`; nothing on stdout; returns `-1` | [x] |
| 2 | `forward_goto_example` | boundary `x == 0` is **not** rejected (`0 < 0` false) | stdout `Processing: 0\n`; returns `0` (not an error) | [x] |
| 3 | `forward_goto_example` | `x == INT_MIN` (most negative, one step past `-1`-style range) | same as row 1: stderr message, returns `-1` | [x] |
| 4 | `open_with_cleanup` | `fopen(filename,"r")` returns NULL — nonexistent path (`ENOENT`) → `goto cleanup` with `fp == NULL` | stderr `Error: opening or processing file <name>\n`; **no** `fclose` (guard `if(fp)` false); returns `NULL` | [x] |
| 5 | `open_with_cleanup` | `fopen` fails, empty filename `""` (`ENOENT`) | as row 4, `%s` prints empty string | [x] |
| 6 | `open_with_cleanup` | `fopen` fails, unreadable file (mode `0000`, `EACCES`) | as row 4 | [x] |
| 7 | `open_with_cleanup` | `filename == NULL` — null pointer straight into `fopen`, then into `fprintf`'s `%s` | whatever glibc does for both (`fopen` fails ⇒ `EFAULT`; `%s` prints `(null)`); returns `NULL`. Rust must match byte-for-byte | [x] |
| 8 | `open_with_cleanup` | `ferror(fp)` non-zero after the `fgets` loop → `goto cleanup` with `fp != NULL`, so the shared label **does** `fclose(fp)`. Triggered by `fopen`-ing a **directory** (glibc opens it, `fgets` fails `EISDIR`) | stdout empty; stderr `Error: opening or processing file <dir>\n`; returns `NULL` | [x] |
| 9 | `open_with_cleanup` | pathological long path (`> PATH_MAX`, `ENAMETOOLONG`) | as row 4 | [x] |
| 10 | `driver` | `res == -1`, i.e. `num < 0` — returns before ever touching `filename` (so an invalid/NULL filename is never observed) | stderr `Error: negative input\n` only; returns `-1` | [x] |
| 11 | `driver` | `out == NULL` — `num >= 0` but `open_with_cleanup` failed (any of rows 4–9) | stdout `Processing: <num>\nGoto output: <2*num>\n`, stderr the file error; returns `-2` | [x] |
| 12 | `driver` | `num < 0` **and** `filename == NULL` simultaneously | short-circuits at row 10: returns `-1`, no `(null)` on stderr | [x] |
| 13 | `fgets` size constant | line longer than `sizeof(buffer)-1 == 99` bytes: not an error, but the 100-byte cap must split identically | identical `printf("%s")` chunks; returns the open `FILE*` | [x] |
| 14 | `forward_goto_example` | signed-overflow inputs `x >= 0x40000000` where `x * 2` wraps negative (C UB, in practice wrapping) | stdout `Processing: <x>\n`; returns wrapped `x*2`; and `driver` prints `Goto output: <wrapped>` because the wrapped value is even and thus never `-1` | [x] |

Notes on things that are **not** in the table because the C does not check them:

* There are no enums anywhere in the API, so there is no out-of-range-enum case
  to pass across FFI. The only non-pointer parameter is `int num`, whose entire
  `INT_MIN..=INT_MAX` domain is exercised (rows 1–3, 14 plus randomized values in
  Phase B).
* `driver`'s return value `-1` is ambiguous with a legitimate
  `forward_goto_example` result only in theory — `x*2` is always even — so no
  extra row.
* On the success path `open_with_cleanup` returns a still-open `FILE*` at EOF;
  the C never validates it. Tests assert non-NULL + `feof != 0` + `ferror == 0`
  for both libraries, then `fclose` it (Phase B).
