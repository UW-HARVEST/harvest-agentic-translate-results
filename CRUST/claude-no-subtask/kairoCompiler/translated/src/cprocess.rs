use crate::compiler::CompileProcess;
/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    // Verify the file actually exists/readable.
    if std::fs::metadata(filename).is_err() {
        return None;
    }
    Some(crate::compiler::compile_process_create(
        filename,
        filename_out,
        flags,
    ))
}
/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    // Bridge to internal next_char via the compiler module's logic.
    let _ = lex_process;
    crate::compiler::__internal_next_char()
}
/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let _ = lex_process;
    crate::compiler::__internal_peek_char()
}
/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, c: char) {
    let _ = lex_process;
    crate::compiler::__internal_push_char(c);
}
