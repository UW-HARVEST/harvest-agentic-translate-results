# Differential verification log: `c_src/src/main.c` vs `translation/src/main.rs`

The C program is the ground truth. Both programs were built and then executed as
subprocesses with identical argument vectors; stdout, stderr and exit status were
compared byte for byte.

* C binary: `c_src/build/driver` (via `cmake .. && cmake --build .`)
* Rust binary: `translation/target/release/driver` (via `cargo build --release`)
* Test suite: `translation/tests/differential.rs` (22 tests, none ignored)

Nothing in `c_src/` was modified. The test harness will, if `c_src/build/driver`
is absent, configure an out-of-tree CMake build into `translation/target/c_build`
so that `c_src/` is never written to.

---

## Mismatches found

### 1. Broken stdout pipe: Rust survived where C is killed by SIGPIPE — FIXED

**Severity: real behavioral divergence on every code path in the program.**

| input | C | Rust (before fix) |
|---|---|---|
| `driver hello` with stdout = closed pipe | killed by signal 13 (SIGPIPE), no exit code | exit code **0**, no signal |
| `driver hello abc` with stdout = closed pipe | killed by signal 13 | exit code **1**, no signal |
| `driver` (usage error) with stdout = closed pipe | killed by signal 13 | exit code **1**, no signal |

Reproduction (`driver hello | true` in essence):

```
$ python3 -c '
import os, signal
r,w=os.pipe(); os.close(r)
pid=os.fork()
if pid==0:
    signal.signal(signal.SIGPIPE, signal.SIG_DFL)   # what a shell does
    os.dup2(w,1); os.execv("./driver",["./driver","hello"])
print(hex(os.waitpid(pid,0)[1]))'
```

**Cause.** The Rust standard library sets `SIGPIPE` to `SIG_IGN` during runtime
initialization, *before* `main` runs. A C program never does this — it inherits
the default disposition from its parent, and a shell always hands children
`SIG_DFL`. So in the C program the first `write(2)` to a pipe with no reader
raises SIGPIPE and kills the process, while in Rust that same write merely
returned `Err(EPIPE)`. The translation additionally discards every write result
(`let _ = out.write_all(...)`), which faithfully mirrors C never checking
`printf`'s return value, but combined with the ignored SIGPIPE it meant the Rust
program quietly ran to completion and returned its normal 0/1 exit code.

Note this affects *all* inputs, not just the success path, because every branch
of this program writes to stdout — including the three error paths.

**First attempt at a fix**, which was *wrong* — see mismatch 2: unconditionally
force `SIG_DFL` as the first statement of `main`.

**Regression test.** `broken_stdout_pipe_kills_both_with_sigpipe` and
`broken_stdout_pipe_with_large_payload` in `tests/differential.rs`. These were
confirmed non-vacuous: commenting out the restore call makes both fail with
`C=(None, Some(13)) RUST=(Some(0), None)`.

The large-payload variant matters separately because C's stdout is fully
buffered when it is a pipe: for a short string the failing write happens only in
the exit-time flush, whereas a 60 000-byte string forces a `write(2)` from
inside `printf` itself. Both routes had to be checked.

---

### 2. Over-correction: forcing SIG_DFL broke SIGPIPE-ignoring parents — FIXED

**Found by asking "what if the parent already ignored SIGPIPE?" after fixing
mismatch 1.** The first fix traded one divergence for another.

A C program does not *set* its SIGPIPE disposition, it *inherits* it, and an
ignored disposition survives `execve`. So when the parent has already set
SIGPIPE to `SIG_IGN`, the C program inherits `SIG_IGN` and **survives** a broken
pipe, exiting 0 or 1 normally. Forcing `SIG_DFL` made the Rust die instead:

| input, parent has SIGPIPE=SIG_IGN | C | Rust (after fix 1) |
|---|---|---|
| `driver hello` | exit 0 | **killed by signal 13** |
| `driver` (usage) | exit 1 | **killed by signal 13** |
| `driver hello abc` | exit 1 | **killed by signal 13** |
| 60 000-byte string | exit 0 | **killed by signal 13** |

(This case accidentally "passed" before fix 1, because Rust's own `SIG_IGN`
happened to coincide with the inherited `SIG_IGN`.)

**Cause.** `std::rt::init` overwrites the inherited disposition before `main`
runs, so by the time user code executes, the original value is already lost and
cannot be recovered by querying the current state.

**Fix.** Capture the inherited disposition from an ELF `.init_array`
constructor, which glibc invokes from `__libc_start_main` *before* it calls
`main` and therefore before `std::rt::init`, then restore exactly that value at
the top of `main`:

```rust
static INHERITED_SIGPIPE: AtomicUsize = AtomicUsize::new(SIG_DFL);

#[used]
#[link_section = ".init_array"]
static CAPTURE_SIGPIPE_CTOR: extern "C" fn() = capture_inherited_sigpipe;

extern "C" fn capture_inherited_sigpipe() {
    unsafe {
        let previous = signal(SIGPIPE, SIG_DFL);   // returns the old handler
        if previous != SIG_ERR {
            signal(SIGPIPE, previous);             // put it straight back
            INHERITED_SIGPIPE.store(previous, Ordering::Relaxed);
        }
    }
}
```

This also handles a parent that installed a custom SIGPIPE *handler*: the
function pointer is captured and reinstated, so the handler runs and the write
returns `EPIPE` just as it would for the C.

Both dispositions now agree for every input tried:
`SIG_DFL` parent → both killed by signal 13; `SIG_IGN` parent → both exit 0/1.

**Regression test.** `broken_stdout_pipe_with_inherited_sig_ign_survives_in_both`,
which uses `CommandExt::pre_exec` to set `SIG_IGN` in the child before exec.

Note that in practice most launchers hand children `SIG_DFL` — every shell does,
and both `subprocess.Popen` (`restore_signals=True` by default) and Rust's
`std::process::Command` explicitly reset SIGPIPE to `SIG_DFL` in the child. The
`SIG_IGN` case is reachable via a bare `fork` + `execv`, which is how it was
found.

---

## Test-harness defect found and fixed (not a translation bug)

`long_input_windows` initially built a 4096-byte argument from
`(0u8..=255).cycle()`. A NUL byte cannot appear inside an argv string, so
`Command::spawn` failed with `nul byte found in provided data` before either
program ran. Changed to `(1u8..=255).cycle()`. This was a bug in the test, not
in the translation — recorded here because a spawn failure looks like a test
failure and would otherwise mislead the next reader.

---

## C quirks deliberately preserved (verified, not "fixed")

These all look like bugs. The C is authoritative, so the Rust reproduces them.

1. **`start > len` and `stop > len` compare `int` against `size_t`.** The usual
   arithmetic conversions promote the `int` to unsigned 64-bit, so **every
   negative `start`/`stop` becomes an enormous value** and is reported as "off
   the end of the string" rather than as a negative index. Rust reproduces this
   with `(start as i64 as u64) > len`.
   - `driver hello -1` → `Error: start is off the end of the string!` / exit 1
   - `driver hello 0 -1` → `Error: stop is off the end of the string!` / exit 1

2. **The third-argument integer check is dead code.** Line 62 calls
   `strtol(argv[3], NULL, 10)` with a *NULL* endptr, so `end` is never updated;
   line 63 then tests the **stale `end` left over from parsing `argv[2]`**.
   Because argv strings are laid out contiguously, `end` can point at most to
   `argv[2]`'s NUL terminator, i.e. `argv[3] - 1`, so `end == argv[3]` is never
   true. Verified empirically with a throwaway probe (in `$TMPDIR`, not in
   `c_src/`) that printed `end`, `argv[3]` and their difference: the difference
   was −1, −2, −3 or −4 across every input tried, never 0.

   Consequence: **"Third argument must be an integer!" can never be printed.** A
   non-numeric third argument silently yields `stop = 0`, which then trips the
   `stop <= start` check instead:
   - `driver hello 0 abc` → `Error: stop must come after start!` / exit 1

   The Rust models the pointer as an `(argv index, byte offset)` pair so the
   comparison is false for the same structural reason rather than by hard-coding
   `false`. `third_argument_integer_check_is_dead_code` also asserts that neither
   binary ever emits the string "Third argument".

3. **The two "must be an integer!" messages have no trailing newline**, unlike
   every other message. `driver hello abc` emits exactly
   `Second argument must be an integer!` with no `\n`.

4. **All output, including errors, goes to stdout; stderr is always empty.**
   Asserted by `stderr_is_always_empty`.

5. **`long` → `int` truncation on assignment to `start`/`stop`**, and strtol's
   saturation to `LONG_MAX`/`LONG_MIN` on overflow (with `ERANGE`), which then
   truncates. This produces some genuinely surprising accepted inputs:
   - `driver hello 4294967296` → `4294967296 as i32 == 0` → prints `hello`
   - `driver hello 4294967301` → `5 == len` → prints an empty line
   - `driver hello 9223372036854775807` → `LONG_MAX as i32 == -1` → "off the end"
   - `driver hello 99999999999999999999999` → saturates to `LONG_MAX` → `-1` → "off the end"

6. **strtol's partial-conversion grammar is a success, not an error.** Leading
   C-locale whitespace (`' '`, `\t`, `\n`, `\v`, `\f`, `\r`) and one optional
   sign are consumed, digits are read until the first non-digit, and the rest is
   ignored. So `" 2"`, `"+2"`, `"2abc"`, `"0x3"` (base 10 → parses `0`, stops at
   `x`) and `"007"` are all *valid*. Only a complete absence of digits (`""`,
   `"abc"`, `"-"`, `"+"`, `"   "`, `"\xff"`) sets `end == argv[2]`.

7. **`start == len` is allowed** (the check is `>`, not `>=`), giving a precision
   of 0 and printing just a newline: `driver hello 5` → `"\n"`, exit 0.

8. **Bounds are checked before ordering.** For `driver hello 3 -1` both
   `stop > len` (after unsigned conversion) and `stop <= start` hold; the C
   prints the *stop-bounds* message. Asserted explicitly.

9. **The program is byte-oriented, not character-oriented.** `strlen` and the
   `%.*s` precision count bytes, so a multi-byte UTF-8 sequence can be sliced in
   half and argv need not be valid UTF-8. The Rust therefore uses
   `std::env::args_os()` with `OsStrExt::as_bytes()` and writes raw bytes; using
   `std::env::args()` would have panicked on non-UTF-8 argv.

10. **`printf`'s return value is never checked**, so write failures do not change
    the exit status. Verified against `/dev/full` (every write fails with
    `ENOSPC`) and against a closed fd 1 (`EBADF`): both programs still exit 0 or
    1 as usual. The Rust's `let _ = ...` on every write is correct here — it is
    only the *signal* disposition that had to be fixed (see mismatch 1).

---

## Inputs enumerated from the C source

Every branch in `main` and the input class that reaches it:

| C line | branch | reaching input | result |
|---|---|---|---|
| 36 | `argc == 1` | `driver` | usage, exit 1 |
| 36 | `argc > 4` | `driver a b c d` | usage, exit 1 |
| 47 | `argc >= 3` false | `driver hello` | `start = 0` |
| 48–49 | `end == argv[2]` | `driver hello abc` | "Second argument…", exit 1 |
| 53 | `start > len` | `driver hello 6`, `driver hello -1` | "start is off the end…", exit 1 |
| 58 | `else start = 0` | `driver hello` | success |
| 61 | `argc == 4` | `driver hello 1 3` | parse stop |
| 63 | `end == argv[3]` | **unreachable — dead code** | never |
| 68 | `stop > len` | `driver hello 0 6`, `driver hello 0 -1` | "stop is off the end…", exit 1 |
| 73 | `stop <= start` | `driver hello 3 2`, `driver hello 0 abc` | "stop must come after start!", exit 1 |
| 78 | `else stop = len` | `driver hello 1` | success |
| 82 | final `printf` | `driver hello 1 3` → `el\n` | exit 0 |

Additional input classes covered beyond the branch table: the empty string as
`argv[1]`; `start == len` (empty output); an exhaustive start × stop grid over
`-3..=12` for five different strings; non-UTF-8 and multi-byte-splitting argv;
120 000-byte strings at the exact length boundary; every-byte-except-NUL
payloads; a matrix of numeric spellings in both numeric positions; `argc == 0`
via a raw `execv` with an empty argv (unreachable from a shell — both programs
print the usage error and exit 1); stdin data being ignored; and locale
environment variables (`LC_ALL`, `LC_NUMERIC`, `LANG`) not perturbing `strtol`,
since the C never calls `setlocale` and so stays in the "C" locale.

---

## Final state

* Both programs build with no errors and no warnings.
* `cargo test` in `translation/`: **22 passed, 0 failed, 0 ignored.**
* Beyond the committed suite, ad-hoc fuzzing compared roughly **24 500** further
  argument vectors with zero mismatches in stdout, stderr or exit status:
  random and systematic sweeps over strings × numeric spellings × arities 1–7
  (6 530 + 8 720 cases), a full start × stop grid (2 280), a `strtol` acceptance
  grammar sweep (423 cases: `"+ 5"`, `"--5"`, `"-+5"`, 10 000 leading zeros,
  5 000-digit numbers, `"0X5"`, `"5.9"`, `"inf"`, `"nan"`, Arabic-Indic and
  full-width digits, every C-locale whitespace byte), every numeric boundary in
  both positions (5 328 cases: `len`, `len±1`, `INT_MAX`, `INT_MIN`, `UINT_MAX`,
  `LONG_MAX`, `LONG_MIN`, 2^31, 2^32, 2^63, 2^64, each ±1, and ±10^30), and a
  re-comparison of two from-scratch rebuilds (1 056).
* The Rust was checked for panic paths the C does not have: the final
  `&argv[1][offset..]` is safe because `start > len` is rejected first (and
  `start == len` is deliberately allowed, yielding an empty slice), and the
  `%.*s` precision can never be negative because `stop <= start` is rejected on
  the three-argument path while the shorter paths give `stop = len >= start`.
  `print_precision_string` still handles a negative precision the way C does
  (treated as omitted), defensively.
* Two mismatches were found, both in the SIGPIPE disposition, and both fixed in
  the Rust. The C was never modified.

Both mismatches were in process-level behaviour rather than in the string
logic — the argument parsing, the int/size_t comparison quirks, the truncation
and the output formatting were all already correct. That is worth noting because
a suite that only compared stdout on well-formed input would have reported a
clean pass: the SIGPIPE bugs are visible only in the exit status, and only when
stdout is unwritable.
