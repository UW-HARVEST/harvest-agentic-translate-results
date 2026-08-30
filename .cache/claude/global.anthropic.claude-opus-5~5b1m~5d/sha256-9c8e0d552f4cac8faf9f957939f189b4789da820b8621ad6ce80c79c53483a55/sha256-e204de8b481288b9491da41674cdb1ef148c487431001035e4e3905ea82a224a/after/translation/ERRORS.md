# Differential verification log — `c_src/src/main.c` vs. `translation/`

## Summary

**No behavioral mismatches were found.** The Rust translation is byte-identical to
the C program on stdout, stderr and exit status across the *entire* reachable
input space (see "Coverage argument" below), so no fix to `src/main.rs` was
required.

This file records the C behaviors that were verified, the traps that a naive
translation falls into (each confirmed to be *correctly* handled here, and each
proven to be caught by the test suite via mutation testing), and the input
classes enumerated from the C source.

## The program under test

```c
void printHexCharLine(char charHex) { printf("%02x\n", charHex); }

int main() {
    char data;
    data = ' ';
    fscanf(stdin, "%c", &data);
    { char result = data + 1; printHexCharLine(result); }
    return 0;
}
```

## Build / run commands

| | command |
|---|---|
| Build C | `cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .` |
| Run C | `c_src/build/driver` (reads stdin) |
| Build Rust | `cd translation && cargo build --release` |
| Run Rust | `translation/target/release/driver` (reads stdin) |
| Test | `cd translation && cargo test` (builds the C binary itself if absent) |

## Coverage argument: the input space is finite and fully enumerated

The program's observable behavior depends on exactly one thing: the value that
`data` holds after the `fscanf`. That gives 257 distinct reachable states:

1. the `%c` conversion **fails** → `data` keeps its initialized `' '` (0x20);
2. the `%c` conversion **succeeds** → `data` is one of the 256 possible bytes.

`tests/differential.rs` covers all 257 exhaustively
(`exhaustive_every_single_byte_value` plus the three conversion-failure tests),
so there is no unexercised path, `if`, or early return in the C source. Nothing
in the program branches on input *length*, so there is no "maximum the code
handles" beyond the first byte; that is separately pinned by feeding 1 MiB of
input and asserting the trailing bytes are ignored.

## Behaviors verified, and the traps a naive translation hits

### 1. `char` is **signed** on this ABI (x86-64 Linux), and `%x` reinterprets the promoted `int`

`printHexCharLine` takes a `char`, which is promoted to `int` for the variadic
`printf` call; `%x` then reads that `int` as an `unsigned int`. For any negative
`char` the result is therefore **eight** hex digits, not two:

| stdin byte | `data + 1` as `signed char` | C stdout |
|---|---|---|
| `0x41` (`'A'`) | `0x42` | `42` |
| `0x7e` | `0x7f` | `7f` |
| `0x80` | `-127` | `ffffff81` |
| `0xfe` | `-1` | `ffffffff` |

*Trap:* translating `char` as Rust `u8` yields `81` where C prints `ffffff81`.
The translation avoids this with `char_hex as i32 as u32`.
Verified by mutating to `as u8 as u32` → 4 tests fail.

### 2. Signed overflow at `0x7f` wraps to `-128`

`0x7f + 1` overflows `signed char`. The compiled C truncates to `-128`, which
prints as `ffffff80`.

*Trap:* a saturating or panicking add gives `7f` or a crash.
The translation uses `wrapping_add(1)`.
Verified by mutating to `saturating_add` → 3 tests fail.

### 3. `0xff` is the one negative byte that produces short output

`-1 + 1 == 0`, so the output is `00`, not `ffffff00`.

### 4. `%02x` is a **minimum** field width, never a truncation

Input `0x00` → result `0x01` → `01` (padded). Input `0x80` → `ffffff81`
(8 digits, *not* clipped to 2).

*Trap:* using `{:x}` drops the pad; using `{:02x}` on a truncated 2-digit value
drops the sign extension.
Verified by mutating to `{:x}` → 3 tests fail.

### 5. A failed conversion leaves `data` at its initialized `' '`

`data` is explicitly set to `' '` *before* the `fscanf`, so when no byte can be
read the program prints `21` (0x20 + 1) and still exits **0**. There is no error
path, no diagnostic on stderr, and no non-zero exit anywhere in this program.

Three separate ways to reach this state are tested:
* empty stdin (closed pipe),
* stdin redirected from `/dev/null`,
* stdin redirected from a **directory** — `open(2)` succeeds on Linux but
  `read(2)` fails with `EISDIR`, so the conversion fails on an I/O *error*
  rather than on EOF. C prints `21`, exit 0, empty stderr; Rust matches because
  it ignores both `Ok(0)` and `Err(..)` from `read`.

*Trap:* initializing to `0` instead of `' '`, or treating a read error as fatal
(`unwrap()`/`expect()` would exit non-zero and write a panic message to stderr,
where C silently exits 0).
Verified by mutating the initializer to `0` → 3 tests fail.

### 6. `fscanf("%c")` does **not** skip leading whitespace and does **not** stop at newlines

Unlike `%d`/`%s`, the `%c` conversion has no leading-whitespace skip; unlike
`fgets`, it is not line-oriented. The very first byte on stdin always wins:

| stdin | C stdout | why |
|---|---|---|
| `"   x"` | `21` | reads the space, not `'x'` |
| `"\n"` | `0b` | the newline itself is a successful conversion |
| `"\nX"` | `0b` | reads the newline, not `'X'` |
| `"\tx"` | `0a` | reads the tab |
| `"ABC"` | `42` | only `'A'` is consumed |

*Trap:* using `read_line`/`trim`, or skipping whitespace, changes the answer for
all of these.

### 7. Input need not be valid UTF-8

`0x9f`, `0xe2 0x82`, `0xff 0xfe 0xfd` are all read as raw bytes.

*Trap:* `read_to_string` / `lines()` errors or panics on invalid UTF-8, where C
happily reads the byte. The translation reads into a `[u8; 1]`.

### 8. Excess input is discarded without error

1 MiB of input (larger than any stdio buffer) still prints one line; neither
program blocks, errors, or dies of `SIGPIPE` on the undrained pipe.

## Test-suite integrity

* 18 tests in `translation/tests/differential.rs`, all passing in both debug
  (`cargo test`) and release (`cargo test --release`).
* Every test compares **stdout, stderr and exit status**; none compares stdout
  alone.
* Both programs are driven as **subprocesses**; the Rust code is never loaded as
  a library, and there are no `#[no_mangle]`/cdylib/libloading constructs.
* No test is `#[ignore]`d, skipped or disabled.
* The suite was mutation-tested: four independent single-line regressions were
  injected into `src/main.rs` (unsigned `char`, wrong initializer, saturating
  add, dropped field width) and **each was caught by multiple tests**. The
  original source was restored and re-verified afterwards.
* `c_src/` was never modified (only the ignored `c_src/build/` output directory
  is created there by the build).
