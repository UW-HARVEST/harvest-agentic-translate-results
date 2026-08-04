use std::fs;
use crate::compiler::{CompileProcess, CompileProcessInputFile, Pos, ClonableFile};
use crate::vector::vector_create;

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    if !std::path::Path::new(filename).exists() {
        return None;
    }
    let input_file = match ClonableFile::new(filename) {
        Ok(f) => f,
        Err(_) => return None,
    };
    let out_file = if !filename_out.is_empty() {
        // Create the output file if it does not exist.
        if let Err(_) = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename_out)
        {
            return None;
        }
        match ClonableFile::new(filename_out) {
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
            fp: Some(input_file),
            abs_path: Some(filename.to_string()),
        },
        token_vec: None,
        node_vec: Some(vector_create(std::mem::size_of::<usize>())),
        node_tree_vec: Some(vector_create(std::mem::size_of::<usize>())),
        ofile: out_file,
    })
}
/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    use std::io::Read;
    if let Some(compiler) = lex_process.compiler.as_mut() {
        compiler.pos.col += 1;
        if let Some(cfile) = compiler.cfile.fp.as_mut() {
            let mut buf = [0u8; 1];
            match cfile.file.read(&mut buf) {
                Ok(0) => return '\u{FFFF}',
                Ok(_) => {
                    let c = buf[0] as char;
                    if c == '\n' {
                        compiler.pos.line += 1;
                        compiler.pos.col = 1;
                    }
                    return c;
                }
                Err(_) => return '\u{FFFF}',
            }
        }
    }
    '\u{FFFF}'
}
/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    use std::io::{Read, Seek, SeekFrom};
    if let Some(compiler) = lex_process.compiler.as_mut() {
        if let Some(cfile) = compiler.cfile.fp.as_mut() {
            let pos = match cfile.file.stream_position() {
                Ok(p) => p,
                Err(_) => return '\u{FFFF}',
            };
            let mut buf = [0u8; 1];
            let res = cfile.file.read(&mut buf);
            let _ = cfile.file.seek(SeekFrom::Start(pos));
            match res {
                Ok(0) => return '\u{FFFF}',
                Ok(_) => return buf[0] as char,
                Err(_) => return '\u{FFFF}',
            }
        }
    }
    '\u{FFFF}'
}
/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    use std::io::{Seek, SeekFrom};
    if let Some(compiler) = lex_process.compiler.as_mut() {
        if let Some(cfile) = compiler.cfile.fp.as_mut() {
            if let Ok(pos) = cfile.file.stream_position() {
                if pos > 0 {
                    let _ = cfile.file.seek(SeekFrom::Start(pos - 1));
                }
            }
        }
    }
}
