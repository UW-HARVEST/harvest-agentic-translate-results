use std::fs;
use std::sync::Mutex;
use lazy_static::lazy_static;
use crate::compiler::{CompileProcess, CompileProcessInputFile, ClonableFile, Pos};
use crate::lex_process::LexProcess;

lazy_static! {
    pub static ref FILE_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    pub static ref FILE_READ_INDEX: Mutex<usize> = Mutex::new(0);
}

/// Set the file content (used by compile_process_create).
pub fn set_file_buffer(content: Vec<u8>) {
    *FILE_BUFFER.lock().unwrap() = content;
    *FILE_READ_INDEX.lock().unwrap() = 0;
}

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let data = match fs::read(filename) {
        Ok(d) => d,
        Err(_) => return None,
    };

    set_file_buffer(data);

    let fp = ClonableFile::new(filename).ok();

    let ofile = if !filename_out.is_empty() {
        let _ = fs::write(filename_out, "");
        ClonableFile::new(filename_out).ok()
    } else {
        None
    };

    Some(CompileProcess {
        flags,
        pos: Pos::default(),
        cfile: CompileProcessInputFile {
            fp,
            abs_path: Some(filename.to_string()),
        },
        token_vec: None,
        node_vec: Some(crate::vector::vector_create(std::mem::size_of::<u64>())),
        node_tree_vec: Some(crate::vector::vector_create(std::mem::size_of::<u64>())),
        ofile,
    })
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    if let Some(compiler) = lex_process.compiler.as_mut() {
        compiler.pos.col += 1;
    }
    let buf = FILE_BUFFER.lock().unwrap();
    let mut idx = FILE_READ_INDEX.lock().unwrap();
    if *idx >= buf.len() {
        return '\u{FFFF}';
    }
    let c = buf[*idx] as char;
    *idx += 1;
    if c == '\n' {
        if let Some(compiler) = lex_process.compiler.as_mut() {
            compiler.pos.line += 1;
            compiler.pos.col = 1;
        }
    }
    c
}

/// Peeks the next character without consuming it.
pub fn compile_process_peek_char(_lex_process: &mut LexProcess) -> char {
    let buf = FILE_BUFFER.lock().unwrap();
    let idx = FILE_READ_INDEX.lock().unwrap();
    if *idx >= buf.len() {
        return '\u{FFFF}';
    }
    buf[*idx] as char
}

/// Ungets a character by moving the position back by one.
pub fn compile_process_push_char(_lex_process: &mut LexProcess, _c: char) {
    let mut idx = FILE_READ_INDEX.lock().unwrap();
    if *idx > 0 {
        *idx -= 1;
    }
}
