use std::fs;
use crate::compiler::{CompileProcess, CompileProcessInputFile, ClonableFile, Pos};
use crate::vector::vector_create;

// We store input file contents and a position cursor inside CompileProcess via a side channel.
// To match C's getc/ungetc behavior, we need a per-process read cursor. Since we cannot add fields
// (signatures are fixed), we use a global registry keyed by the abs_path string.
use std::sync::Mutex;
use std::collections::HashMap;
use lazy_static::lazy_static;

#[derive(Default, Clone)]
struct FileBuffer {
    bytes: Vec<u8>,
    rindex: usize,
}

lazy_static! {
    static ref FILE_BUFFERS: Mutex<HashMap<String, FileBuffer>> = Mutex::new(HashMap::new());
}

fn buffer_key_for(process: &CompileProcess) -> Option<String> {
    process.cfile.abs_path.clone()
}

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    let bytes = fs::read(filename).ok()?;

    // For output, just verify writability if a non-empty filename was given.
    let ofile = if !filename_out.is_empty() {
        match std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename_out)
        {
            Ok(_) => ClonableFile::new(filename_out).ok(),
            Err(_) => return None,
        }
    } else {
        None
    };

    let cfile_handle = ClonableFile::new(filename).ok();

    let abs_path = std::fs::canonicalize(filename)
        .ok()
        .and_then(|p| p.to_str().map(|s| s.to_string()))
        .unwrap_or_else(|| filename.to_string());

    let mut map = FILE_BUFFERS.lock().unwrap();
    map.insert(abs_path.clone(), FileBuffer { bytes, rindex: 0 });

    Some(CompileProcess {
        flags,
        pos: Pos { line: 0, col: 0, filename: Some(abs_path.clone()) },
        cfile: CompileProcessInputFile {
            fp: cfile_handle,
            abs_path: Some(abs_path),
        },
        token_vec: None,
        node_vec: Some(vector_create(8)),
        node_tree_vec: Some(vector_create(8)),
        ofile,
    })
}

fn read_one(map: &mut HashMap<String, FileBuffer>, key: &str) -> char {
    if let Some(buf) = map.get_mut(key) {
        if buf.rindex < buf.bytes.len() {
            let c = buf.bytes[buf.rindex];
            buf.rindex += 1;
            return c as char;
        }
    }
    // EOF: in C, getc returns -1 which is then assigned to char.
    (-1i32 as u8 as char)
}

fn peek_one(map: &mut HashMap<String, FileBuffer>, key: &str) -> char {
    if let Some(buf) = map.get(key) {
        if buf.rindex < buf.bytes.len() {
            return buf.bytes[buf.rindex] as char;
        }
    }
    (-1i32 as u8 as char)
}

fn unget_one(map: &mut HashMap<String, FileBuffer>, key: &str, c: char) {
    if let Some(buf) = map.get_mut(key) {
        if buf.rindex > 0 {
            buf.rindex -= 1;
            // Match C ungetc semantics: place c into the spot
            buf.bytes[buf.rindex] = c as u8;
        }
    }
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let key = lex_process
        .compiler
        .as_ref()
        .and_then(|c| buffer_key_for(c))
        .unwrap_or_default();
    let mut map = FILE_BUFFERS.lock().unwrap();
    let c = read_one(&mut map, &key);
    drop(map);
    if let Some(compiler) = lex_process.compiler.as_mut() {
        compiler.pos.col += 1;
        if c == '\n' {
            compiler.pos.line += 1;
            compiler.pos.col = 1;
        }
    }
    c
}

/// Peeks the next character without consuming it.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let key = lex_process
        .compiler
        .as_ref()
        .and_then(|c| buffer_key_for(c))
        .unwrap_or_default();
    let mut map = FILE_BUFFERS.lock().unwrap();
    peek_one(&mut map, &key)
}

/// Ungets a character.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, c: char) {
    let key = lex_process
        .compiler
        .as_ref()
        .and_then(|c| buffer_key_for(c))
        .unwrap_or_default();
    let mut map = FILE_BUFFERS.lock().unwrap();
    unget_one(&mut map, &key, c);
}
