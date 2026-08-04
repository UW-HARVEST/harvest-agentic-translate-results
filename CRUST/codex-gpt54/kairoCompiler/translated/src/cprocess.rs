use crate::compiler::{ClonableFile, CompileProcess, CompileProcessInputFile, Pos};
use crate::vector::vector_create;

fn eof_char() -> char {
    '\0'
}

pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let fp = ClonableFile::new(filename).ok()?;
    let ofile = if filename_out.is_empty() {
        None
    } else {
        ClonableFile::create(filename_out).ok()
    };

    Some(CompileProcess {
        flags,
        pos: Pos::default(),
        cfile: CompileProcessInputFile {
            fp: Some(fp),
            abs_path: Some(filename.to_string()),
        },
        token_vec: None,
        node_vec: Some(vector_create(std::mem::size_of::<u64>())),
        node_tree_vec: Some(vector_create(std::mem::size_of::<u64>())),
        ofile,
    })
}

pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return eof_char();
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return eof_char();
    };

    compiler.pos.col += 1;
    match file.read_char() {
        Ok(Some(c)) => {
            if c == '\n' {
                compiler.pos.line += 1;
                compiler.pos.col = 1;
            }
            c
        }
        _ => eof_char(),
    }
}

pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let Some(compiler) = lex_process.compiler.as_mut() else {
        return eof_char();
    };
    let Some(file) = compiler.cfile.fp.as_mut() else {
        return eof_char();
    };

    file.peek_char().ok().flatten().unwrap_or(eof_char())
}

pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    if let Some(compiler) = lex_process.compiler.as_mut() {
        if let Some(file) = compiler.cfile.fp.as_mut() {
            let _ = file.push_char();
        }
    }
}
