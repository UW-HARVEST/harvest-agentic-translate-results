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
    // Output file: create it for writing (the C code uses fopen with "w")
    let out_file = if !filename_out.is_empty() {
        // Create the file for writing, like C's fopen("w")
        match std::fs::File::create(filename_out) {
            Ok(_) => Some(ClonableFile::new(filename_out).ok()?),
            Err(_) => return None,
        }
    } else {
        None
    };

    Some(CompileProcess {
        flags,
        pos: Pos::default(),
        cfile: CompileProcessInputFile {
            fp: Some(file),
            abs_path: Some(filename.to_string()),
        },
        token_vec: None,
        node_vec: Some(vector_create(8)),
        node_tree_vec: Some(vector_create(8)),
        ofile: out_file,
    })
}
/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    compiler.pos.col += 1;
    if let Some(ref mut cf) = compiler.cfile.fp {
        let mut buf = [0u8; 1];
        match cf.file.read(&mut buf) {
            Ok(1) => {
                let c = buf[0] as char;
                if c == '\n' {
                    compiler.pos.line += 1;
                    compiler.pos.col = 1;
                }
                c
            }
            _ => '\0',
        }
    } else {
        '\0'
    }
}
/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    if let Some(ref mut cf) = compiler.cfile.fp {
        let mut buf = [0u8; 1];
        match cf.file.read(&mut buf) {
            Ok(1) => {
                // ungetc equivalent: seek back
                use std::io::Seek;
                let _ = cf.file.seek(std::io::SeekFrom::Current(-1));
                buf[0] as char
            }
            _ => '\0',
        }
    } else {
        '\0'
    }
}
/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    if let Some(ref mut cf) = compiler.cfile.fp {
        use std::io::Seek;
        let _ = cf.file.seek(std::io::SeekFrom::Current(-1));
    }
}
