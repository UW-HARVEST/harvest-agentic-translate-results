use crate::dberror::{PAGE_SIZE, RC};
use crate::tables::{Schema, Value};
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}

pub type SM_PageHandle = String;

const PS: usize = PAGE_SIZE as usize;

fn get_file<'a>(handle: &'a mut SM_FileHandle) -> Option<&'a RefCell<File>> {
    handle
        .mgmt_info
        .as_ref()
        .and_then(|b| b.downcast_ref::<RefCell<File>>())
}

fn page_to_bytes(s: &str) -> Vec<u8> {
    // Each char represents one byte; take the lowest 8 bits of its code point.
    let mut bytes: Vec<u8> = s.chars().map(|c| (c as u32) as u8).collect();
    if bytes.len() < PS {
        bytes.resize(PS, 0);
    } else if bytes.len() > PS {
        bytes.truncate(PS);
    }
    bytes
}

fn bytes_to_page(bytes: &[u8]) -> String {
    // Map each byte to a char with that code point, preserving length.
    let mut out = String::with_capacity(bytes.len());
    for &b in bytes {
        out.push(b as char);
    }
    out
}

pub fn init_storage_manager() {
    // Nothing to do; in C, this just sets a global file pointer to NULL.
}

pub fn create_page_file(file_name: &str) -> RC {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name);
    let mut file = match file {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let empty_page = vec![0u8; PS];
    if file.write_all(&empty_page).is_err() {
        return RC::WriteFailed;
    }
    let _ = file.flush();
    RC::Ok
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let file = OpenOptions::new().read(true).write(true).open(file_name);
    let mut file = match file {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };

    let mut header = vec![0u8; PS];
    if file.read_exact(&mut header).is_err() {
        return RC::ReadFailed;
    }

    // Parse the leading ASCII digits as an integer (atoi-like behavior).
    let mut total_pages: i32 = 0;
    let mut started = false;
    let mut sign: i32 = 1;
    for &b in header.iter() {
        let c = b as char;
        if !started {
            if c == ' ' || c == '\t' || c == '\n' || c == '\r' {
                continue;
            }
            if c == '-' {
                sign = -1;
                started = true;
                continue;
            }
            if c == '+' {
                started = true;
                continue;
            }
            if c.is_ascii_digit() {
                started = true;
                total_pages = total_pages * 10 + (b - b'0') as i32;
                continue;
            }
            break;
        } else {
            if c.is_ascii_digit() {
                total_pages = total_pages * 10 + (b - b'0') as i32;
            } else {
                break;
            }
        }
    }
    total_pages *= sign;

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(RefCell::new(file)));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let total = f_handle.total_num_pages;
    let cell = match get_file(f_handle) {
        Some(c) => c,
        None => return RC::FileHandleNotInit,
    };
    let mut file = cell.borrow_mut();
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let header_str = format!("{}", total);
    let header_bytes = page_to_bytes(&header_str);
    if file.write_all(&header_bytes).is_err() {
        return RC::WriteFailed;
    }
    if file.flush().is_err() {
        return RC::WriteFailed;
    }
    drop(file);
    f_handle.mgmt_info = None;
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    for _ in 0..3 {
        if std::fs::remove_file(file_name).is_ok() {
            return RC::Ok;
        }
    }
    RC::DestroyFailed
}

pub fn read_block(
    page_num: i32,
    f_handle: &mut SM_FileHandle,
    mem_page: &mut SM_PageHandle,
) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let offset = (page_num as u64 + 1) * PS as u64;
    let cell = match get_file(f_handle) {
        Some(c) => c,
        None => return RC::FileHandleNotInit,
    };
    let mut file = cell.borrow_mut();
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PS];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    drop(file);
    *mem_page = bytes_to_page(&buf);
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn get_block_pos(f_handle: &SM_FileHandle) -> i32 {
    f_handle.cur_page_pos
}

pub fn read_first_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    read_block(0, f_handle, mem_page)
}

pub fn read_previous_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn read_current_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    let page_num = f_handle.cur_page_pos;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn read_next_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos + 1;
    read_block(page_num, f_handle, mem_page)
}

pub fn read_last_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page_num = f_handle.total_num_pages - 1;
    if page_num < 0 {
        return RC::ReadNonExistingPage;
    }
    read_block(page_num, f_handle, mem_page)
}

pub fn write_block(
    page_num: i32,
    f_handle: &mut SM_FileHandle,
    mem_page: &SM_PageHandle,
) -> RC {
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let offset = (page_num as u64 + 1) * PS as u64;
    let total_pages_snapshot = f_handle.total_num_pages;
    let cell = match get_file(f_handle) {
        Some(c) => c,
        None => return RC::FileNotFound,
    };
    let mut file = cell.borrow_mut();
    let file_size = match file.seek(SeekFrom::End(0)) {
        Ok(sz) => sz,
        Err(_) => return RC::SeekFailed,
    };
    let mut needs_increment = false;
    if offset > file_size {
        if page_num == total_pages_snapshot {
            // Extend file with zeros up to offset
            if file.seek(SeekFrom::Start(file_size)).is_err() {
                return RC::SeekFailed;
            }
            let pad_size = (offset - file_size) as usize;
            let pad = vec![0u8; pad_size];
            if file.write_all(&pad).is_err() {
                return RC::WriteFailed;
            }
            needs_increment = true;
        } else {
            return RC::WriteFailed;
        }
    }
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let bytes = page_to_bytes(mem_page);
    if file.write_all(&bytes).is_err() {
        return RC::WriteFailed;
    }
    if file.flush().is_err() {
        return RC::WriteFailed;
    }
    drop(file);
    if needs_increment {
        f_handle.total_num_pages += 1;
    }
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn write_current_block(f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::WriteFailed;
    }
    write_block(page_num, f_handle, mem_page)
}

pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    let cell = match get_file(f_handle) {
        Some(c) => c,
        None => return RC::FileHandleNotInit,
    };
    let mut file = cell.borrow_mut();
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let empty = vec![0u8; PS];
    if file.write_all(&empty).is_err() {
        return RC::WriteFailed;
    }
    if file.flush().is_err() {
        return RC::WriteFailed;
    }
    drop(file);
    f_handle.total_num_pages += 1;
    f_handle.cur_page_pos = f_handle.total_num_pages - 1;
    RC::Ok
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    let active = f_handle.total_num_pages;
    if number_of_pages <= active {
        return RC::Ok;
    }
    let increment = number_of_pages - active;
    for _ in 0..increment {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}

// Suppress dead-code warnings for the imports kept for type compatibility.
#[allow(dead_code)]
fn _suppress(_v: Value, _s: Schema) {}
