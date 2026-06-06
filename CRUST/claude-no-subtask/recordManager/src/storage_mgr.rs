use crate::dberror::{RC, PAGE_SIZE};
use std::cell::RefCell;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}
pub type SM_PageHandle = String;

/// Wrapper for the underlying File so we can put it in `mgmt_info`.
pub(crate) struct PageFile {
    pub(crate) file: RefCell<File>,
}

pub fn init_storage_manager() {
    // No-op
}

pub fn create_page_file(file_name: &str) -> RC {
    let file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name)
    {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let empty = vec![0u8; PAGE_SIZE as usize];
    let mut file = file;
    if file.write_all(&empty).is_err() {
        return RC::WriteFailed;
    }
    if file.flush().is_err() {
        return RC::WriteFailed;
    }
    RC::Ok
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    if !Path::new(file_name).exists() {
        return RC::FileNotFound;
    }
    let file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let mut file = file;
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    // Header: ASCII number for total pages
    // Read up to first non-digit
    let mut s = String::new();
    for &b in &buf {
        if b.is_ascii_digit() {
            s.push(b as char);
        } else {
            break;
        }
    }
    let total_pages = s.parse::<i32>().unwrap_or(0);
    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(PageFile {
        file: RefCell::new(file),
    }));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let pf = match f_handle.mgmt_info.as_ref() {
        Some(b) => b,
        None => return RC::FileHandleNotInit,
    };
    let pf = match pf.downcast_ref::<PageFile>() {
        Some(pf) => pf,
        None => return RC::FileHandleNotInit,
    };
    let mut file = pf.file.borrow_mut();
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let header = format!("{}", f_handle.total_num_pages);
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let header_bytes = header.as_bytes();
    let len = std::cmp::min(header_bytes.len(), buf.len());
    buf[..len].copy_from_slice(&header_bytes[..len]);
    if file.write_all(&buf).is_err() {
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

fn read_page_internal(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let pf = match f_handle.mgmt_info.as_ref() {
        Some(b) => b,
        None => return RC::FileHandleNotInit,
    };
    let pf = match pf.downcast_ref::<PageFile>() {
        Some(pf) => pf,
        None => return RC::FileHandleNotInit,
    };
    let mut file = pf.file.borrow_mut();
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    *mem_page = String::from_utf8_lossy(&buf).to_string();
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    read_page_internal(page_num, f_handle, mem_page)
}

pub fn get_block_pos(f_handle: &SM_FileHandle) -> i32 {
    f_handle.cur_page_pos
}

pub fn read_first_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    read_block(0, f_handle, mem_page)
}

pub fn read_previous_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page = f_handle.cur_page_pos - 1;
    if page < 0 {
        return RC::ReadNonExistingPage;
    }
    read_block(page, f_handle, mem_page)
}

pub fn read_current_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page = f_handle.cur_page_pos;
    read_block(page, f_handle, mem_page)
}

pub fn read_next_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page = f_handle.cur_page_pos + 1;
    read_block(page, f_handle, mem_page)
}

pub fn read_last_block(f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    let page = f_handle.total_num_pages - 1;
    if page < 0 {
        return RC::ReadNonExistingPage;
    }
    read_block(page, f_handle, mem_page)
}

pub fn write_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    if page_num < 0 {
        return RC::WriteFailed;
    }
    let pf = match f_handle.mgmt_info.as_ref() {
        Some(b) => b,
        None => return RC::FileNotFound,
    };
    let pf = match pf.downcast_ref::<PageFile>() {
        Some(pf) => pf,
        None => return RC::FileNotFound,
    };
    let mut file = pf.file.borrow_mut();
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    let file_size = file.seek(SeekFrom::End(0)).unwrap_or(0);
    if offset > file_size {
        if page_num == f_handle.total_num_pages {
            // Pad zeros
            let pad_len = (offset - file_size) as usize;
            let pad = vec![0u8; pad_len];
            if file.seek(SeekFrom::Start(file_size)).is_err() {
                return RC::SeekFailed;
            }
            if file.write_all(&pad).is_err() {
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
    let bytes = mem_page.as_bytes();
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let len = std::cmp::min(bytes.len(), buf.len());
    buf[..len].copy_from_slice(&bytes[..len]);
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    f_handle.cur_page_pos = page_num;
    RC::Ok
}

pub fn write_current_block(f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    let page = f_handle.cur_page_pos;
    if page < 0 || page >= f_handle.total_num_pages {
        return RC::WriteFailed;
    }
    write_block(page, f_handle, mem_page)
}

pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    let pf = match f_handle.mgmt_info.as_ref() {
        Some(b) => b,
        None => return RC::FileHandleNotInit,
    };
    let pf = match pf.downcast_ref::<PageFile>() {
        Some(pf) => pf,
        None => return RC::FileHandleNotInit,
    };
    let mut file = pf.file.borrow_mut();
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let buf = vec![0u8; PAGE_SIZE as usize];
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    f_handle.total_num_pages += 1;
    f_handle.cur_page_pos = f_handle.total_num_pages - 1;
    RC::Ok
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
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
