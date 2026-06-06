use crate::compiler::{CompileProcess, CompileProcessInputFile, Pos};
use crate::vector::vector_create;
use std::cell::RefCell;
use std::collections::HashMap;

/// Per-file state for character reading. Keyed by absolute path.
thread_local! {
    static FILE_STATES: RefCell<HashMap<String, FileState>> = RefCell::new(HashMap::new());
}

struct FileState {
    contents: Vec<u8>,
    position: usize,
    pushed_back: Vec<u8>,
}

fn ensure_file_state(path: &str) {
    FILE_STATES.with(|m| {
        let mut map = m.borrow_mut();
        if !map.contains_key(path) {
            let contents = std::fs::read(path).unwrap_or_default();
            map.insert(
                path.to_string(),
                FileState {
                    contents,
                    position: 0,
                    pushed_back: Vec::new(),
                },
            );
        }
    });
}

fn read_one_char_for_path(path: &str) -> char {
    let mut result: char = '\0';
    FILE_STATES.with(|m| {
        let mut map = m.borrow_mut();
        if let Some(state) = map.get_mut(path) {
            if let Some(b) = state.pushed_back.pop() {
                result = b as char;
                return;
            }
            if state.position < state.contents.len() {
                let b = state.contents[state.position];
                state.position += 1;
                result = b as char;
            } else {
                result = '\0';
            }
        }
    });
    result
}

fn push_back_char_for_path(path: &str, c: char) {
    FILE_STATES.with(|m| {
        let mut map = m.borrow_mut();
        if let Some(state) = map.get_mut(path) {
            state.pushed_back.push(c as u8);
        }
    });
}

/// Creates a new compile_process, reading in the entire input file. Returns None on failure.
pub fn compile_process_create(
    filename: &str,
    filename_out: &str,
    flags: i32,
) -> Option<CompileProcess> {
    use std::path::Path;

    if !Path::new(filename).exists() {
        return None;
    }

    let fp = match crate::compiler::ClonableFile::new(filename) {
        Ok(f) => Some(f),
        Err(_) => return None,
    };

    let ofile = if !filename_out.is_empty() {
        match std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(filename_out)
        {
            Ok(_) => crate::compiler::ClonableFile::new(filename_out).ok(),
            Err(_) => return None,
        }
    } else {
        None
    };

    // Initialize file state for reading
    ensure_file_state(filename);

    Some(CompileProcess {
        flags,
        pos: Pos::default(),
        cfile: CompileProcessInputFile {
            fp,
            abs_path: Some(filename.to_string()),
        },
        token_vec: None,
        node_vec: Some(vector_create(std::mem::size_of::<u64>())),
        node_tree_vec: Some(vector_create(std::mem::size_of::<u64>())),
        ofile,
    })
}

/// Mimics getc() by returning the next character in the process's file buffer.
pub fn compile_process_next_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let path = match lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone())
    {
        Some(p) => p,
        None => return '\0',
    };
    if let Some(c) = lex_process.compiler.as_mut() {
        c.pos.col += 1;
    }
    let ch = read_one_char_for_path(&path);
    if ch == '\n' {
        if let Some(c) = lex_process.compiler.as_mut() {
            c.pos.line += 1;
            c.pos.col = 1;
        }
    }
    ch
}

/// Peeks the next character without consuming it, or '\0' at EOF.
pub fn compile_process_peek_char(lex_process: &mut crate::lex_process::LexProcess) -> char {
    let path = match lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone())
    {
        Some(p) => p,
        None => return '\0',
    };
    let c = read_one_char_for_path(&path);
    if c != '\0' {
        push_back_char_for_path(&path, c);
    }
    c
}

/// Ungets a character by moving the position back by one, ignoring if already at start of file.
pub fn compile_process_push_char(lex_process: &mut crate::lex_process::LexProcess, c: char) {
    let path = match lex_process
        .compiler
        .as_ref()
        .and_then(|c| c.cfile.abs_path.clone())
    {
        Some(p) => p,
        None => return,
    };
    push_back_char_for_path(&path, c);
}
