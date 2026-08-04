use std::fs;
use std::io::Write;
use std::sync::Mutex;
use crate::compiler::{CompileProcess, CompileProcessInputFile, ClonableFile, Pos};
use crate::vector::vector_create;

use lazy_static::lazy_static;

lazy_static! {
    pub static ref INPUT_BUFFER: Mutex<Vec<u8>> = Mutex::new(Vec::new());
    pub static ref INPUT_POS: Mutex<usize> = Mutex::new(0);
}

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    // Open input file. Read all contents into the global input buffer.
    let contents = match fs::read(filename) {
        Ok(c) => c,
        Err(_) => return None,
    };
    {
        let mut buf = INPUT_BUFFER.lock().unwrap();
        *buf = contents;
        let mut pos = INPUT_POS.lock().unwrap();
        *pos = 0;
    }

    // Create output file.
    let abs_path_out = if !filename_out.is_empty() {
        match fs::File::create(filename_out) {
            Ok(mut f) => {
                let _ = f.write_all(b"");
                Some(filename_out.to_string())
            }
            Err(_) => return None,
        }
    } else {
        None
    };

    let cfile = CompileProcessInputFile {
        fp: ClonableFile::new(filename).ok(),
        abs_path: Some(filename.to_string()),
    };

    let ofile = if let Some(p) = abs_path_out.as_ref() {
        ClonableFile::new(p).ok()
    } else {
        None
    };

    let process = CompileProcess {
        flags,
        pos: Pos {
            line: 1,
            col: 1,
            filename: Some(filename.to_string()),
        },
        cfile,
        token_vec: None,
        node_vec: Some(vector_create(8)),
        node_tree_vec: Some(vector_create(8)),
        ofile,
    };
    Some(process)
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let buf = INPUT_BUFFER.lock().unwrap();
    let mut pos = INPUT_POS.lock().unwrap();
    if let Some(compiler) = lex_process.compiler.as_mut() {
        compiler.pos.col += 1;
    }
    if *pos >= buf.len() {
        return '\u{FF}'; // EOF
    }
    let c = buf[*pos];
    *pos += 1;
    if c == b'\n' {
        if let Some(compiler) = lex_process.compiler.as_mut() {
            compiler.pos.line += 1;
            compiler.pos.col = 1;
        }
    }
    c as char
}

/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(_lex_process: &mut crate::lex_process::LexProcess) -> char {
    let buf = INPUT_BUFFER.lock().unwrap();
    let pos = INPUT_POS.lock().unwrap();
    if *pos >= buf.len() {
        return '\u{FF}';
    }
    buf[*pos] as char
}

/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(_lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    let mut pos = INPUT_POS.lock().unwrap();
    if *pos > 0 {
        *pos -= 1;
    }
}
