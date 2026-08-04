use crate::compiler::CompileProcess;
use crate::vector::{vector_create, Vector};
use crate::compiler::Pos;
/// Function pointer table for reading chars, peeking, and ungetting.
/// Marked Copy and Clone so we can move it freely (fixing E0507).
#[derive(Clone, Copy, Debug)]
pub struct LexProcessFunctions {
    pub next_char: fn(&mut LexProcess) -> char,
    pub peek_char: fn(&mut LexProcess) -> char,
    pub push_char: fn(&mut LexProcess, char),
}
/// The LexProcess struct, referencing a CompileProcess, token vector, etc.
#[derive(Debug, Default, Clone)]
pub struct LexProcess {
    pub pos: Pos,
    pub token_vec: Option<Vector>,
    pub compiler: Option<Box<CompileProcess>>,
    pub function: Option<LexProcessFunctions>,
    pub private: Option<()>,
    pub current_expression_count: i32,
    pub parentheses_buffer: Option<crate::buffer::Buffer>,
}
/// Creates a new LexProcess, allocating a Vector to store tokens, referencing the given CompileProcess.
pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    LexProcess {
        pos: Pos {
            line: 1,
            col: 1,
            filename: None,
        },
        token_vec: Some(vector_create(8)),
        compiler: Some(Box::new(compiler)),
        function: Some(functions),
        private,
        current_expression_count: 0,
        parentheses_buffer: None,
    }
}
/// Frees the lex process, including the token vector. In Rust, dropping is enough.
pub fn lex_process_free(_process: LexProcess) {
    // Drop is automatic.
}
/// Returns the private data pointer (always None in this safe version).
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}
/// Returns a reference to the token vector if any.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}
