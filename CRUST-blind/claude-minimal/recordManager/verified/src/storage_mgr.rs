use crate::dberror::{RC, PAGE_SIZE};
use std::fs::{File, OpenOptions, remove_file};
use std::io::{Read, Write, Seek, SeekFrom};

pub struct SM_FileHandle {
    pub file_name: String,
    pub total_num_pages: i32,
    pub cur_page_pos: i32,
    pub mgmt_info: Option<Box<dyn std::any::Any>>,
}
pub type SM_PageHandle = String;

pub fn init_storage_manager() {
    // No-op: in C this just sets pageFile = NULL.
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
    let empty_page = vec![0u8; PAGE_SIZE as usize];
    match file.write_all(&empty_page) {
        Ok(_) => RC::Ok,
        Err(_) => RC::WriteFailed,
    }
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(file_name);
    let mut file = match file {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    match file.read_exact(&mut buf) {
        Ok(_) => {}
        Err(_) => return RC::ReadFailed,
    }
    // Parse total_num_pages from string
    let s: String = buf.iter()
        .take_while(|&&c| c != 0)
        .map(|&c| c as char)
        .collect();
    let total_num_pages: i32 = s.trim().parse().unwrap_or(0);

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total_num_pages;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let mut file = match f_handle.mgmt_info.take() {
        Some(b) => match b.downcast::<File>() {
            Ok(f) => f,
            Err(_) => return RC::FileHandleNotInit,
        },
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::Start(0)).is_err() {
        return RC::SeekFailed;
    }
    let header_str = format!("{}", f_handle.total_num_pages);
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let bytes = header_str.as_bytes();
    buf[..bytes.len()].copy_from_slice(bytes);
    if file.write_all(&buf).is_err() {
        return RC::WriteFailed;
    }
    drop(file);
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    let mut trial = 0;
    let max_trial = 3;
    while trial < max_trial {
        if remove_file(file_name).is_ok() {
            return RC::Ok;
        }
        trial += 1;
    }
    RC::DestroyFailed
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let file = match f_handle.mgmt_info.as_mut() {
        Some(b) => match b.downcast_mut::<File>() {
            Some(f) => f,
            None => return RC::FileHandleNotInit,
        },
        None => return RC::FileHandleNotInit,
    };
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    *mem_page = String::from_utf8_lossy(&buf).into_owned();
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
    let total_num_pages = f_handle.total_num_pages;
    let file = match f_handle.mgmt_info.as_mut() {
        Some(b) => match b.downcast_mut::<File>() {
            Some(f) => f,
            None => return RC::FileNotFound,
        },
        None => return RC::FileNotFound,
    };
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    let file_size = match file.seek(SeekFrom::End(0)) {
        Ok(p) => p,
        Err(_) => return RC::SeekFailed,
    };
    if offset > file_size {
        if page_num == total_num_pages {
            // Pad with zero bytes
            let pad = vec![0u8; (offset - file_size) as usize];
            if file.write_all(&pad).is_err() {
                return RC::WriteFailed;
            }
            f_handle.total_num_pages += 1;
        } else {
            return RC::WriteFailed;
        }
    }
    let file = match f_handle.mgmt_info.as_mut() {
        Some(b) => match b.downcast_mut::<File>() {
            Some(f) => f,
            None => return RC::FileNotFound,
        },
        None => return RC::FileNotFound,
    };
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let bytes = mem_page.as_bytes();
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let copy_len = std::cmp::min(bytes.len(), PAGE_SIZE as usize);
    buf[..copy_len].copy_from_slice(&bytes[..copy_len]);
    if file.write_all(&buf).is_err() {
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
    let file = match f_handle.mgmt_info.as_mut() {
        Some(b) => match b.downcast_mut::<File>() {
            Some(f) => f,
            None => return RC::FileHandleNotInit,
        },
        None => return RC::FileHandleNotInit,
    };
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

// Suppress "unused" warning for Value/Schema imports if any
#[allow(dead_code)]
fn _unused(_v: Option<crate::tables::Value>, _s: Option<crate::tables::Schema>) {}
