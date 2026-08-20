# ERRORS.md — Phase C error-surface table

Derived mechanically from the complete C source (`c_src/src/main.c`, 68 lines,
one translation unit). The grep sweep below is exhaustive for that file.

## Mechanical sweep of every rejection construct

```
$ grep -nE 'return|NULL|assert|RETURN_ERROR|errno|exit|<|>|<=|>=|==|!=|MAX|MIN|abort' c_src/src/main.c
27:    if (line != NULL)      <- the only validation in the file
64:    return 0;              <- main's unconditional success return
```

Findings:

* error-return macros (`RETURN_ERROR`, …): **none**
* `return -1` / `return NULL` / error enums / error codes: **none**
  (`printLine`, `bad`, `good` are all `void`; `main` unconditionally
  `return 0;`)
* `assert` / `abort` / `exit`: **none**
* explicit range checks, min/max constants, length/size parameters: **none**
  (no function takes a length, count, index, size, mode or enum argument)
* null checks: **exactly one** — `printLine`'s `if (line != NULL)`
* allocation, I/O-error or `errno` handling: **none** (the `puts` return value
  is discarded, so write errors are silently ignored)

Because the C library has no error codes and no fallible return values, every
"rejection" is observable **only** as *"no bytes are written and the call
returns normally"*. The differential tests therefore assert both
implementations produce byte-identical stdout **and** do not crash.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| E1 | `printLine` | `line == NULL` (`if (line != NULL)` guard fails) | no output at all, returns normally (void) | `err_e1_print_line_null` | ✅ pass |
| E2 | `printLine` | `line != NULL` but points at an immediate NUL (`""`), i.e. the zero-length "degenerate" input | `puts("")` ⇒ exactly one `"\n"` byte | `err_e2_print_line_empty` | ✅ pass |
| E3 | `printLine` | NULL passed *repeatedly* / interleaved with valid strings — the guard must not latch or corrupt later output | only the valid strings are printed, in order | `err_e3_null_interleaved_with_valid` | ✅ pass |
| E4 | `printLine` | oversized input: 1 MiB string (far past `BUFSIZ`), and lengths exactly `BUFSIZ-1 / BUFSIZ / BUFSIZ+1` (4095/4096/4097 and 8191/8192/8193) — no length is validated, so nothing may be truncated | full string + `"\n"`, never truncated | `err_e4_oversized_lengths` | ✅ pass |
| E5 | `printLine` | non-UTF-8 / non-ASCII bytes (`0x80`–`0xFF`), including lone continuation bytes and overlong-encoding-looking sequences — C copies raw bytes and never validates encoding | bytes copied verbatim + `"\n"` (no replacement chars, no panic) | `err_e5_invalid_utf8_bytes` | ✅ pass |
| E6 | `printLine` | every single byte value `0x01..=0xFF` as a 1-byte string (`0x00` is unrepresentable: it terminates the string) | that byte + `"\n"` | `err_e6_all_single_byte_values` | ✅ pass |
| E7 | `printLine` | string containing `printf` format directives (`%s %d %n %p %%`) — the C format string is the fixed `"%s\n"`, so these are **data**, never interpreted | the literal characters + `"\n"` (no format-string interpretation, no crash on `%n`) | `err_e7_format_specifiers_are_data` | ✅ pass |
| E8 | `printLine` | string containing embedded control bytes: `\n`, `\r`, `\t`, `\x1b`, `\x7f` | bytes verbatim + one trailing `"\n"` | `err_e8_embedded_control_bytes` | ✅ pass |
| E9 | `main` | out-of-range/degenerate `argc`/`argv` across the FFI boundary: `argc = 0`, `argc = -1`, `argc = INT_MIN`, `argc = INT_MAX`, `argv = NULL` — the C body never dereferences them | the fixed 7-line output, return value `0` | `err_e9_main_degenerate_argc_argv` | ✅ pass |
| E10 | `bad` / `good` | no arguments exist to invalidate; boundary is repeated invocation (no internal state may accumulate) and the fact that `bad()` must **not** call the `static helperBad()` | `bad` prints only `"bad()\n"`; `good` prints `"good()\nhelperGood()\n"`; identical on every repetition | `err_e10_no_arg_entry_points_repeated` | ✅ pass |
| E11 | *(whole library)* | out-of-range enum values passed across the FFI boundary | **not applicable** — the C API declares no enum, and no `int`/flag parameter other than `main`'s `argc` (covered by E9). Documented rather than invented. | `err_e9_main_degenerate_argc_argv` (argc = `INT_MIN`/`INT_MAX` stands in for the "int with no valid variant" case) | ✅ pass |

### Deliberately not tested (undefined behaviour in C)

| condition | why it is excluded |
|-----------|--------------------|
| `printLine(p)` where `p` is non-NULL but not NUL-terminated, or dangling/unmapped | C `puts` reads past the buffer ⇒ undefined behaviour (typically SIGSEGV). There is no defined C result to match, so any Rust behaviour is conformant; a differential test would only compare two crashes. |
| `printLine` with fd 1 closed | `puts` fails and sets the stream error flag; the C code discards the result, and no bytes are observable either way. Nothing is asserted about an unobservable state. |

**All 10 applicable rows (E1–E10, with E11 documented as N/A and covered by E9)
have passing differential tests.**
