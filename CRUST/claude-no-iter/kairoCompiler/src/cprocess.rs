use crate::compiler::{self, CompileProcess};
use crate::lex_process::LexProcess;

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let proc = compiler::compile_process_create(filename, filename_out, flags);
    if proc.cfile.fp.is_none() {
        None
    } else {
        Some(proc)
    }
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    let path_opt = lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone());
    let path = match path_opt {
        Some(p) => p,
        None => return '\0',
    };
    if let Some(comp) = lex_process.compiler.as_mut() {
        comp.pos.col += 1;
    }
    let c = compiler::buf_next(&path);
    if c == '\n' {
        if let Some(comp) = lex_process.compiler.as_mut() {
            comp.pos.line += 1;
            comp.pos.col = 1;
        }
    }
    c
}

/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    let path_opt = lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone());
    match path_opt {
        Some(p) => compiler::buf_peek(&p),
        None => '\0',
    }
}

/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(lex_process: &mut LexProcess, _c: char) {
    let path_opt = lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone());
    if let Some(p) = path_opt {
        compiler::buf_push(&p);
    }
}
