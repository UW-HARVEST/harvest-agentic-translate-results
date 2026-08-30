# Verification log — `c_src/src/main.c` vs. `translation/`

## How this was verified

Both programs were built and driven as subprocesses, exactly as a shell would:

| Program | Build | Run |
|---|---|---|
| C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` | `./c_src/build/driver` |
| Rust | `cd translation && cargo build --release` | `./translation/target/release/driver` |

`translation/tests/differential.rs` runs both binaries for each input and asserts
that **stdout, stderr and exit status** all match byte for byte. It never loads
the Rust code as a library. The C binary is built on demand by the test harness
(via CMake, cached in a `OnceLock`) so `cargo test` is self-contained.

## Input classes enumerated from the C source

`main` is `int x = 0; scanf("%d", &x); driver(x); return 0;`. `driver` and
`print_hex` contain no input-dependent branches beyond the fixed 16-iteration
loop, so every branch in the program lives inside the `%d` conversion:

| # | Class | Input examples | Effect |
|---|---|---|---|
| 1 | Input failure before any conversion | `""`, `" "`, `"\n\n"`, `/dev/null` | `x` untouched → `0` |
| 2 | Matching failure (no digit after optional sign) | `"abc"`, `".5"`, `"-"`, `"- 5"`, `"--5"` | `x` untouched → `0` |
| 3 | Successful conversion | `"0"`, `"42"` | `x = value` |
| 4 | Leading whitespace skipped, incl. across newlines | `"   42"`, `"\n\n\n7"`, 100 000 spaces | whitespace consumed |
| 5 | Optional sign | `"+42"`, `"-42"`, `"-0"` | sign applied |
| 6 | Conversion stops at first non-digit | `"12abc"`, `"3 4"`, `"12.75"`, `"0x10"` | remainder unread |
| 7 | Out of range | `"2147483648"`, `"9223372036854775808"`, `"9"×1000` | glibc `strtol` saturation, then truncation to `int` |
| 8 | Non-text bytes | `"\0"`, `"\xff\xff5"`, invalid UTF-8 | byte-oriented parse |
| 9 | Redirections / argv | closed stdin, closed stdout, extra argv | no behavioural difference |

## Mismatches found

**None.** Every input class above produced identical stdout, stderr and exit
status on the first differential run. In addition to the 19 named tests, an
ad-hoc sweep of **3 436** inputs (3 000 random byte strings over the parser's
alphabet, plus 400 random integers in ±2⁷⁰ and the range edges) produced **0**
mismatches, so no change to `translation/src/main.rs` was required.

The pre-existing translation already handles the subtle points correctly. They
are recorded here because they are the places a translation *would* have gone
wrong, and each now has a regression test:

### 1. Struct object representation and byte order

`print_hex((unsigned char *)&house, sizeof(house))` dumps the raw object
representation of

```c
typedef struct { int floors; int bedrooms; double bathrooms; } house_t;
```

On the x86-64 SysV ABI this is 16 bytes with offsets 0 / 4 / 8 and **no padding**
(`double` is 8-byte aligned and offset 8 is already aligned). Rust rebuilds this
explicitly with `to_le_bytes()` rather than transmuting a `#[repr(C)]` struct,
which would have been correct here but is fragile. `bathrooms = 2.0` is
`0x4000000000000000`, little-endian `0000000000000040` — visible as the tail of
every expected output, e.g. `2a000000030000000000000000000040` for input `42`.

*Would-be bug:* dumping big-endian, or assuming padding between `bedrooms` and
`bathrooms`, or using `#[repr(Rust)]` field ordering.

### 2. `scanf` failure leaves the variable untouched

On input failure or matching failure `scanf` returns `EOF`/`0` and never writes
through `&x`, so `x` keeps its initializer `0`. Rust models this by returning
`Option<i32>` from `scanf_d` and only assigning on `Some`. Note the program still
prints normally and still exits `0` — there is no error path to stderr and no
non-zero exit anywhere in this program.

*Would-be bug:* treating a parse failure as an error (printing to stderr, or
`exit(1)`), which would differ in both stderr and exit status while stdout
happened to look plausible.

### 3. Out-of-range saturation is `long`-shaped, not `int`-shaped

glibc's `%d` collects the digits and passes them to `strtol`, which saturates to
`LONG_MAX` / `LONG_MIN` on overflow; the result is then truncated on assignment
to `int`. So:

- `"9223372036854775808"` → `LONG_MAX` → `(int)-1` → `ffffffff`
- `"-9223372036854775809"` → `LONG_MIN` → `(int)0` → `00000000`
- `"2147483648"` (fits in `long`) → truncates to `INT_MIN` → `00000080`

*Would-be bug:* saturating at `i32::MAX`/`i32::MIN` instead of the `long` range.
That gets `"2147483648"` right by accident but prints `ffffff7f` instead of
`ffffffff` for `"9223372036854775808"` — a mismatch only reachable with a
> 2⁶³ input. Covered by `long_range_edges_and_saturation`.

### 4. `%d` is decimal-only and reads across newlines

`"0x10"` parses as `0` and stops at `x`; `"010"` is `10`, not octal `8`; leading
whitespace including newlines is skipped, unlike `fgets`. Covered by
`leading_zeros_are_decimal_not_octal` and
`scanf_skips_leading_whitespace_across_newlines`.

### 5. Input is bytes, not UTF-8

The Rust reader works over `u8` from a locked `Stdin` and never calls
`read_to_string`/`lines()`, so NUL bytes, `0xff` and invalid UTF-8 are handled
the same way C handles them: as ordinary non-digit characters that end (or fail)
the conversion. A `String`-based reader would have errored or panicked here.
Covered by `non_utf8_and_nul_bytes`.

### 6. Output is written, not `println!`ed

`print_hex` builds the line and writes it with `write_all`, discarding the
result — matching C's `printf`, whose return value `main` also ignores. With a
closed stdout both programs stay silent and still exit `0`; `println!` would have
panicked, producing stderr output and exit status `101`. Verified manually with
`>&-` on both binaries.

## Completion gate

- [x] Both programs build with no errors and no warnings.
- [x] Every enumerated input class produces identical stdout, stderr and exit status.
- [x] `cargo test` passes in `translation/` (19 tests, debug and `--release`).
- [x] No test is disabled, skipped or `#[ignore]`d.
- [x] Nothing in `c_src/` was modified (only the untracked `c_src/build/` output directory was created).
