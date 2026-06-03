use crate::compiler::{self, CompileProcess};
use crate::vector::Vector;
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
}

/// Creates a new LexProcess, allocating a Vector to store tokens, referencing the given CompileProcess.
pub fn lex_process_create(
    compiler: CompileProcess,
    _functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    use crate::vector::vector_create;
    LexProcess {
        pos: Pos {
            line: 1,
            col: 1,
            filename: compiler.cfile.abs_path.clone(),
        },
        token_vec: Some(vector_create(std::mem::size_of::<usize>())),
        compiler: Some(Box::new(compiler)),
        function: None,
        private,
    }
}

/// Frees the lex process, including the token vector. In Rust, dropping is enough.
pub fn lex_process_free(_process: LexProcess) {
    // Drops automatically.
}

/// Returns the private data pointer (always None in this safe version).
pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}

/// Returns a reference to the token vector if any.
pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}

// Re-export the compiler's lex helpers for convenience
pub use compiler::compile_process_next_char;
pub use compiler::compile_process_peek_char;
pub use compiler::compile_process_push_char;
