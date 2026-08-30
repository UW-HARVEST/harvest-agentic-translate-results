# Differential Verification Errors

No C-versus-Rust runtime mismatches were found. The existing Rust translation
matched the C executable for every enumerated input class, so no translation
code changes were required.

During test development, the first version of the embedded-NUL test used two
adjacent Rust byte-string literals, which does not compile. The cause was a
test-harness syntax error; it was corrected to one byte string before the
differential suite could run.
