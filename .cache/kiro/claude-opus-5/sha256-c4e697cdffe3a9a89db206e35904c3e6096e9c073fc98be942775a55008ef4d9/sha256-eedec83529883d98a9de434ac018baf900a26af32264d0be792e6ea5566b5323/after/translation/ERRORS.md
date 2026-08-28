# Differential testing log: `c_src/src/container_of.c` vs `translation/`

The C program is the ground truth. Everything below records a place where the
Rust translation did *not* match it, and what the cause was. Fixes were made in
`translation/src/main.rs` only; nothing in `c_src/` was changed.

## What the C program actually does

```c
int main(int argc, char** argv) {
    int a = atoi(argv[1]);
    int b = atoi(argv[2]);
    struct test t;
    memset(&t, 0, sizeof(t));
    t.a = a; t.b = b;
    printf("%d\n", find_container_of_a(&t.a)->a + find_container_of_b(&t.b)->b);
}
```

There is no stdin, no loop and no explicit `if`. The input classes are therefore
entirely in the shape of `argv` and in what `atoi` does with each string:

| Class | Input | Behaviour |
|---|---|---|
| both arguments present | `1 2` | prints `a + b` and `"\n"`, exits 0 |
| extra arguments | `1 2 3 …` | ignored, same as `1 2` |
| empty argument string | `"" ""` | `atoi("")` is 0 |
| unparseable | `abc`, `-`, `+`, `.5` | 0 |
| trailing garbage | `12abc` | parse stops at the first non-digit |
| leading whitespace / sign | `"\t -42"` | `isspace` skipped, sign honoured |
| no base prefixes | `0x10`, `010` | base is 10: `0`, and `10` (not 8) |
| beyond `int` | `4294967296` | `(int)` truncation to the low 32 bits |
| beyond `long` | `9…9` (400 digits) | `strtol` saturates, then truncates |
| addition overflow | `2147483647 1` | wraps (what gcc emits here) |
| **fewer than 2 arguments** | *(none)*, `5` | `atoi(NULL)` → **killed by SIGSEGV**, no output |

`find_container_of_a` / `find_container_of_b` subtract `offsetof` and the result
is immediately re-dereferenced through the same member, so both round-trips are
identity operations. They cannot fail or alter the printed value; only the
`atoi` results and the addition can.

## Mismatch 1 — wrong termination signal when arguments are missing

**Severity: this was the only genuine behavioural divergence found.**

*Inputs:* `driver` (no arguments) and `driver 5` (one argument).

| | C | Rust (before) | Rust (after) |
|---|---|---|---|
| stdout | empty | empty | empty |
| stderr | empty | empty | empty |
| status | signal 11 (SIGSEGV), shell reports 139 | signal 6 (SIGABRT), shell reports 134 | signal 11, 139 |

*Cause.* The C code never checks `argc`, so `argv[1]` (or `argv[2]`) is the NULL
terminator of the argument vector. glibc's `atoi` is `(int)strtol(nptr, NULL, 10)`,
which dereferences that NULL and the process is killed by SIGSEGV.

The translation modelled this with `std::process::abort()`. Both crash and both
print nothing, so a stdout-only comparison passes — but the wait status is
different, which is exactly the failure mode the task warns about.

*Fix, attempt 1 (insufficient).* Replaced `abort()` with `raise(SIGSEGV)`. The
process was **still** reported as `Aborted`, because the Rust standard library
installs its own SIGSEGV handler on an alternate stack to detect stack overflow;
for a fault it does not recognise, that handler ends in `abort()`, converting
signal 11 into signal 6.

*Fix, attempt 2 (correct).* Restore the default disposition before raising, so
the kernel's default action for SIGSEGV terminates the process:

```rust
signal(SIGSEGV, SIG_DFL);
raise(SIGSEGV);
```

Now both programs are reported as terminated by signal 11 with empty stdout and
stderr. Covered by `no_arguments_dies_the_same_way`,
`one_argument_dies_the_same_way` and `no_output_is_produced_on_the_crashing_paths`;
the last of these also asserts the outcome is not a clean `exit(0)`, so a future
change that quietly turns the crash into a normal exit cannot pass.

A deliberate note on why this was not "fixed" into an error message: adding
`argc` validation, a usage string on stderr or `exit(1)` would all have been
divergences from the C, which prints nothing and dies on a signal.

## Verified-equivalent areas (no mismatch, checked explicitly)

* **`atoi` saturation then truncation.** `9223372036854775808` and above give
  `-1` (LONG_MAX truncated); `-9223372036854775809` and below give `0`
  (LONG_MIN truncated). The Rust `atoi` accumulates in `i64`, latches a
  `saturated` flag, keeps consuming digits, then clamps and casts — matching
  for inputs up to 400 digits.
* **`isspace` set.** The Rust version skips exactly `' '`, `\t`, `\n`, `\v`,
  `\f`, `\r`, matching the C locale's `isspace`.
* **Signed addition overflow.** C signed overflow is undefined, but the binary
  as built (no optimisation flags in `CMakeLists.txt`) wraps; `wrapping_add`
  reproduces it. Checked at both `int` boundaries.
* **Non-UTF-8 arguments.** `argv` is bytes in C. The translation uses
  `args_os` and operates on raw bytes, so `a\xffb` behaves identically instead
  of panicking or lossily decoding.
* **Output formatting.** `printf("%d\n", …)` — a bare decimal and a single
  trailing newline, no padding or precision. Compared byte for byte.
* **`container_of` round-trip.** Exercised on every passing input; identity in
  both, as expected.

## How this was checked

* `translation/tests/differential.rs` — 21 tests, each running both binaries as
  subprocesses and asserting stdout, stderr and termination (exit code *or*
  signal, kept distinct) all agree. The C binary is built via its own CMake
  setup if it is not already present. No test is `#[ignore]`d.
* An additional out-of-band sweep of 3000 randomly generated argument pairs
  (mixed in-range ints, out-of-range 70-bit values, and garbage fragments)
  produced zero differences.
* `cargo test` passes in both debug and release profiles, with no warnings.
