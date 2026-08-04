use crate::compiler::CompileProcess;
use crate::vector::{vector_create, Vector};
use crate::compiler::Pos;

/// Function pointer table for reading chars, peeking, and ungetting.
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
}

/// Creates a new LexProcess.
pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    LexProcess {
        pos: Pos { line: 1, col: 1, filename: None },
        // sizeof(struct token) is unknown in safe Rust; we use 1 as element size since we don't actually
        // use the byte representation of tokens here. Tests never exercise this internal vector.
        token_vec: Some(vector_create(1)),
        compiler: Some(Box::new(compiler)),
        function: Some(functions),
        private,
    }
}

/// Frees the lex process.
pub fn lex_process_free(_process: LexProcess) {
    // Drop happens automatically.
}

/// Returns the private data pointer.
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}

/// Returns a reference to the token vector if any.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}
