use crate::compiler::{CompileProcess, CompileProcessInputFile, Pos, ClonableFile, LexProcess};
use crate::vector::vector_create;
use std::io::Read;

pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let cfile = ClonableFile::new(filename).ok()?;
    let ofile = if !filename_out.is_empty() {
        Some(ClonableFile::new(filename_out).ok()?)
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
        node_vec: Some(vector_create(8)),
        node_tree_vec: Some(vector_create(8)),
        ofile,
    })
}

pub fn compile_process_next_char(lex_process: &mut LexProcess) -> char {
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    compiler.pos.col += 1;
    let c = if let Some(ref mut cf) = compiler.cfile.fp {
        let mut buf = [0u8; 1];
        match cf.file.read(&mut buf) {
            Ok(1) => buf[0] as char,
            _ => (-1i8 as u8) as char,
        }
    } else {
        (-1i8 as u8) as char
    };
    if c == '\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    c
}

pub fn compile_process_peek_char(lex_process: &mut LexProcess) -> char {
    use std::io::Seek;
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    if let Some(ref mut cf) = compiler.cfile.fp {
        let mut buf = [0u8; 1];
        match cf.file.read(&mut buf) {
            Ok(1) => {
                cf.file.seek(std::io::SeekFrom::Current(-1)).ok();
                buf[0] as char
            }
            _ => (-1i8 as u8) as char,
        }
    } else {
        (-1i8 as u8) as char
    }
}

pub fn compile_process_push_char(lex_process: &mut LexProcess, _c: char) {
    use std::io::Seek;
    let compiler = lex_process.compiler.as_mut().expect("no compiler");
    if let Some(ref mut cf) = compiler.cfile.fp {
        cf.file.seek(std::io::SeekFrom::Current(-1)).ok();
    }
}
