use crate::dberror::RC;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::sync::{Arc, Mutex};

pub const PAGE_SIZE: usize = 4096;

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}

pub type SM_PageHandle = String;

pub fn init_storage_manager() {
    // No-op equivalent
}

pub fn create_page_file(file_name: &str) -> RC {
    let file_result = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name);
    let mut file = match file_result {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let empty_page = vec![0u8; PAGE_SIZE];
    match file.write_all(&empty_page) {
        Ok(_) => {}
        Err(_) => return RC::WriteFailed,
    }
    let _ = file.flush();
    RC::Ok
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let file_result = OpenOptions::new().read(true).write(true).open(file_name);
    let mut file = match file_result {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let mut buf = vec![0u8; PAGE_SIZE];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    // C atoi: parse leading integer in textual form
    let total = parse_atoi_bytes(&buf);
    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(Arc::new(Mutex::new(file))));
    RC::Ok
}

fn parse_atoi_bytes(buf: &[u8]) -> i32 {
    let mut i = 0;
    while i < buf.len() && (buf[i] == b' ' || buf[i] == b'\t') {
        i += 1;
    }
    let mut sign: i32 = 1;
    if i < buf.len() && (buf[i] == b'+' || buf[i] == b'-') {
        if buf[i] == b'-' {
            sign = -1;
        }
        i += 1;
    }
    let mut v: i64 = 0;
    while i < buf.len() && buf[i].is_ascii_digit() {
        v = v * 10 + (buf[i] - b'0') as i64;
        i += 1;
    }
    (v as i32) * sign
}

fn get_file(f_handle: &SM_FileHandle) -> Option<Arc<Mutex<File>>> {
    f_handle
        .mgmt_info
        .as_ref()
        .and_then(|b| b.downcast_ref::<Arc<Mutex<File>>>().cloned())
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let file_arc = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    let mut buf = vec![0u8; PAGE_SIZE];
    let s = format!("{}", f_handle.total_num_pages);
    let bytes = s.as_bytes();
    let copy_len = bytes.len().min(PAGE_SIZE);
    buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
    {
        let mut file = file_arc.lock().unwrap();
        if file.seek(SeekFrom::Start(0)).is_err() {
            return RC::SeekFailed;
        }
        if file.write_all(&buf).is_err() {
            return RC::WriteFailed;
        }
        let _ = file.flush();
    }
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

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let file_arc = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    let offset = (page_num as u64 + 1) * PAGE_SIZE as u64;
    let mut buf = vec![0u8; PAGE_SIZE];
    {
        let mut file = file_arc.lock().unwrap();
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return RC::SeekFailed;
        }
        if file.read_exact(&mut buf).is_err() {
            return RC::ReadFailed;
        }
    }
    *mem_page = bytes_to_page_string(&buf);
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

pub fn write_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let file_arc = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileNotFound,
    };
    let offset = (page_num as u64 + 1) * PAGE_SIZE as u64;
    let bytes = page_string_to_bytes(mem_page);
    {
        let mut file = file_arc.lock().unwrap();
        // Pad to required offset if file is shorter
        let file_size = match file.seek(SeekFrom::End(0)) {
            Ok(sz) => sz,
            Err(_) => return RC::SeekFailed,
        };
        if offset > file_size {
            if page_num == f_handle.total_num_pages {
                let pad = (offset - file_size) as usize;
                let zeros = vec![0u8; pad];
                if file.write_all(&zeros).is_err() {
                    return RC::WriteFailed;
                }
                f_handle.total_num_pages += 1;
            } else {
                return RC::WriteFailed;
            }
        }
        if file.seek(SeekFrom::Start(offset)).is_err() {
            return RC::SeekFailed;
        }
        if file.write_all(&bytes).is_err() {
            return RC::WriteFailed;
        }
        let _ = file.flush();
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
    let file_arc = match get_file(f_handle) {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    let zeros = vec![0u8; PAGE_SIZE];
    {
        let mut file = file_arc.lock().unwrap();
        if file.seek(SeekFrom::End(0)).is_err() {
            return RC::SeekFailed;
        }
        if file.write_all(&zeros).is_err() {
            return RC::WriteFailed;
        }
        let _ = file.flush();
    }
    f_handle.total_num_pages += 1;
    f_handle.cur_page_pos = f_handle.total_num_pages - 1;
    RC::Ok
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    if number_of_pages <= f_handle.total_num_pages {
        return RC::Ok;
    }
    let increment = number_of_pages - f_handle.total_num_pages;
    for _ in 0..increment {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}

// Helper: convert raw bytes (u8 buffer of size PAGE_SIZE) to a String.
// We use a lossless 1:1 mapping to preserve binary data.
pub fn bytes_to_page_string(buf: &[u8]) -> String {
    // Each byte is encoded as a single char where char value == byte (0..255)
    let mut s = String::with_capacity(buf.len());
    for &b in buf {
        s.push(b as char);
    }
    s
}

// Helper: convert a page-string back to PAGE_SIZE bytes (truncating or padding as needed).
pub fn page_string_to_bytes(s: &str) -> Vec<u8> {
    let mut v = Vec::with_capacity(PAGE_SIZE);
    for c in s.chars() {
        let code = c as u32;
        if code <= 0xFF {
            v.push(code as u8);
        } else {
            // multibyte char encountered; encode as UTF-8 bytes
            let mut buf = [0u8; 4];
            let enc = c.encode_utf8(&mut buf);
            v.extend_from_slice(enc.as_bytes());
        }
        if v.len() >= PAGE_SIZE {
            v.truncate(PAGE_SIZE);
            break;
        }
    }
    while v.len() < PAGE_SIZE {
        v.push(0);
    }
    v
}
