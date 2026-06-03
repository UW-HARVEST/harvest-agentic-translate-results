use crate::compiler::{CompileProcess, CompileProcessInputFile, ClonableFile, Pos};
use crate::vector::vector_create;
use std::path::Path;

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let in_file = ClonableFile::new(filename).ok()?;
    let _out_file = if !filename_out.is_empty() {
        // Try to create the output file path; if creation fails, return None.
        match std::fs::File::create(filename_out) {
            Ok(_) => Some(()),
            Err(_) => return None,
        }
    } else {
        None
    };

    let abs_path = Path::new(filename)
        .canonicalize()
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| filename.to_string());

    let process = CompileProcess {
        flags,
        pos: Pos::default(),
        cfile: CompileProcessInputFile {
            fp: Some(in_file),
            abs_path: Some(abs_path),
        },
        token_vec: None,
        node_vec: Some(vector_create(std::mem::size_of::<usize>())),
        node_tree_vec: Some(vector_create(std::mem::size_of::<usize>())),
        ofile: None,
    };
    Some(process)
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return (-1i32 as u8) as char,
    };
    compiler.pos.col += 1;
    let cfile = match compiler.cfile.fp.as_mut() {
        Some(f) => f,
        None => return (-1i32 as u8) as char,
    };

    let c = if let Some(b) = cfile.pushback.pop() {
        b as char
    } else if cfile.read_pos < cfile.content.len() {
        let b = cfile.content[cfile.read_pos];
        cfile.read_pos += 1;
        b as char
    } else {
        // EOF -> emulate getc returning -1 as char.
        return (-1i32 as u8) as char;
    };

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
        None => return (-1i32 as u8) as char,
    };
    let cfile = match compiler.cfile.fp.as_mut() {
        Some(f) => f,
        None => return (-1i32 as u8) as char,
    };

    if let Some(&b) = cfile.pushback.last() {
        return b as char;
    }
    if cfile.read_pos < cfile.content.len() {
        return cfile.content[cfile.read_pos] as char;
    }
    (-1i32 as u8) as char
}

/// Ungets a character by pushing it back onto the pushback stack.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, c: char) {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return,
    };
    if let Some(cfile) = compiler.cfile.fp.as_mut() {
        cfile.pushback.push(c as u8);
    }
}
