# Differential findings: `c_src` vs `translation`

Every mismatch found while comparing the two programs as executables (same
stdin; stdout, stderr and exit status compared), what caused it, and what the
fix was. Reproduce with `cargo test` in `translation/`.

## How the two programs are run

```
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .   # -> c_src/build/driver
cd translation && cargo build --release                                # -> translation/target/release/driver
```

Both read stdin and write stdout; `tests/differential.rs` builds the C side on
demand so `cargo test` is self-contained.

## What the C program branches on

Only **two** `fgets()` calls happen over the whole run. `goodG2B()` hard-codes
`data = 7` and never touches stdin, so the first line goes to `goodB2G()` and
the second to `bad()`. Each is `fgets(inputBuffer, 14, stdin)`, i.e. at most 13
bytes, so a longer line is *split* between the two sinks rather than skipped.

The two sinks differ, and that difference is the point of the file:

| sink | guard | rejection message |
|---|---|---|
| `goodB2G()` | `data >= 0 && data < 10` | `ERROR: Array index is out-of-bounds` |
| `bad()` | `data >= 0` only | `ERROR: Array index is negative.` |

So `bad()` performs `buffer[data] = 1` for any `data >= 10` — an out-of-bounds
stack write, and the source of most of the work below.

---

## Mismatch 1 — out-of-bounds index printed ten zeros instead of crashing

**Inputs:** second line in `16..=19` or `26..=27` (e.g. `"1\n16\n"`).

**C:** killed by SIGSEGV (exit status 139), stdout **empty**.
**Rust (before):** exit 0, full output ending in `Finished bad()`.

**Cause.** The port treated the out-of-bounds write as a discarded no-op, on the
assumption that only the ten in-bounds slots are observable. That is wrong: the
write can land on the frame's control data. `objdump -d` of the binary CMake
builds gives the layout exactly (gcc, no optimisation flags):

```
bad():  sub $0x40,%rsp    buffer          @ rbp-0x40   (10 ints -> rbp-0x19)
                          2-byte gap      @ rbp-0x18
                          inputBuffer[14] @ rbp-0x16
                          i               @ rbp-0x8
                          data            @ rbp-0x4
                          saved rbp       @ rbp+0x0
                          return address  @ rbp+0x8
main(): sub $0x10,%rsp => main's rbp == bad's rbp + 0x20
                          argc/argv       @ rbp+0x10 .. rbp+0x1f
                          main saved rbp  @ rbp+0x20
                          main ret addr   @ rbp+0x28
```

`buffer[i]` is at `bad_rbp - 0x40 + 4*i`, so:

| index | target | outcome |
|---|---|---|
| `0..=9` | `buffer` | in bounds |
| `10..=13` | gap + `inputBuffer` (already consumed) | unobservable |
| `14` | `i` — reset to 0 by the very next statement | unobservable |
| `15` | `data` — never read again | unobservable |
| `16..=17` | `bad()`'s saved rbp | **SIGSEGV** |
| `18..=19` | `bad()`'s return address | **SIGSEGV** |
| `20..=23` | `main()`'s `argc`/`argv` — dead | unobservable |
| `24..=25` | `main()`'s saved rbp — never dereferenced | unobservable |
| `26..=27` | `main()`'s return address | **SIGSEGV** |

The write happens *before* the print loop, but the damage is only detected on
`ret`, so C prints all ten zeros first and then dies. stdout still comes out
empty because it is fully buffered when it is a pipe, the whole output is far
under one buffer, and `exit()` never runs to flush it.

**Fix.** `oob_write_is_fatal()` in `src/main.rs` encodes the table above. The
port prints the ten values, then terminates without flushing.

## Mismatch 2 — the crash was SIGABRT (134) rather than SIGSEGV (139)

**Inputs:** the same ones as mismatch 1, after that fix.

**Cause.** `raise(SIGSEGV)` returned instead of killing the process. Rust's
runtime installs a SIGSEGV handler to turn stack overflow into a clean abort;
for a fault outside the guard page that handler simply returns, so control fell
through to the `abort()` guard on the next line.

**Fix.** Restore the default disposition first — `signal(SIGSEGV, SIG_DFL)` —
then `raise(SIGSEGV)`. Both are declared in an `extern "C"` block; the
`*-linux-gnu` targets already link libc, so no new dependency. stderr stays
empty, matching C (the `Segmentation fault (core dumped)` line comes from the
shell, not the process).

## Mismatch 3 — far out-of-bounds indices did not crash

**Inputs:** long lines and large values, e.g. `"11111111111111111111\n"`
(`bad()` receives the leftover `1111111`) and `"2147483647\n2147483647\n"`.

**C:** killed (139). **Rust (before):** exit 0.

**Cause.** Above `main()`'s frame the write walks up through the argv/envp block
towards the top of the `[stack]` mapping and faults once it leaves it. The
initial fix stopped modelling faults past index 27 entirely.

**Fix.** Compare the target address against the top of `[stack]` from
`/proc/self/maps`, using the address of the Rust `buffer` as the stand-in for
C's. That reproduces the real mechanism instead of hard-coding a threshold, and
it tracks the environment size — see the next section for why that matters.

Verified stable over repeated runs after the fix: indices `28..=200` exit 0 on
both, indices `1_000_000 .. 16_777_216` exit 139 on both, under both an empty
and a padded environment.

---

## Not reproducible by any program: the stack-top boundary

Two input classes are excluded from the test suite because **the C program is
not a function of its input there** — the same input, the same binary, gives
different answers on different runs. No translation can match them, so
asserting on them would only produce a flaky suite.

**(a) Whether the write faults at all**, for indices near the top of the stack
mapping. The first faulting index scales with the size of the argv/envp block,
and the kernel's per-run stack randomisation smears it into a wide band rather
than a clean edge. Measured on the C binary, one index per column, three runs
in a normal ~3 KB environment:

```
        1300  1400  1500  1600  1800  2000  2500  3000
run 1    139     0     0   139     0     0     0   139
run 2      0     0     0     0     0     0     0   139
run 3      0   139     0   139     0   139   139   139
```

The band moves with the environment size, in both directions:

| environment | first index observed to fault |
|---|---|
| `env -i` (empty) | ~250 |
| normal, ~3 KB | ~1200 |
| ~3 KB + 32 KiB of padding | ~16000 |

The tests therefore assert exact agreement only where it is environment-
independent: indices `<= 200` never fault (verified under an empty environment,
the worst case), and indices `>= 1_000_000` always do (ARG_MAX caps the
argv/envp block well under the 4 MiB such an index reaches, verified under both
an empty and a 32 KiB-padded environment). `oob_write_far_past_stack_top_is_fatal`
and `oob_write_above_main_frame_is_unobservable` cover those two regions.

The band in between is covered by `oob_write_outcome_is_always_one_c_could_produce`,
which asserts the property that *does* hold for every index: the run ends either
at exit 0 with the complete transcript, or killed by a signal with **empty**
stdout and empty stderr — never a partial transcript, a flushed buffer, or a
panic message. That is what would break if the port modelled the crash sloppily,
and it cannot go flaky.

**(b) Which signal**, for very large indices. Once `4*index` reaches a few GiB
the target address can pass the x86-64 canonical boundary
(`0x0000_7fff_ffff_ffff`), which is reported as SIGBUS rather than SIGSEGV, and
whether it does depends on where ASLR put the stack — the top varies across a
~16 GiB range (`0x7ffc_…` to `0x7fff_f…`, measured from `/proc/self/maps`). Five
runs of the C binary per index:

```
  index          offset     runs
  10_000_000     0.04 GiB   139 139 139 139 139
  268_435_456    1.00 GiB   139 139 139 139 139
  536_870_912    2.00 GiB   139 139 139 139 139
  1_073_741_824  4.00 GiB   139 139 139 135 139
  2_000_000_000  7.45 GiB   135 139 139 139 139
  2_147_483_647  8.00 GiB   139 135 139 139 135     (135 = SIGBUS)
```

Below roughly a 1 GiB offset it is always SIGSEGV, which is why the fatal-index
test stops at 16_777_216 (a 64 MiB offset). Above that the signal is a coin
flip; the port always raises SIGSEGV, the more frequent outcome, but the split
itself is unmatchable. For reference,
`"2222222222222222222222222222222222222222\n"` (`bad()` receives index
1724130190) over ten runs of the C binary:

```
135 139 135 139 139 139 135 139 135 135      (135 = SIGBUS, 139 = SIGSEGV)
```

The port raises SIGSEGV, which is the majority outcome, but the split itself is
unmatchable.

---

## Behaviours checked and already correct

These were verified rather than fixed; they are the places a port would
plausibly drift, and the tests pin them down.

- **`fgets` is not `scanf`.** It stops at the newline, keeps it in the buffer,
  and takes at most 13 bytes — so `"12345678901234\n"` feeds `goodB2G()` the
  first 13 digits and `bad()` the remainder. A port that read "one line per
  call" would diverge on every line longer than 13 bytes.
- **`fgets` returning NULL** only at EOF-with-nothing-read: empty input prints
  `fgets() failed.` twice; `"5"` with no newline is a successful read, and only
  the *next* call fails.
- **`atoi` is `(int)strtol`.** Leading whitespace (space, `\t`, `\n`, `\v`,
  `\f`, `\r`) skipped, one optional sign, digits only, decimal even with leading
  zeros (`"007"` is 7), stops at the first non-digit, and truncates on overflow:
  `"2147483648"` becomes `INT_MIN` and therefore takes the *negative* branch.
  `"abc"`, `"+"`, `"-"` and `""` all give 0, i.e. index 0.
- **NUL bytes** are copied through by `fgets` and stop `atoi`.
- **Non-UTF-8 input** must not be rejected, which is why the port works in
  bytes throughout rather than in `String`.
- **Output formatting**: `printf("%s\n", ...)` / `printf("%d\n", ...)`, one
  value per line, trailing newline present, nothing on stderr on any path.
- **`argc`/`argv`** are declared in `main` and never used; arguments change
  nothing.

## Branches unreachable by any input

Two of the C's eight conditionals cannot be reached from stdin, so no test
covers them — they are dead by construction, not untested:

- `printLine`'s `if (line != NULL)` — every call site passes a string literal.
- `goodG2B`'s `else` (`data >= 0` false) — `data` is assigned the constant 7 on
  the line above.

The port keeps both, so the structure still corresponds line for line.
