# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` section and `c_src/CMakeLists.txt` declares no
options or conditional compilation. There is exactly one valid feature
combination:

| combination | Cargo invocation |
|-------------|------------------|
| no features | `cargo ... --no-default-features` |

## Runtime configurations and input shapes

The C source has one public entry point, `main`, no arguments, no public
headers, and no runtime mode or flag. The rows below are the complete
cross-product after pruning equivalent cases. They follow the branches and
boundaries created by `while (fgets(text, 128, stdin))` and
`fputs(text, stdout)`.

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|-----|
| C1 | `main` | no options; immediate EOF (zero input bytes) | [x] |
| C2 | `main` | no options; one newline-terminated chunk of 0-126 bytes before the newline | [x] |
| C3 | `main` | no options; multiple newline-terminated chunks, each shorter than the 127-byte read limit | [x] |
| C4 | `main` | no options; final nonempty chunk is not newline-terminated and is shorter than 127 bytes | [x] |
| C5 | `main` | no options; exactly 127 non-newline bytes, filling one `fgets` result, followed by EOF | [x] |
| C6 | `main` | no options; a logical line longer than 127 bytes, requiring multiple `fgets` calls | [x] |
| C7 | `main` | no options; newline at the read boundary (127th or 128th input byte) | [x] |
| C8 | `main` | no options; embedded NUL before a newline or EOF, causing `fputs` to truncate that `fgets` chunk | [x] |
| C9 | `main` | no options; embedded NULs combined with a line longer than 127 bytes, independently truncating affected chunks | [x] |

Every row must pass many deterministic randomized inputs against both shared
objects before its checkbox is changed.
