# Differential Test Mismatches

No mismatches were found.

The C and Rust executables matched for stdout, stderr, and exit status across
the following input classes:

- empty input and EOF after one line
- minimum and maximum valid indices (`0` and `9`)
- negative and upper out-of-bounds input at the checked sink
- negative input at the unchecked sink
- nonnumeric input, leading whitespace, signs, and numeric suffixes
- 32-bit integer truncation from `atoi`
- embedded NUL bytes
- the 13-byte `fgets` boundary and line-oriented reads
- trailing input ignored after both reads

The unchecked C sink is only exercised with indices in its valid array range or
with a negative index. Values above `9` would invoke undefined out-of-bounds
memory access in the C executable and are not an input class the C code handles.
