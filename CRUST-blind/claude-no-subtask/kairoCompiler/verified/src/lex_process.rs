use crate::compiler::CompileProcess;
use crate::vector::{vector_create, Vector};
use crate::compiler::Pos;
use crate::buffer::Buffer;

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
    // Additional fields needed by the lexer.
    pub current_expression_count: i32,
    pub parentheses_buffer: Option<Buffer>,
    // Private string buffer used when lexing a string source.
    pub private_buffer: Option<Buffer>,
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
        // Tokens are stored externally in a global storage; the vector holds u64 indices.
        token_vec: Some(vector_create(std::mem::size_of::<u64>())),
        compiler: Some(Box::new(compiler)),
        function: Some(functions),
        private,
        current_expression_count: 0,
        parentheses_buffer: None,
        private_buffer: None,
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
