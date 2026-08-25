# Error Surface

The public C API is `int main(void)` in effect: it has no pointer, length, enum,
or option parameters. Consequently, caller-supplied null pointers, zero or
oversized lengths, and out-of-range enum values do not exist at this FFI
boundary.

The source has no `assert`, error enum, range rejection, null check,
`RETURN_ERROR`, `return -1`, or `return NULL`. Its complete I/O error surface
comes from the return values of the two stdio calls:

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| E1 | `main` / `fgets` | `stdin` is unreadable, so `fgets(text, 128, stdin)` returns `NULL` | loop terminates and `main` returns `0` | [x] |
| E2 | `main` / `fputs` | `stdout` is unwritable, so `fputs(text, stdout)` returns `EOF` | return value is ignored; input processing continues to EOF and `main` returns `0` | [x] |
