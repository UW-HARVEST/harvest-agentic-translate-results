# Differential Error Log

No C-to-Rust behavioral mismatches were found.

The differential suite compares stdout, stderr, and exit status for:

- empty input and immediate EOF
- single-byte, newline-only, and single-line input
- multiple lines, empty lines, and a final line without a newline
- the 127-byte `fgets` payload boundary, including exact and split reads
- embedded NUL bytes, including a NUL at the read boundary
- non-UTF-8 bytes and carriage returns
- ignored command-line arguments
- long input spanning many reads
- a real stdin read error
- a real stdout write error

The existing Rust implementation matched the C executable in every case, so no
translation change was required.
