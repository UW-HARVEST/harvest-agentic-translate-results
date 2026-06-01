use std::sync::Mutex;
use lazy_static::lazy_static;

use crate::compiler::CompileProcess;

/// Global file-read state. The lexer reads bytes from this buffer using
/// `compile_process_next_char` / `_peek_char` / `_push_char`.
lazy_static! {
    pub static ref FILE_BUF: Mutex<FileBuf> = Mutex::new(FileBuf::default());
}

#[derive(Default, Debug)]
pub struct FileBuf {
    pub data: Vec<u8>,
    pub pos: usize,
}

pub fn set_file_content(content: Vec<u8>) {
    let mut fb = FILE_BUF.lock().unwrap();
    fb.data = content;
    fb.pos = 0;
}

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    // Read the input file fully into the global file buffer.
    let bytes = match std::fs::read(filename) {
        Ok(b) => b,
        Err(_) => return None,
    };
    set_file_content(bytes);

    // If an output filename was provided, attempt to create it.
    if !filename_out.is_empty() {
        if std::fs::File::create(filename_out).is_err() {
            return None;
        }
    }

    let mut process = CompileProcess::default();
    process.flags = flags;
    process.cfile.abs_path = Some(filename.to_string());
    process.pos.line = 1;
    process.pos.col = 1;
    Some(process)
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    if let Some(compiler) = lex_process.compiler.as_mut() {
        compiler.pos.col += 1;
    }
    let mut fb = FILE_BUF.lock().unwrap();
    if fb.pos >= fb.data.len() {
        return '\0';
    }
    let c = fb.data[fb.pos] as char;
    fb.pos += 1;
    if c == '\n' {
        if let Some(compiler) = lex_process.compiler.as_mut() {
            compiler.pos.line += 1;
            compiler.pos.col = 1;
        }
    }
    c
}

/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(_lex_process: &mut crate::lex_process::LexProcess) -> char {
    let fb = FILE_BUF.lock().unwrap();
    if fb.pos >= fb.data.len() {
        return '\0';
    }
    fb.data[fb.pos] as char
}

/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(_lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    let mut fb = FILE_BUF.lock().unwrap();
    if fb.pos > 0 {
        fb.pos -= 1;
    }
}
