use std::io::{Read, Seek, SeekFrom};
use crate::compiler::{CompileProcess, CompileProcessInputFile, ClonableFile, Pos};
use crate::vector::vector_create;

/// Creates a new compile_process, opening the input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let cfile = match ClonableFile::new(filename) {
        Ok(f) => f,
        Err(_) => return None,
    };
    let ofile = if !filename_out.is_empty() {
        match ClonableFile::new_writable(filename_out) {
            Ok(f) => Some(f),
            Err(_) => None,
        }
    } else {
        None
    };

    Some(CompileProcess {
        flags,
        pos: Pos::default(),
        cfile: CompileProcessInputFile {
            fp: Some(cfile),
            abs_path: Some(filename.to_string()),
        },
        token_vec: None,
        // node_vec / node_tree_vec: vector of node pointers (8 bytes each).
        node_vec: Some(vector_create(std::mem::size_of::<u64>())),
        node_tree_vec: Some(vector_create(std::mem::size_of::<u64>())),
        ofile,
    })
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return '\u{FF}',
    };
    compiler.pos.col += 1;
    let c = read_one_char(compiler);
    if c == '\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    c
}
/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return '\u{FF}',
    };
    let c = read_one_char(compiler);
    if c != '\u{FF}' {
        unread_one_char(compiler);
    }
    c
}
/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    if let Some(compiler) = lex_process.compiler.as_mut() {
        unread_one_char(compiler);
    }
}

fn read_one_char(compiler: &mut CompileProcess) -> char {
    if let Some(cf) = compiler.cfile.fp.as_mut() {
        let mut buf = [0u8; 1];
        match cf.file_mut().read(&mut buf) {
            Ok(1) => buf[0] as char,
            _ => '\u{FF}',
        }
    } else {
        '\u{FF}'
    }
}

fn unread_one_char(compiler: &mut CompileProcess) {
    if let Some(cf) = compiler.cfile.fp.as_mut() {
        let _ = cf.file_mut().seek(SeekFrom::Current(-1));
    }
}
