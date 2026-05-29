use std::fs;
use crate::compiler::{CompileProcess, CompileProcessInputFile, ClonableFile, Pos};

/// Creates a new compile_process. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    // Try open input file
    let input_file = match ClonableFile::new(filename) {
        Ok(f) => f,
        Err(_) => return None,
    };

    // Open output file (write mode)
    let out_file: Option<ClonableFile> = if !filename_out.is_empty() {
        match fs::File::create(filename_out) {
            Ok(_) => match ClonableFile::new(filename_out) {
                Ok(f) => Some(f),
                Err(_) => return None,
            },
            Err(_) => return None,
        }
    } else {
        None
    };

    let mut process = CompileProcess::default();
    process.flags = flags;
    process.cfile = CompileProcessInputFile {
        fp: Some(input_file),
        abs_path: Some(filename.to_string()),
    };
    process.ofile = out_file;
    process.pos = Pos {
        line: 1,
        col: 1,
        filename: Some(filename.to_string()),
    };
    process.node_vec = Some(crate::vector::vector_create(std::mem::size_of::<usize>()));
    process.node_tree_vec = Some(crate::vector::vector_create(std::mem::size_of::<usize>()));

    Some(process)
}

/// Read the next character. Returns 0xFF (-1 char) at EOF.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    use std::io::Read;
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return '\u{FFFF}',
    };
    compiler.pos.col += 1;
    let mut buf = [0u8; 1];
    let c: char = match compiler.cfile.fp.as_mut() {
        Some(cfile) => {
            // Need to access file directly. ClonableFile holds a File.
            match cfile.read_byte() {
                Some(b) => b as char,
                None => '\u{FFFF}',
            }
        }
        None => '\u{FFFF}',
    };
    let _ = buf;
    if c == '\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    c
}

/// Peek next char without consuming.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return '\u{FFFF}',
    };
    match compiler.cfile.fp.as_mut() {
        Some(cfile) => match cfile.peek_byte() {
            Some(b) => b as char,
            None => '\u{FFFF}',
        },
        None => '\u{FFFF}',
    }
}

/// Push (unget) character back.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, c: char) {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return,
    };
    if let Some(cfile) = compiler.cfile.fp.as_mut() {
        cfile.unget_byte(c as u8);
    }
}
