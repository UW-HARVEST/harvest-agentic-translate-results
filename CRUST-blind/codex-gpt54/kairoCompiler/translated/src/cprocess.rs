use crate::compiler::{ClonableFile, CompileProcess, CompileProcessInputFile, Pos};
use crate::vector::vector_create;
use std::fs::OpenOptions;
use std::path::PathBuf;

pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let input = ClonableFile::new(filename).ok()?;
    let abs_path = std::fs::canonicalize(filename)
        .unwrap_or_else(|_| PathBuf::from(filename))
        .to_string_lossy()
        .into_owned();

    let ofile = if filename_out.is_empty() {
        None
    } else {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(filename_out)
            .ok()?;
        Some(ClonableFile::from_file(PathBuf::from(filename_out), file))
    };

    Some(CompileProcess {
        flags,
        pos: Pos {
            line: 0,
            col: 0,
            filename: Some(abs_path.clone()),
        },
        cfile: CompileProcessInputFile {
            fp: Some(input),
            abs_path: Some(abs_path),
        },
        token_vec: None,
        node_vec: Some(vector_create(8)),
        node_tree_vec: Some(vector_create(8)),
        ofile,
    })
}

pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return '\0';
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return '\0';
    };

    compiler.pos.col += 1;
    let c = file.next_char();
    if c == '\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    c
}

pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return '\0';
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return '\0';
    };
    file.peek_char()
}

pub fn compile_process_push_char(
    lex_process: &mut crate::lex_process::LexProcess,
    c: char,
) {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return;
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return;
    };
    file.push_char(c);
}
