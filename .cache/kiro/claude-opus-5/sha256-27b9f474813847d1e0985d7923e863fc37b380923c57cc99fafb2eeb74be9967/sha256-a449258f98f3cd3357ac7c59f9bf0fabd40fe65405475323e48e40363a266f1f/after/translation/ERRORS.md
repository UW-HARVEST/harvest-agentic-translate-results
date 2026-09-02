# ERRORS.md — differential verification of the C → Rust translation

Scope: `c_src/src/main.c` (the whole program) versus `translation/src/main.rs`.

## What the C program does

```c
static void print_hex(unsigned char *p, int len) {
    for (int i = 0; i < len; i++) printf("%02x", p[i]);
    printf("\n");
}
void driver(int x) { print_hex((unsigned char *)&x, sizeof(x)); }
int main() { int x = 0; scanf("%d", &x); driver(x); return 0; }
```

`main` has exactly one branch point, and it is inside `scanf`, whose return
value is discarded:

| `scanf("%d", &x)` outcome | effect on `x` |
| --- | --- |
| conversion succeeds | `x` = converted value |
| EOF before any non-whitespace | untouched, stays `0` |
| matching failure (no digit at the conversion point) | untouched, stays `0` |

`print_hex`'s loop bound is the constant `sizeof(int)` (4 here), so output is
always exactly 8 lowercase hex digits plus `\n`. `main` always `return 0`.
Nothing is ever written to stderr. All remaining behavioural variation lives in
glibc's `%d` conversion.

## Result: no mismatches found

Every input exercised produced byte-identical stdout, byte-identical stderr and
an identical exit status. No change to `translation/src/main.rs` was required,
and `c_src/` was not modified (only the generated `c_src/build/` directory was
added, per the build instructions).

Because there was nothing to fix, this file records what was checked and the
hazards that *would* have been mismatches under a naive translation, each with
the observed agreeing output. Those are the entries a future reader should
re-check first if the Rust is ever edited.

## Hazards verified, not mismatches

Each row was run through both binaries. "Naive Rust" is what the obvious
implementation (`read_line` + `trim` + `parse::<i32>()`) would have produced.

| # | stdin | Both programs print | Naive Rust would print | Why |
| --- | --- | --- | --- | --- |
| 1 | `1` | `01000000` | `00000001` | The bytes are dumped in **host order**. x86-64 is little-endian, so the low byte comes first. Requires `to_ne_bytes`, not `to_be_bytes`. |
| 2 | *(empty)* | `00000000` | *(parse error path)* | `scanf` returns `EOF` and never writes through the pointer; `x` keeps its initialiser. The return value is discarded, so there is no error output and the exit status is still 0. |
| 3 | `abc` | `00000000` | *(parse error path)* | Matching failure. Same as above: `x` untouched, exit 0, empty stderr. |
| 4 | `-` / `+` / `--5` / `- 5` | `00000000` | *(parse error path)* | A sign with no digit following it is a matching failure, not a zero. Same observable result here only because `x`'s initialiser is `0`. |
| 5 | `2147483648` | `00000080` | *(overflow error)* | glibc converts with `strtol` into a 64-bit `long`, then stores via `*(int *)`, **truncating**. `0x80000000` → `INT_MIN`. No error is reported. |
| 6 | `4294967296` | `00000000` | *(overflow error)* | Same truncation: the low 32 bits of 2^32 are zero. |
| 7 | `2147483648999` | `e7030000` | *(overflow error)* | Low 32 bits of 2147483648999 = 999. |
| 8 | `9223372036854775808` | `ffffffff` | *(overflow error)* | Past `LONG_MAX`, `strtol` **saturates** to `0x7fffffffffffffff`; truncating that to `int` gives `-1`. Saturation, not wrapping — this is the subtle one. |
| 9 | `-9223372036854775809` | `00000000` | *(overflow error)* | Past `LONG_MIN`, saturates to `0x8000000000000000`; low 32 bits are zero. |
| 10 | `999…9` (10 000 nines) | `ffffffff` | *(overflow error)* | Saturation again; the digit run length is unbounded, so the implementation must keep consuming digits after it has already saturated rather than overflowing or bailing out. |
| 11 | `-999…9` (10 000 nines) | `00000000` | *(overflow error)* | Negative saturation counterpart of #10. |
| 12 | `\n\n\n42` | `2a000000` | `00000000` | `%d` skips leading whitespace **across newlines**. `fgets`/`read_line` would stop at the first newline and see an empty line. |
| 13 | 100 000 spaces then `42` | `2a000000` | — | The whitespace skip is unbounded and must not be capped by any internal buffer size. |
| 14 | ` \t\n\r\x0b\x0c 77` | `4d000000` | — | All six C-locale whitespace characters are skipped, including vertical tab (`0x0b`) and form feed (`0x0c`). |
| 15 | `12 34` | `0c000000` | — | Only one conversion is performed; the rest of stdin is never read. |
| 16 | `0x10` | `00000000` | — | `%d` is decimal-only. It converts `0` and stops at `x`; it does **not** honour the `0x` prefix the way `%i` or `strtol(…, 0)` would. |
| 17 | `007` | `07000000` | — | Leading zeros are not octal under `%d`. |
| 18 | `5.5` / `5abc` / `5\xff` | `05000000` | — | Conversion stops at the first non-digit and the trailing bytes are simply left unread. |
| 19 | `\x00123` | `00000000` | — | NUL is neither whitespace nor a digit, so it is a matching failure. It is not a terminator that gets skipped. |
| 20 | `\x80 5`, `\xa0 5`, `\xff\xfe abc` | `00000000` | — | High bytes are not whitespace in the C locale (notably `0xa0`, the Latin-1 non-breaking space), so they cause a matching failure. Any translation that used a Unicode-aware `char::is_whitespace` would skip `0xa0` and print `05000000`. |
| 21 | `１２３` (U+FF11…, UTF-8) | `00000000` | — | Full-width digits are not ASCII digits. A translation using `char::is_numeric` would diverge. |
| 22 | `-0` | `00000000` | — | Negative zero collapses to `0`; no sign bit survives. |

The reason the translation is already correct on all of these is that
`translation/src/main.rs` does not use `parse()`. It reimplements the glibc
conversion directly: a byte-at-a-time reader with one pushback slot, an
`is_c_space` limited to the six C-locale whitespace bytes, an explicit digit
loop that sets a `saturated` flag and keeps consuming digits afterwards,
`strtol`'s clamp to `i64::MIN`/`i64::MAX`, and a final `wide as i32`
truncation. Byte order comes from `to_ne_bytes`.

## Inputs exercised

Permanent suite — `translation/tests/differential.rs`, 25 tests, 0 ignored.
Each test spawns both binaries as subprocesses, writes the same bytes to stdin,
and asserts on all three of stdout, stderr and exit status. The Rust code is
never linked as a library. The C binary is built on demand via `cmake` by the
test harness itself.

Exhaustive coverage in the suite:

- all 256 single-byte inputs
- all 529 two-byte inputs over `0-9 + - SP TAB LF CR VT FF x a . NUL 0xff`
- all 729 three-byte inputs over `0 1 8 9 + - SP LF NUL a`
- every integer in `-300..=300`, in six surface forms each (bare, signed,
  leading space, trailing newline, leading zero, trailing junk)
- 2^k − 2 … 2^k + 2 for k = 0..71, both signs — straddling the 32-bit and
  64-bit boundaries and running past them
- digit-run lengths 1..25, both signs
- each leading digit 0-9 followed by 18 zeros, both signs
- input lengths 1, 2, 127/128/129, 255/256/257, 1023/1024/1025,
  4095/4096/4097, 8191/8192/8193, 65535/65536/65537, with five fillers each

Additional throwaway fuzzing (not part of the suite) compared 17 594 inputs,
including 3 000 random raw byte blobs and 3 000 random blobs over the
`scanf`-relevant alphabet: 0 mismatches.

Also confirmed to agree, outside the harness:

- stdin redirected from `/dev/null`, from a closed descriptor (`<&-`), and from
  a directory — all `00000000`, exit 0
- stdout closed (`>&-`) and stdout connected to a pipe that closes early — both
  exit 0. No SIGPIPE divergence: the C `printf` failure is discarded and Rust
  ignores SIGPIPE by default, so neither dies by signal.
- `LC_ALL` set to `C`, `C.UTF-8`, `en_US.UTF-8`, `de_DE.UTF-8`, `POSIX` —
  identical output. `%d` does not do locale digit grouping (that would need
  `%'d`).

## Known non-observable difference

The two programs consume different amounts of stdin. glibc's `scanf` reads a
full stdio block ahead; the Rust reader consumes only the bytes the conversion
needs. This is invisible through stdout, stderr and exit status — the process
exits immediately afterwards and no other reader inherits the descriptor — so
it is not a mismatch under the comparison being made. It would only become
observable if the program were changed to hand the remaining stdin to a child
process or to `exec`.

## Reproducing

```sh
cd c_src && mkdir -p build && cd build && cmake .. && cmake --build .
cd ../../translation && cargo build --release && cargo test
```
