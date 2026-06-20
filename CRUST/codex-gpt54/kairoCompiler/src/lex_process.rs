use crate::compiler::CompileProcess;
use crate::compiler::Pos;
use crate::compiler::Token;
use crate::vector::{vector_create, Vector};

#[derive(Clone, Copy, Debug)]
pub struct LexProcessFunctions {
    pub next_char: fn(&mut LexProcess) -> char,
    pub peek_char: fn(&mut LexProcess) -> char,
    pub push_char: fn(&mut LexProcess, char),
}

#[derive(Debug, Default, Clone)]
pub struct LexProcess {
    pub pos: Pos,
    pub token_vec: Option<Vector>,
    pub compiler: Option<Box<CompileProcess>>,
    pub function: Option<LexProcessFunctions>,
    pub private: Option<()>,
}

pub fn lex_process_create(
    compiler: CompileProcess,
    functions: LexProcessFunctions,
    private: Option<()>,
) -> LexProcess {
    LexProcess {
        pos: Pos {
            line: 1,
            col: 1,
            ..Pos::default()
        },
        token_vec: Some(vector_create(std::mem::size_of::<Token>())),
        compiler: Some(Box::new(compiler)),
        function: Some(functions),
        private,
    }
}

pub fn lex_process_free(_process: LexProcess) {}

pub fn lex_process_private(process: &LexProcess) -> Option<()> {
    process.private
}

pub fn lex_process_tokens(process: &LexProcess) -> Option<&Vector> {
    process.token_vec.as_ref()
}
