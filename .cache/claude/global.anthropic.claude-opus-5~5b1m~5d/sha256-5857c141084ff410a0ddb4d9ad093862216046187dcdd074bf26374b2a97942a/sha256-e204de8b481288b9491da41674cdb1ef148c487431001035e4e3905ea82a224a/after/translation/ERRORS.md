# Differential verification log — `c_src/src/main.c` vs `translation/src/main.rs`

The C program is 5 lines, but almost all of its behaviour lives in
`scanf("%d %d", &x, &y)` and in the undefined behaviour of `div()`. Every
mismatch below was found by running both binaries on the same stdin and
diffing stdout / stderr / exit status.

## Enumerated input classes (what the C actually branches on)

| Class | Example input | C behaviour |
|---|---|---|
| both conversions succeed | `7 3` | `quotient: 2, remainder: 1`, exit 0 |
| empty input | `` (EOF) | both conversions are input failures, `x = y = 1` → `quotient: 1, remainder: 0` |
| whitespace only | `" \t\n"` | same as empty |
| one item only | `5` | second conversion is an input failure, `y` stays `1` |
| matching failure on first item | `abc`, `-  5`, `+`, `.5` | nothing stored, `x = y = 1` |
| matching failure on second item | `5 x`, `5 -` | `x = 5`, `y` stays `1` |
| values split across newlines | `1\n2` | `%d` skips newlines, so `x = 1, y = 2` |
| extra trailing input | `9 4 7 extra` | ignored |
| out-of-range magnitude | `99999999999999999999999` | saturates then truncates (see below) |
| `INT_MAX` / `INT_MIN` | `2147483647 1` | exact |
| divide by zero | `10 0`, `0 0` | killed by **SIGFPE** (no output, status = signal 8) |
| `INT_MIN / -1` | `-2147483648 -1` | killed by **SIGFPE** |
| non-text bytes | `7\0 3`, `\xff7 3` | NUL/high bytes are just non-digits |

## Mismatches found and their causes

### 1. Out-of-range integers: `%d` goes through a `long`, then truncates

*Input:* `99999999999999999999999 5`

A first-cut translation that parsed with `str::parse::<i32>()` and fell back to
the initial value on error printed `quotient: 1, remainder: 0`. The C prints
`quotient: 0, remainder: -1`.

*Cause:* glibc's `%d` conversion is `strtol`-based: it accumulates into a
`long`, saturates at `LONG_MAX` / `LONG_MIN` on overflow (the conversion still
*succeeds*, so the value **is** stored), and the resulting `long` is then
truncated when assigned through the `int *` argument.
`99999999999999999999999` → `LONG_MAX` = `0x7FFF_FFFF_FFFF_FFFF` → low 32 bits
`0xFFFF_FFFF` → `-1`.

*Fix:* `scan_int` accumulates in `i64` with saturation to `i64::MIN`/`i64::MAX`
and then truncates with `value as u64 as u32 as i32`. Verified against
`4294967296` → `0`, `2147483648` → `-2147483648`,
`-9223372036854775808` → `0`, `18446744073709551617` → `LONG_MAX` → `-1`.

### 2. Values that fail to convert must keep the initial `1`

*Input:* `abc`, and `5 x`

Setting `x = 0` / `y = 0` on a parse error (the natural Rust `unwrap_or(0)`
reflex) diverges twice over: it prints `quotient: 0` instead of
`quotient: 1`, and `y = 0` turns a clean exit into a SIGFPE crash.

*Cause:* `scanf` stores nothing for a conversion that ends in a matching or
input failure, so the initialisers `int x = 1, y = 1;` survive. Also, once the
first conversion fails, `scanf` returns immediately and the second conversion is
never attempted — so `y` cannot be set if `x` was not.

*Fix:* `main` only attempts the second `scan_int` inside the `if let Some(v)`
of the first, and leaves `x`/`y` at `1` otherwise.

### 3. Sign pushback on a matching failure

*Input:* `- 5`, `+ `, `-  5 2`

Consuming the `-`/`+` and then treating `5` as the *second* value produced
`quotient: 1, remainder: 0` from a different code path in an early version and
gave wrong results for inputs like `- 5 2`.

*Cause:* `%d` allows an optional sign but requires at least one digit right
after it; when the digit is missing the whole directive is a matching failure
and `scanf` stops. Whether the sign is pushed back is invisible here because
`scanf` never reads again.

*Fix:* `scan_int` resets `*pos` to the start of the token and returns `None`
when no digit follows the sign; the caller then does not attempt `y`.

### 4. Divide-by-zero must be SIGFPE, not a Rust panic

*Input:* `10 0`, `0 0`, `-2147483648 -1`

Rust's `/` panics: exit status 101 (or SIGABRT with `panic = "abort"`) plus a
`thread 'main' panicked...` line on stderr. The C binary emits *nothing* on
stderr and is killed by signal 8 (`SIGFPE`), which a shell reports as 136.
A stdout-only test passes here while stderr and status both differ — this is
exactly the case the task warns about.

*Cause:* `div(x, 0)` and `div(INT_MIN, -1)` are undefined behaviour in C; on
x86-64 the compiler emits a bare `idiv`, and the CPU raises `#DE` → `SIGFPE`.

*Fix:* `c_div` performs the division with inline `cdq; idiv` on x86-64, so the
same hardware fault is raised at the same point, with no output and no stderr.
A non-x86-64 fallback calls `std::process::abort()`; the dedicated test
`division_fault_is_a_signal_not_an_exit_code` asserts a *signal* (not an exit
code) on both sides so a panic can never be mistaken for a match.
Note `5 4294967296` also faults: the denominator truncates to `0`.

### 5. Truncation can *create* the `INT_MIN / -1` fault

*Input:* `-2147483648 4294967295`

`4294967295` fits in a `long`, truncates to `-1`, and the division then
overflows → SIGFPE. Covered by `division_faults_match`.

## Non-issues confirmed by testing (behaviour that already matched)

* Reading all of stdin up front instead of lazily: the program never reads
  again, so buffering is unobservable. Verified with 9000-byte inputs, closed
  stdin and `/dev/null` stdin.
* `printf` spacing and the single trailing `\n` are byte-compared.
* stderr is empty on every input class tested.
* 400 deterministic pseudo-random inputs over the alphabet
  `" \t\n\r+-0123456789abxz.,"` plus 800 `/dev/urandom`-derived inputs: no
  differences.

## Current status

`cargo test` in `translation/`: 12 tests, all passing, none ignored. Nothing in
`c_src/` was modified.
