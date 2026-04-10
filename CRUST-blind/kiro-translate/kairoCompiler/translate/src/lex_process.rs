use crate::compiler::{CompileProcess, LexProcess, LexProcessFunctions, Pos};
use crate::vector::{vector_create, vector_free, Vector};

/// Creates a new LexProcess, allocating a Vector to store tokens, referencing the given CompileProcess.
pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    LexProcess {
        pos: Pos { line: 1, col: 1, filename: None },
        token_vec: Some(vector_create(std::mem::size_of::<crate::compiler::Token>())),
        compiler: Some(Box::new(compiler)),
        function: Some(functions),
        private,
        current_expression_count: 0,
        parentheses_buffer: None,
    }
}
/// Frees the lex process, including the token vector. In Rust, dropping is enough.
pub fn lex_process_free(process: LexProcess) {
    drop(process);
}
/// Returns the private data pointer (always None in this safe version).
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}
/// Returns a reference to the token vector if any.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}
