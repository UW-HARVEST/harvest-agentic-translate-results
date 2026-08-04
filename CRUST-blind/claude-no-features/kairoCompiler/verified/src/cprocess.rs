use std::fs;
use std::sync::Mutex;
use std::collections::HashMap;
use lazy_static::lazy_static;
use crate::compiler::{CompileProcess, CompileProcessInputFile, ClonableFile, Pos};

/// State for an open file: contents and read position.
pub(crate) struct FileState {
    pub bytes: Vec<u8>,
    pub pos: usize,
}

lazy_static! {
    /// Maps absolute file paths to file read state.
    pub(crate) static ref CFILE_STATE: Mutex<HashMap<String, FileState>> = Mutex::new(HashMap::new());
    /// Maps output file path to a buffer of write content.
    pub(crate) static ref OFILE_STATE: Mutex<HashMap<String, Vec<u8>>> = Mutex::new(HashMap::new());
}

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let bytes = fs::read(filename).ok()?;
    {
        let mut state = CFILE_STATE.lock().unwrap();
        state.insert(filename.to_string(), FileState { bytes, pos: 0 });
    }

    let in_file = ClonableFile::new(filename).ok()?;
    let out_file = if !filename_out.is_empty() {
        // create the output file
        let _ = fs::File::create(filename_out).ok()?;
        let mut state = OFILE_STATE.lock().unwrap();
        state.insert(filename_out.to_string(), Vec::new());
        Some(ClonableFile::new(filename_out).ok()?)
    } else {
        None
    };

    let mut cp = CompileProcess::default();
    cp.flags = flags;
    cp.cfile = CompileProcessInputFile {
        fp: Some(in_file),
        abs_path: Some(filename.to_string()),
    };
    cp.ofile = out_file;
    cp.pos = Pos {
        line: 1,
        col: 1,
        filename: Some(filename.to_string()),
    };
    cp.node_vec = Some(crate::vector::vector_create(std::mem::size_of::<usize>()));
    cp.node_tree_vec = Some(crate::vector::vector_create(std::mem::size_of::<usize>()));
    Some(cp)
}

/// Reads the next byte from the file associated with the lex_process's compiler.
fn read_next_byte(compiler: &mut CompileProcess) -> Option<u8> {
    let path = compiler.cfile.abs_path.as_ref()?.clone();
    let mut state = CFILE_STATE.lock().unwrap();
    let fs = state.get_mut(&path)?;
    if fs.pos >= fs.bytes.len() {
        return None;
    }
    let b = fs.bytes[fs.pos];
    fs.pos += 1;
    Some(b)
}

fn peek_next_byte(compiler: &CompileProcess) -> Option<u8> {
    let path = compiler.cfile.abs_path.as_ref()?.clone();
    let state = CFILE_STATE.lock().unwrap();
    let fs = state.get(&path)?;
    if fs.pos >= fs.bytes.len() {
        return None;
    }
    Some(fs.bytes[fs.pos])
}

fn unget_byte(compiler: &CompileProcess) {
    if let Some(path) = compiler.cfile.abs_path.as_ref() {
        let mut state = CFILE_STATE.lock().unwrap();
        if let Some(fs) = state.get_mut(path) {
            if fs.pos > 0 {
                fs.pos -= 1;
            }
        }
    }
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = match lex_process.compiler.as_mut() {
        Some(c) => c,
        None => return '\u{FFFF}',
    };
    compiler.pos.col += 1;
    let b = match read_next_byte(compiler) {
        Some(v) => v,
        None => return '\u{FFFF}',
    };
    if b == b'\n' {
        compiler.pos.line += 1;
        compiler.pos.col = 1;
    }
    b as char
}
/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let compiler = match lex_process.compiler.as_ref() {
        Some(c) => c,
        None => return '\u{FFFF}',
    };
    match peek_next_byte(compiler) {
        Some(b) => b as char,
        None => '\u{FFFF}',
    }
}
/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, _c: char) {
    if let Some(c) = lex_process.compiler.as_ref() {
        unget_byte(c);
    }
}
