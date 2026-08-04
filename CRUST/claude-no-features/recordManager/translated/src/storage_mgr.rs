use crate::dberror::{RC, PAGE_SIZE};
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

pub fn init_storage_manager() {
    // No-op
}

pub fn create_page_file(file_name: &str) -> RC {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_name);
    match file {
        Ok(mut f) => {
            let empty = vec![0u8; PAGE_SIZE as usize];
            match f.write_all(&empty) {
                Ok(_) => RC::Ok,
                Err(_) => RC::WriteFailed,
            }
        }
        Err(_) => RC::FileNotFound,
    }
}

struct FileHandleData {
    file: RefCell<File>,
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let file = OpenOptions::new().read(true).write(true).open(file_name);
    match file {
        Ok(mut f) => {
            let mut buf = vec![0u8; PAGE_SIZE as usize];
            match f.read_exact(&mut buf) {
                Ok(_) => {
                    // parse total_num_pages from string
                    let mut s = String::new();
                    for &b in &buf {
                        if b == 0 {
                            break;
                        }
                        s.push(b as char);
                    }
                    let total = s.trim().parse::<i32>().unwrap_or(0);
                    f_handle.file_name = file_name.to_string();
                    f_handle.total_num_pages = total;
                    f_handle.cur_page_pos = 0;
                    f_handle.mgmt_info = Some(Box::new(FileHandleData {
                        file: RefCell::new(f),
                    }));
                    RC::Ok
                }
                Err(_) => RC::ReadFailed,
            }
        }
        Err(_) => RC::FileNotFound,
    }
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    if let Some(mgmt) = f_handle.mgmt_info.take() {
        if let Ok(data) = mgmt.downcast::<FileHandleData>() {
            let mut file = data.file.borrow_mut();
            if file.seek(SeekFrom::Start(0)).is_err() {
                return RC::SeekFailed;
            }
            let header = format!("{}", f_handle.total_num_pages);
            let mut buf = vec![0u8; PAGE_SIZE as usize];
            for (i, b) in header.as_bytes().iter().enumerate() {
                if i >= buf.len() {
                    break;
                }
                buf[i] = *b;
            }
            if file.write_all(&buf).is_err() {
                return RC::WriteFailed;
            }
        }
    }
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    match std::fs::remove_file(file_name) {
        Ok(_) => RC::Ok,
        Err(_) => RC::DestroyFailed,
    }
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let mgmt = match &f_handle.mgmt_info {
        Some(m) => m,
        None => return RC::FileHandleNotInit,
    };
    let data = match mgmt.downcast_ref::<FileHandleData>() {
        Some(d) => d,
        None => return RC::FileHandleNotInit,
    };
    let mut file = data.file.borrow_mut();
    let offset = ((page_num + 1) as u64) * (PAGE_SIZE as u64);
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    *mem_page = buf.iter().map(|&b| b as char).collect();
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
    let page_num = f_handle.cur_page_pos;
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
    let mgmt = match &f_handle.mgmt_info {
        Some(m) => m,
        None => return RC::FileNotFound,
    };
    let data = match mgmt.downcast_ref::<FileHandleData>() {
        Some(d) => d,
        None => return RC::FileNotFound,
    };
    let mut file = data.file.borrow_mut();
    let offset = ((page_num + 1) as u64) * (PAGE_SIZE as u64);
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let file_size = file.stream_position().unwrap_or(0);
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
    let bytes: Vec<u8> = mem_page.chars().take(PAGE_SIZE as usize).map(|c| c as u8).collect();
    let mut padded = bytes;
    if padded.len() < PAGE_SIZE as usize {
        padded.resize(PAGE_SIZE as usize, 0);
    }
    if file.write_all(&padded).is_err() {
        return RC::WriteFailed;
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
    let mgmt = match &f_handle.mgmt_info {
        Some(m) => m,
        None => return RC::FileHandleNotInit,
    };
    let data = match mgmt.downcast_ref::<FileHandleData>() {
        Some(d) => d,
        None => return RC::FileHandleNotInit,
    };
    let mut file = data.file.borrow_mut();
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let zeros = vec![0u8; PAGE_SIZE as usize];
    if file.write_all(&zeros).is_err() {
        return RC::WriteFailed;
    }
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
    let needed = number_of_pages - active;
    for _ in 0..needed {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}
