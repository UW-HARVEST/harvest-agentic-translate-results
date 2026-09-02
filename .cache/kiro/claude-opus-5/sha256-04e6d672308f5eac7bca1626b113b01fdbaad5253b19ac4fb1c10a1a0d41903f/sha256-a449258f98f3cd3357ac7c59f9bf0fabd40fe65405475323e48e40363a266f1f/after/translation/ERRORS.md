# Differential verification of the Rust translation of `c_src/src/luggage.c`

Reference build: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
→ `c_src/build/driver`. `CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so the
reference is compiled at **`-O0`** (gcc 11.5.0, glibc 2.34).

Rust build: `cd translation && cargo build --release` → `translation/target/release/driver`.

Both are driven as subprocesses by `translation/tests/differential.rs`
(36 tests, ~1900 input/argv combinations, no `#[ignore]`), which compares
stdout, stderr and the exit status byte for byte. In addition ~65 000 randomised
inputs were run through an out-of-tree fuzzer during development with zero
mismatches.

---

## Mismatches found and fixed

### 1. Uninitialised stack buffers observable on the first loop iteration

**Symptom** — for stdin `abc\n` and argv `- - - -`:

```
C    : "0000000000 \x03    abc\n"
Rust : "0000000000     abc\n"          <-- luggage id printed as empty
```

**Cause** — `main`'s per-iteration buffers are declared without an initialiser:

```c
unsigned int time_stamp;
char luggage_id[LUGGAGE_ID_LENGTH + 1];
...
comments[0] = 0;                      /* only `comments` is initialised */
```

A `scanf` *matching* failure (as opposed to an input failure) returns 0, not
`EOF`, so the C code does **not** `break`; it goes on to `strcpy` the buffer that
the failed conversion never touched. On the second and later iterations that
buffer holds the previous record's value, which the translation already modelled
by hoisting the variables out of the loop. On the **first** iteration it holds
whatever main's stack frame contained, which the C standard leaves undefined.

The reference `-O0` build reproducibly leaves `time_stamp == 0`,
`luggage_id == "\x03"` and `flight_id` / `departure` / `arrival` empty — leftovers
from libc start-up code that ran in the same stack region. Verified stable over
200 runs and unaffected by argv length, an empty environment (`env -i`) and a
3 KB environment variable.

**Fix** — `src/main.rs` seeds `luggage_id` from `UNINIT_LUGGAGE_ID` (`b"\x03"`)
and leaves the other buffers empty, so the reference build's observable output is
reproduced.

**Caveat (this is genuinely undefined behaviour).** The byte is an artefact of
the reference build, not of the C source. Recompiling the same file at other
optimisation levels yields different garbage:

| build | first byte(s) of `luggage_id` |
|-------|-------------------------------|
| `-O0` (reference) | `03` |
| `-O1` | `7c f3 b5 …` |
| `-O2` | `cc 5b a3 …` |
| `-O3` | `fc cc 60 …` |
| `-Os` | `61 …` |

Affected input classes: any stdin whose **first** record makes
`scanf("%8[A-Z0-9] …")` fail to match — e.g. stdin starting with a lowercase
word, punctuation, or a bare `+`/`-` sign. Tested by
`matching_failure_on_first_iteration_uses_uninitialised_buffers`.

### 2. `strcpy` truncates the comment at an embedded NUL, `%80[^\n]` does not

**Symptom** — for stdin `5 LUG1 FL1 JFK LAX ab\0cd\n`:

```
C    : "0000000005 LUG1 FL1 JFK LAX  ab\n"
Rust : "0000000005 LUG1 FL1 JFK LAX  ab\x00cd\n"
```

**Cause** — the scanset `[^\n]` matches every byte except `\n`, including `\0`,
so `scanf` stores the NUL in the local buffer and counts it against the field
width of 80. The subsequent `strcpy(new_directive->comments, comments)` then
copies only up to the first NUL, and `printf("%s")` / `strcmp` see the truncated
string. The translation was storing the full byte run.

**Fix** — a `strcpy_bytes()` helper truncates at the first NUL, applied to all
five fields when the directive is built. Truncation happens at *store* time, not
at *scan* time, because the bytes after the NUL still consume field width and so
still determine where the next conversion starts reading. Tested by
`nul_bytes_in_the_comment_truncate_the_stored_string`, including a case where the
NUL is followed by 100 more bytes.

### 3. Exit status 0 instead of death by `SIGPIPE`

**Symptom** — with a large input and a reader that goes away
(`driver - - - - < big | head -c 20`):

```
C    : killed by SIGPIPE, shell reports status 141
Rust : status 0
```

**Cause** — the Rust runtime sets `SIGPIPE` to `SIG_IGN` before `main` runs, so
writes to a broken pipe return `EPIPE`, which the program ignored before calling
`exit(0)`. The C program inherits the default disposition and is killed.

**Fix** — `restore_default_sigpipe()` resets `SIGPIPE` to `SIG_DFL` at the top of
`main` via a direct `signal(2)` call. Tested by `dying_on_a_closed_stdout_pipe`.
Note that a *closed* stdout (fd 1 not open at all) yields `EBADF` rather than
`SIGPIPE`, and both programs exit 0 there — also checked.

---

## Behaviours confirmed identical (no change needed)

These were the risky spots; each is covered by a named test.

* **`%d` into an `unsigned int *`.** glibc parses with `strtol` semantics and
  stores the result as an `int`, so out-of-range input clamps to
  `LONG_MIN`/`LONG_MAX` and is then truncated: `99999999999999999999` →
  `4294967295`, `4294967296` → `0`, `-5` → `4294967291`. Decimal only, so `010`
  is ten. A 10 000-digit number is handled without panicking (the accumulator
  latches an overflow flag long before it could wrap).
* **`EOF` vs. matching failure.** Only `== EOF` breaks the read loop. All four
  `scanf` calls have their own EOF exit, and a partially assigned call still
  returns ≥ 1 and therefore does *not* break — e.g. `55 LUG1` (flight id hits
  EOF) falls through to the next `scanf`, which is the one that breaks.
* **The dropped last record.** A final line without a trailing newline *and*
  without a comment makes `%80[^\n]` an input failure, so the otherwise complete
  record is discarded. Adding a comment makes it a successful match and the
  record survives.
* **The double space before comments.** `%3[A-Z]` for the arrival is not followed
  by a whitespace directive, so `%80[^\n]` keeps the separating blank; the format
  string then adds its own, producing `… LAX  comment`. With no comment at all
  the line ends in a single trailing space.
* **Field widths and spill-over.** Conversions stop at 8/6/3/3/80 characters and
  the leftovers are re-parsed by the *following* conversion, so
  `5 123456789 FL1 JFK LAX c` yields luggage id `12345678`, flight id `9`,
  departure `FL`, arrival `1`, comment ` JFK LAX c`. A comment longer than 80
  bytes leaves the tail in the stream, where it becomes the next record's
  timestamp.
* **`scanf` reads across newlines.** One field per line, blank lines between
  fields, and several records on one line all parse the same way.
* **Whitespace class.** `\t`, `\n`, `\v`, `\f`, `\r` and space are all skipped by
  the whitespace directives and by `%d`.
* **`supersedes()` stops at the first later directive with a matching luggage id**
  and reports "superseded" only if the departure also matches — even when a still
  later directive would have matched. Preserved verbatim.
* **Insertion order.** `addRoutingDirectiveToList` inserts before the first
  strictly greater timestamp, so equal timestamps keep input order, and the
  supersede scan walks the *sorted* list rather than the input order.
* **`matches()` wildcard.** Only a leading `-` is a wildcard (`-abc` is one too);
  an empty argument is not, and it matches a field that was never assigned.
* **argv/stderr.** `argc != 5` writes
  `Command line error: 4 arguments expected\n` to stderr and exits 1 without
  reading stdin. Checked for argc 1–4, 6 and 7. Non-UTF-8 argv bytes are compared
  as raw bytes.
* **`%010u` formatting**, including values above `INT_MAX` and exactly zero.

---

## Known divergence that is not practically reachable

`addRoutingDirectiveToList` and `supersedes` are recursive with one frame per
list element (48 bytes per `supersedes` frame at `-O0`), so with the default
8 MiB stack the C program would overflow at roughly 175 000 directives, while the
Rust translation is iterative and would not.

That threshold cannot be reached in practice: reaching a recursion depth of *n*
also costs Θ(n²) work in `printMatchingDirectives`, so at n ≈ 175 000 the C
program needs hours of CPU time before it could crash. Measured wall-clock:
n = 20 000 completes in seconds and matches exactly; at n = 100 000 the C program
exceeds a 60 s budget while the Rust program finishes, which is a speed
difference, not a behavioural one. The test suite therefore exercises volume at
n = 400 (recursive insert at head, tail and middle, plus the O(n²) supersede
scan) and does not attempt to reproduce a stack overflow.

---

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # C reference
cd ../../translation && cargo build --release                           # Rust
cargo test --release                                                    # differential suite
```

`cargo test` also works in the debug profile (arithmetic-overflow checks on) and
builds the C reference automatically if `c_src/build/driver` is missing. Nothing
under `c_src/` is modified by any of this apart from the `build/` directory that
CMake creates.
