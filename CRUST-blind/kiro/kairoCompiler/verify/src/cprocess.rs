use std::fs;
use std::io::Read;
use crate::compiler::{CompileProcess, CompileProcessInputFile, Pos, ClonableFile};
use crate::vector::vector_create;

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let file = ClonableFile::new(filename).ok()?;
    let out_file = if !filename_out.is_empty() {
        Some(ClonableFile::new(filename_out).ok()?)
    } else {
        None
    };

    // Read file contents into a buffer for character-by-character access
    let contents = fs::read_to_string(filename).ok()?;

    let mut process = CompileProcess::default();
    process.node_vec = Some(vector_create(8)); // sizeof pointer
    process.node_tree_vec = Some(vector_create(8));
    process.flags = flags;
    process.cfile = CompileProcessInputFile {
        fp: Some(file),
        abs_path: Some(filename.to_string()),
    };
    process.ofile = out_file;
    process.file_contents = contents.into_bytes();
    process.file_pos = 0;
    Some(process)
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    if compiler.file_pos >= compiler.file_contents.len() {
        return 0xFF as char; // EOF
    }
    let c = compiler.file_contents[compiler.file_pos] as char;
    compiler.file_pos += 1;
    compiler.pos.col += 1;
    if c == '\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    c
}

/// Peeks the next character without consuming it, or EOF marker at end.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = lex_process.compiler.as_ref().expect("no compiler");
    if compiler.file_pos >= compiler.file_contents.len() {
        return 0xFF as char; // EOF
    }
    compiler.file_contents[compiler.file_pos] as char
}

/// Ungets a character by moving the position back by one.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    if compiler.file_pos > 0 {
        compiler.file_pos -= 1;
    }
}
