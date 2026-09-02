# Mismatches found between the C reference and the Rust translation

Reference: `c_src/src/main.c`, built by `c_src/CMakeLists.txt`.
`CMAKE_BUILD_TYPE` is empty, so the compile line is a bare `gcc src/main.c` —
**no optimisation flags**. That matters: every finding below comes from the
`-O0` stack layout.

Toolchain used for the measurements: gcc 11.5.0 (Red Hat 11.5.0-5), x86-64,
Linux with `kernel.randomize_va_space = 2`.

Verification method: both programs are built, run as subprocesses on identical
stdin, and stdout / stderr / exit status are compared byte for byte
(`translation/tests/differential.rs`).

## Summary

| # | Input class | C | Rust (before) | Status |
|---|---|---|---|---|
| 1 | `bad()` index 16–19 | SIGSEGV, no output | exit 0, full output | fixed |
| 2 | `bad()` index 26–27 | SIGSEGV, no output | exit 0, full output | fixed |
| 3 | `bad()` index past the stack mapping | SIGSEGV, no output | exit 0, full output | fixed |
| 4 | any crashing index, stdout on a pipe | output lost, unflushed | output emitted | fixed |
| 5 | fault threshold placement | env-dependent | biased ~8 indices high | fixed |
| 6 | index in the randomised band | not reproducible even C vs C | — | inherent, documented |
| 7 | index above ~2e8 | SIGSEGV *or* SIGBUS, at random | always SIGSEGV | inherent, documented |

---

## The root cause: `bad()` has no upper bound check

```c
int buffer[10] = { 0 };
if (data >= 0)          /* no `data < 10` — this is the CWE-129 defect */
{
    buffer[data] = 1;
```

`goodB2G` has the same source shape but checks `data >= 0 && data < (10)`, and
`goodG2B` hard-codes `data = 7`, so **only `bad()` can store out of bounds**.
`data` comes from `atoi` on a 13-character `fgets` buffer, so any `int` value is
reachable, including large ones and — via truncation of a 13-digit number —
essentially arbitrary ones.

The original translation absorbed every out-of-bounds store into a 4096-word
padding array, on the reasoning that only `buffer[0..10]` is ever printed so the
stray store is unobservable. That reasoning is wrong at `-O0`: the store lands
on **live** parts of the frame, including return addresses.

### The actual frame, from `objdump -d c_src/build/driver`

```
bad:   push %rbp ; mov %rsp,%rbp ; sub $0x40,%rsp
       movl $0x1,-0x40(%rbp,%rax,4)      <- buffer[data] = 1
main:  push %rbp ; mov %rsp,%rbp ; sub $0x10,%rsp   ; calls bad directly
```

`buffer` is at `%rbp-0x40`, so `buffer[n]` writes 4 bytes at `%rbp - 64 + 4n`.
`main` calls `bad` directly, which puts `bad`'s `%rbp` exactly 32 bytes below
`main`'s (8 for the return address, 8 for the pushed `%rbp`, 16 for `main`'s
`sub $0x10`). That fixes the whole neighbourhood:

| n | offset from `bad`'s `%rbp` | what lives there | effect |
|---|---|---|---|
| 0–9 | −64 … −28 | `buffer[0..10]` | the intended store |
| 10–13 | −24 … −12 | `char inputBuffer[14]` | none, already parsed |
| 14 | −8 | `int i` | none — `i = 0` executes *after* the store |
| 15 | −4 | `int data` | none — never read again |
| **16–17** | 0 … 7 | `bad`'s saved `%rbp` | **crash in `leave`** |
| **18–19** | 8 … 15 | return address into `main` | **crash in `ret`** |
| 20–21 | 16 … 23 | `main`'s `argv` | none, dead |
| 22 | 24 … 27 | padding in `main`'s frame | none |
| 23 | 28 … 31 | `main`'s `argc` | none, dead |
| 24–25 | 32 … 39 | `main`'s saved `%rbp` | none — libc's caller does not rely on it |
| **26–27** | 40 … 47 | `main`'s return address | **crash in `main`'s `ret`** |
| 28 … | 48 … | libc frames, then the argv/env block | absorbed until the stack mapping ends |

Reading the table off predicts crashes at exactly `{16,17,18,19,26,27}`. A sweep
of the real binary over indices 10–400 crashes on exactly that set — the model
was derived from the disassembly and then confirmed, not fitted.

### Mismatch 1 — indices 16–19 (`bad`'s saved `%rbp` and return address)

* C: killed by SIGSEGV (shell reports 139), **stdout completely empty**.
* Rust before: exit 0, full 126-byte output.

16 and 17 corrupt the saved `%rbp`, so `leave` (`mov %rbp,%rsp ; pop %rbp`)
loads a garbage `%rsp` and faults. 18 and 19 corrupt the return address, so
`ret` jumps to `0x1`.

### Mismatch 2 — indices 26–27 (`main`'s return address)

* C: SIGSEGV, stdout empty. `bad()` and `main` run to completion — including
  printing `Finished bad()` — and then `main`'s `ret` jumps to `0x1`.
* Rust before: exit 0, full output.

Indices 24–25 corrupt `main`'s *saved `%rbp`* and do **not** crash: `main`'s
`leave` loads garbage into `%rbp`, but `ret` still finds the correct return
address and `__libc_start_call_main` never dereferences the bad `%rbp`. That
asymmetry is why the crash set is not contiguous, and it is covered by
`oob_indices_adjacent_to_crash_slots_are_absorbed`.

### Mismatch 3 — the store leaves the stack mapping

Above index 27 the store lands in the argv/env block and is absorbed, until it
passes the end of the `[stack]` mapping, at which point the **store itself**
faults, before the print loop runs.

* C: SIGSEGV, stdout empty.
* Rust before: exit 0, full output.

### Mismatch 4 — stdout buffering

All three crash classes produce *no output at all* when stdout is a pipe. glibc
picks full buffering for a non-terminal stream, so the ~126 bytes `printf`
produced are still in the `FILE` buffer when the process dies, and `exit()`
never runs to flush them.

The original translation wrote through `BufWriter` but flushed at the end of
`main`, and Rust's own `Stdout` is a `LineWriter` that flushes on every newline —
either way the output would have escaped before a crash. `CStdout` in
`src/main.rs` now reproduces glibc's choice: line buffered only when fd 1 is a
character device, fully buffered otherwise, and the buffer is simply discarded
when the emulated fault fires.

This is also why the crash *point* is modelled rather than collapsed: on a
terminal the C would emit the ten values before dying at index 18, but nothing
at all at a far-out-of-range index.

### Mismatch 5 — where the fault threshold sits

Found as an intermittent failure of `oob_far_out_of_stack_always_faults` at
n = 3500, not by inspection.

The first fix computed the available headroom as `stack_end - &some_local`,
using a Rust local's address as a stand-in for `bad`'s `%rbp`. Rust's frames sit
deeper than the C's (extra runtime frames below `main`), so that systematically
*over*-estimated the headroom and put the Rust threshold above the C's.

The headroom is now anchored on the process's initial stack pointer instead,
which is a property of the kernel's exec layout rather than of either language's
frames:

```
headroom = (stack_end - startstack) + 304
```

`stack_end` comes from `/proc/self/maps`, `startstack` from field 28 of
`/proc/self/stat`. The constant 304 is `startstack - bad_rbp` for the C binary:
the fixed `__libc_start_main` / `__libc_start_call_main` / `main` / `bad` frames.
It was measured by running the reference under `setarch -R` (ASLR off, so the
threshold is exact) and bisecting — index 1259 survives, 1260 faults, which puts
`stack_end - bad_rbp` in [4976, 4980); with `stack_end - startstack` = 4672 and
`bad_rbp` necessarily 16-byte aligned, the distance is exactly 304.

With ASLR off the two binaries' thresholds now agree to within 8 indices
(32 bytes). The residual is the different length of `argv[0]` — the two
executables live at different paths, so their argv blocks differ in size and the
C process genuinely has a slightly different stack top. No translation can know
the C's own path length, and 32 bytes is negligible against the 8 KiB of stack
randomisation described next.

---

## Inherent nondeterminism: what cannot be matched, and why

These are **not** translation defects. The C is not reproducible against itself
here, so no Rust program could be reproducible against it.

### Mismatch 6 — the randomised band, roughly 2000 indices wide

Whether a store near the top of the stack is still inside the mapping depends on
a per-exec random offset the kernel subtracts from the initial stack pointer
(`arch_align_stack`). Measured spread over 300 execs: **8160 bytes**, i.e. up to
8 KiB in 16-byte steps. Divided by the 4-byte stride that is ~2040 indices in
which the *same* C binary exits 0 on one run and dies on the next:

```
n=2000  C 10/30 crashes   n=2800  C 23/30   n=3300  C 30/30
```

After the `startstack` calibration the Rust tracks the same curve (8/30, 21/30,
30/30 at those points), but per-run agreement is impossible by construction.

The band's position also scales with the environment size, because the argv/env
block is what separates `bad`'s frame from the top of the stack. Measured with
ASLR off: 464 bytes under `env -i`, 4544 under an ordinary shell, 14560 with
10 KB of extra environment. So there is no fixed "safe" index — the tests derive
the bounds at run time from `/proc/self/{maps,stat}` (`always_absorbed_ceiling`,
`always_faults_floor`) instead of hard-coding indices that happen to work in one
shell. Two early test failures were exactly this mistake on my part, not
translation bugs.

Strict comparison is therefore applied to indices 0–48, which are deterministic
in *any* environment (index 48 stores 132 bytes above `bad`'s `%rbp`, while even
the 464-byte minimum argv/env block leaves 768 bytes of headroom), and to
indices at or above the computed fault floor. In between,
`oob_mid_region_invariant` asserts the property that does hold for both
programs: a run either survives and prints exactly the normal all-zero dump, or
it dies from a signal having flushed nothing, and neither ever writes to stderr.

### Mismatch 7 — SIGSEGV vs SIGBUS above ~2e8

For very large indices the wild address is gigabytes above the stack and
sometimes lands in a file-backed mapping, where a store past the end of the file
raises **SIGBUS** (exit 135) instead of SIGSEGV (139). Whether it does is up to
ASLR:

| n | C over 30 runs |
|---|---|
| 100 000 000 | 30 × SIGSEGV |
| 536 870 912 | 3 × SIGBUS, 27 × SIGSEGV |
| 1 431 655 765 | 16 × SIGBUS, 14 × SIGSEGV |
| 2 147 483 647 (INT_MAX) | 13 × SIGBUS, 17 × SIGSEGV |

The Rust always raises SIGSEGV. Below ~2×10⁸ the C is always SIGSEGV, so that is
where the strict comparison stops; above it,
`very_large_indices_always_die_without_output` asserts the stable part — both
die from a signal, with no output on either stream.

---

## Checked and already correct

These were verified by differential testing and needed no change. Recording them
because "no mismatch" is also a result:

* **`fgets` framing.** `fgets(inputBuffer, 14, stdin)` takes at most 13 bytes and
  stops after a newline, so a long line is *split across the two call sites*
  rather than discarded: `12345678901234\n` gives `goodB2G` the first 13
  characters and `bad()` the remaining `4\n`. Verified for 13, 14 and 20
  character lines, and for input with no trailing newline.
* **Two independent reads.** `goodB2G` consumes the first line and `bad()` the
  second, so a one-line input drives `bad()` into its `fgets` failure path.
* **`fgets` returning NULL.** Prints `fgets() failed.`, leaves `data` at −1, and
  then `goodB2G` reports `ERROR: Array index is out-of-bounds` while `bad()`
  reports `ERROR: Array index is negative.` — the two sinks emit *different*
  messages for the same `data`. Empty input reaches both.
* **`atoi` semantics** (`(int) strtol(s, NULL, 10)`): leading whitespace skipped,
  one optional sign, stops at the first non-digit, so `0x10` → 0, `7abc` → 7,
  `--5` → 0, `abc` → 0, `""` → 0.
* **`int` truncation.** 13 digits cannot overflow `long`, but the narrowing to
  `int` wraps: `4294967296` → 0, `4294967306` → **10**, `1234567890123` →
  1912239307. Truncation is a second route into the out-of-bounds indices, so
  `4294967314` → 18 and `4294967322` → 26 both reach crashing slots.
* **Embedded NUL.** `fgets` copies a NUL byte into the buffer and `atoi` stops
  there, so `\0` followed by `5` parses as 0, not 5.
* **`goodG2B`'s dead store** (`data = -1;` then `data = 7;`) and the fixed output
  it produces.
* **stderr is never written**, on any path.
* **Exit code 0** on every non-crashing path.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd translation && cargo build --release && cargo test
```

`cargo test` builds the C reference itself if `c_src/build/driver` is missing.
Nothing under `c_src/` is modified; the only addition is the `build/` directory
that CMake creates.
