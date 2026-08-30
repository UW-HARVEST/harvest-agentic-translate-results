# Differential verification of the C→Rust translation

The C program (`c_src/src/main.c`) is the ground truth:

```c
int main() {
    float x = 0.f;
    scanf("%f", &x);   // on matching/input failure x is left at 0.f
    driver(x);         // prints the 4 in-memory bytes of x as %02x, then '\n'
    return 0;
}
```

All observable behaviour therefore comes from one place: how glibc's
`scanf("%f", ...)` decides what to consume and what bit pattern it produces.
`print_hex` and `driver` are branch-free (`len` is always `sizeof(float)`), and
`main` always returns 0.

Commands used:

- C: `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .`
  → `c_src/build/driver`
- Rust: `cd translation && cargo build --release`
  → `translation/target/release/driver`
- Tests: `cd translation && cargo test` (`tests/differential.rs`, 26 tests,
  none ignored). Each case spawns both binaries, writes the same bytes to
  stdin, and compares stdout, stderr and exit status (including the terminating
  signal on Unix).

## Mismatches found

### 1. `SIGPIPE` disposition — exit status differed

**Symptom.** With stdout connected to a pipe whose read end had already been
closed, the two programs ended differently:

| program | result |
| --- | --- |
| C | killed by signal 13 (`SIGPIPE`), no output |
| Rust | exit code 0, no output |

Reproduced by creating a pipe, closing the read end, and handing the write end
to the child as its stdout.

**Cause.** The Rust standard library installs `SIG_IGN` for `SIGPIPE` before
`main` runs. The failing `write` then returns `EPIPE`, which `print_hex`
discards (matching C's `printf`, whose return value is also ignored), and the
program falls off the end of `main` with status 0. The C program inherits the
default disposition, so the same `write` kills it with signal 13 before it can
return.

**Fix.** `translation/src/main.rs` now restores the default disposition as the
first thing `main` does, via a direct `signal(SIGPIPE, SIG_DFL)` call:

```rust
#[cfg(unix)]
fn restore_default_sigpipe() {
    extern "C" { fn signal(signum: i32, handler: usize) -> usize; }
    const SIGPIPE: i32 = 13;
    const SIG_DFL: usize = 0;
    unsafe { signal(SIGPIPE, SIG_DFL); }
}
```

This is inert for every other input; it only matters when a write to stdout
fails with `EPIPE`. Covered by
`stdout_reader_closed_gives_the_same_signal`.

## Input classes enumerated, and the behaviour each one pins down

No other mismatch was found. The cases below all agree on stdout, stderr and
exit status; they are listed because each one is a distinct decision inside the
`%f` conversion, and several of them look surprising enough to be worth
recording as *confirmed* rather than assumed.

| class | inputs | C behaviour |
| --- | --- | --- |
| input failure (EOF before any conversion) | `""`, `"\n"`, `"   "`, `"\t\r\x0b\x0c"`, `/dev/null`, closed stdin | `x` stays `0.f` → `00000000` |
| leading white space skipped **across newlines** | `"\n\n\n1.5"`, `" \t1.5"`, 10 000 spaces then `1.5` | `scanf` keeps reading past line ends, unlike `fgets` |
| matching failure | `abc`, `.`, `-.`, `-`, `+`, `--1`, `e5`, `..`, `$1.5`, any single non-numeric byte | nothing consumed, `x` stays `0.f` → `00000000` |
| trailing junk ignored | `12abc`, `1.5.5`, `1 2`, `0..5`, `1.5` + 10 000 `z` | first token only |
| signed zero | `-0`, `-0.`, `-.0`, `-0e`, `-0e+`, `-00x`, `-0x0p` | sign is applied to zero → `00000080` |
| digits on one side of the point | `.5`, `5.`, `-.5`, `000.` | valid |
| incomplete decimal exponent | `1e`, `1e+`, `1e-`, `1e+x`, `0.5e` | the `e…` is not consumed; the mantissa still converts (`1e` → `1.0`) |
| `inf` / `infinity` | `inf`, `INF`, `iNf`, `-inf`, `infinity`, `InFiNiTy`, `infinityx` | infinity, sign applied |
| **partial** `infinity` | `infi`, `infin`, `infini`, `infinit`, `-infinit` | matching failure → `00000000`; glibc cannot back out of a partial suffix match |
| `inf` followed by other text | `inf1`, `infz`, `inf inity` | infinity (`inf` alone matched) |
| `nan` | `nan`, `NaN`, `-nan`, `+nan` | quiet NaN `0000c07f`; `-nan` sets the sign → `0000c0ff` |
| `nan(payload)` | `nan(123)`, `nan(0x7)`, `-nan(abc)`, `nan(` | payload is **not** parsed by `%f`; result is the same `0000c07f` |
| hex float | `0x1p3`, `0X1p-3`, `0x.8p1`, `0x1.8`, `0x123456789abcdef1p0`, `0xABCDEF` | C99 hex form accepted |
| bare `0x` prefix | `0x`, `0X`, `-0x`, `0xz`, `0xp1` | rejected → `00000000` |
| `0x` + point, no hex digits | `0x.`, `-0x.`, `0x.z`, `-0x.p1` | the leading `0` still converts, **and keeps its sign**: `-0x.` → `00000080` |
| incomplete hex exponent | `0x1p`, `0x1p+`, `0x1px`, `0x1.8p` | `p…` not consumed; significand still converts |
| overflow | `1e39`, `1e999`, `3.4028236e38`, `0x1p128`, `1e999999999999999999999` | `±inf` (`0000807f` / `000080ff`) |
| underflow | `1e-999`, `0x1p-150`, `1e-46`, `-1e-999` | `±0` |
| exponent-accumulator saturation | `1e999999999`, `1e1000000000`, `1e1000000001`, `1e00000000000000000000005`, `0x1p±1000000000` | value is already inf/0 well before the clamp, so clamping is unobservable |
| 24-bit significand truncation | `16777215`…`16777220`, `2147483648`, `4294967296`, `8388608.5`, `8388609.5` | round to nearest, ties to even |
| exact halfway points | midpoints between 16 chosen adjacent `f32` bit patterns, printed to 60 digits, plus a hair either side | ties to even; the hair decides the direction |
| subnormals | `k·2^-150` and `(k+0.25)·2^-150` for k = 1…39, `0x{k}p-150`, `1e-45`, `1.4e-45`, `0.75e-45` | correct subnormal rounding, including rounding up into the normal range |
| very long input | 100 000 digits, 100 000-digit fraction, 10 000 hex digits, `1e` + 10 000 nines | no buffer limit; correct value |
| embedded NUL / non-UTF-8 | `"\x00"`, `"1\x002"`, `"\xff1.5"`, bytes 128–255 | bytes are handled as bytes; NUL terminates the token |
| exhaustive short inputs | every single byte 0–255; every 2-byte pair over `0123456789.eEpPxX+- \t\nabcfinty()` | all agree |
| randomised sweep | 1 400 generated inputs (garbage strings, decimal shapes, hex shapes) from a fixed-seed xorshift, plus ~50 000 additional inputs from an out-of-tree Python fuzzer during investigation | all agree |

## Notes on the translation

- `Input` reads stdin one byte at a time on demand, so — like `scanf` — the
  program does not wait for EOF once the conversion has finished.
- Write errors from `print_hex` are ignored, matching C's ignored `printf`
  return value; with the `SIGPIPE` fix above the process dies at the write
  instead, exactly as the C does.
- Nothing in `c_src/` was modified. The only addition there is the
  `c_src/build/` directory produced by CMake.
