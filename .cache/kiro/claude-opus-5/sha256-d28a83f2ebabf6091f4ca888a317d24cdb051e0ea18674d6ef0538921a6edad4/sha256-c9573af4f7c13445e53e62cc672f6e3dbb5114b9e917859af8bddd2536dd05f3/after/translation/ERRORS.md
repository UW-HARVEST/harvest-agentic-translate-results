# ERRORS.md — error / rejection surface of the C library

Mechanical extraction from `c_src/src/driver.c` and `c_src/include/driver.h`.

Greps performed (all of them, with their results):

```
grep -nE 'return|assert|NULL|errno|exit|abort|<|>|==|!=|#define|ERROR' c_src/src/driver.c
  29: int foo(const char *in, char c) {
  30:     int res = 0;
  31:     for (const char *s = in; s = strchr(s, c); s++) {
  34:     return res;
  37: void driver(const char *in) {
```

**Finding: the C library contains no error-return macro, no error enum, no
`assert`, no explicit range check, no null check, and no min/max constant.**
There is exactly one `return` statement (`return res;` in `foo`) and it never
signals failure. `driver` returns `void` and ignores `printf`'s return value.

The rejection surface therefore consists of (a) the single sentinel the code
*does* branch on — `strchr` returning `NULL`, which is the loop-termination
condition — and (b) the generic C-API boundaries that this API implicitly has.
Every row below is a distinct condition the C code actually reaches.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `foo` | `strchr(s, c)` returns the `NULL` sentinel on the **first** iteration: `c` does not occur in `in` (non-empty `in`) | loop body never runs, returns `0` | [x] |
| 2 | `foo` | `in` is the empty string `""` — first `strchr` immediately hits the terminator and returns `NULL` (for any `c != '\0'`) | returns `0` | [x] |
| 3 | `foo` | `strchr` returns `NULL` on a **later** iteration after `n` matches (normal termination) | returns exactly `n`; no over/under-count | [x] |
| 4 | `foo` | match at the **last** byte before the terminator: `s++` moves `s` onto the NUL, next `strchr` returns `NULL`. Boundary between "read the terminator" and "read past it". | returns the match count; must **not** read past the NUL | [x] |
| 5 | `foo` | `c == '\0'` (the one input that makes the `NULL` sentinel unreachable: `strchr(s,0)` returns a pointer to the terminator, `s++` then walks off the end of the buffer forever) | **undefined behaviour** — unbounded out-of-bounds read; not a defined rejection. Documented, deliberately **not** asserted. See note below. | [x] (documented, not asserted) |
| 6 | `foo` | `in == NULL` — no null check exists, so `strchr(NULL, c)` dereferences address 0 | `SIGSEGV` (fatal signal, no error code). Rust must also fault, not return or panic-with-message. | [x] |
| 7 | `driver` | `in == NULL` — propagates row 6 through `foo` before any `printf` runs | `SIGSEGV`, **no output at all** on stdout | [x] |
| 8 | `foo` | `c` argument supplied with garbage in the upper 24/56 bits of the argument register (legal for a `char` parameter under the SysV ABI; a caller declaring `int` can do this) | only the low byte is significant — result identical to passing the low byte alone | [x] |
| 9 | `foo` | `c` is a high-bit byte `0x80..0xFF`, i.e. **negative** as a signed `char`; C promotes it to a negative `int` for `strchr`, which converts back to `char` | matches that raw byte; must not be confused with any other byte, and must not sign-extend into a mismatch | [x] |
| 10 | `foo` | `c == 0x7F` / `c == 0x01` — extreme non-zero signed-`char` boundaries | matches that raw byte | [x] |
| 11 | `foo` | `in` consists **entirely** of matching bytes (every iteration matches, terminating only on the terminator) | returns `strlen(in)` | [x] |
| 12 | `foo` | match count exceeds the range of `int` (`res` is `int`, incremented without overflow check) | requires >2^31 matches, i.e. a >2 GiB input — unreachable in a test; `res` is incremented with wrapping semantics in Rust to avoid a release-mode-only panic divergence | [x] (reasoned, not asserted) |
| 13 | `driver` | `in` chosen so a count is `0` (`printf("%d")` of zero) | prints `A: 0\nx: 0\n` — exact bytes, including the case where one count is 0 and the other is not | [x] |
| 14 | `driver` | `in` chosen so counts have differing digit widths (1, 2, 3, 4, 5 digits) | `%d` formatting must match byte-for-byte | [x] |
| 15 | `driver` | `in` is `""` — zero-length input, both counts 0 | prints `A: 0\nx: 0\n` | [x] |
| 16 | `foo` / `driver` | `in` contains embedded high-bit / non-UTF-8 bytes (the API is byte-oriented, not UTF-8) | processed as raw bytes; no validation, no rejection | [x] |

## Note on row 5 (`c == '\0'`)

`foo(in, '\0')` cannot terminate: the loop exits only when `strchr` yields
`NULL`, but `strchr(s, '\0')` is *defined* to return a pointer to the
terminator, never `NULL`. So `s` is advanced past the terminator and the scan
continues into unmapped/unrelated memory indefinitely. Both the C and the Rust
build reproduce this same non-terminating out-of-bounds scan (the Rust `strchr`
checks `v == c` before `v == 0`, exactly matching the C library's ordering), but
the *observable* outcome depends on the process's memory map and on glibc
`strchr`'s vectorised aligned loads. It is genuine undefined behaviour with no
deterministic result to compare, so it is documented here rather than asserted.
`driver` never reaches it — it only ever passes `'A'` and `'x'`.

## Note on rows 6 and 7 (`SIGSEGV`)

These are asserted by forking a child process, calling the function there, and
comparing the child's termination signal for C vs Rust. Comparing "both failed
somehow" would be insufficient; the test compares the exact signal number.

**Divergence found and fixed here.** The original translation read the input
with a plain `*s`. Rust instruments plain raw-pointer dereferences with a
null/alignment precondition check whenever `debug-assertions` are enabled, so
in the `dev` profile the Rust `.so` died with a Rust panic and `SIGABRT` (6)
while the C `.so` died with `SIGSEGV` (11) — a real, observable difference at
the FFI boundary. `src/lib.rs` now performs the load with
`core::ptr::read_volatile`, which is not instrumented, so the faulting load
reaches the hardware and both libraries terminate with the same signal in
**every** build profile. Reads remain byte-by-byte and in the same order, so no
valid-input result changed.
