use crate::dberror::{PAGE_SIZE, RC};
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
    match File::create(file_name) {
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

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let mut file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    match file.read_exact(&mut buf) {
        Ok(_) => {}
        Err(_) => return RC::ReadFailed,
    }
    // Header is the total number of pages stored as ASCII text.
    let header_str = String::from_utf8_lossy(&buf);
    let trimmed = header_str.trim_end_matches(char::from(0));
    let total = trimmed.trim().parse::<i32>().unwrap_or(0);
    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    let total = f_handle.total_num_pages;
    let header_text = format!("{}", total);
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    let bytes = header_text.as_bytes();
    buf[..bytes.len()].copy_from_slice(bytes);
    if let Some(mgmt) = f_handle.mgmt_info.as_mut() {
        if let Some(file) = mgmt.downcast_mut::<File>() {
            if file.seek(SeekFrom::Start(0)).is_err() {
                return RC::SeekFailed;
            }
            if file.write_all(&buf).is_err() {
                return RC::WriteFailed;
            }
        }
    }
    f_handle.mgmt_info = None;
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
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if let Some(mgmt) = f_handle.mgmt_info.as_mut() {
        if let Some(file) = mgmt.downcast_mut::<File>() {
            if file.seek(SeekFrom::Start(offset)).is_err() {
                return RC::SeekFailed;
            }
            if file.read_exact(&mut buf).is_err() {
                return RC::ReadFailed;
            }
            *mem_page = String::from_utf8_lossy(&buf).into_owned();
            f_handle.cur_page_pos = page_num;
            return RC::Ok;
        }
    }
    RC::FileHandleNotInit
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
    let offset = (page_num + 1) as u64 * PAGE_SIZE as u64;
    let total_pages = f_handle.total_num_pages;
    if let Some(mgmt) = f_handle.mgmt_info.as_mut() {
        if let Some(file) = mgmt.downcast_mut::<File>() {
            // Determine current size
            let file_size = match file.seek(SeekFrom::End(0)) {
                Ok(s) => s,
                Err(_) => return RC::SeekFailed,
            };
            if offset > file_size {
                if page_num == total_pages {
                    let pad = vec![0u8; (offset - file_size) as usize];
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
            if let Some(mgmt) = f_handle.mgmt_info.as_mut() {
                if let Some(file) = mgmt.downcast_mut::<File>() {
                    if file.seek(SeekFrom::Start(offset)).is_err() {
                        return RC::SeekFailed;
                    }
                    let bytes = mem_page.as_bytes();
                    let mut buf = vec![0u8; PAGE_SIZE as usize];
                    let n = bytes.len().min(buf.len());
                    buf[..n].copy_from_slice(&bytes[..n]);
                    if file.write_all(&buf).is_err() {
                        return RC::WriteFailed;
                    }
                    f_handle.cur_page_pos = page_num;
                    return RC::Ok;
                }
            }
        }
    }
    RC::FileHandleNotInit
}

pub fn write_current_block(f_handle: &mut SM_FileHandle, mem_page: &SM_PageHandle) -> RC {
    let page_num = f_handle.cur_page_pos;
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::WriteFailed;
    }
    write_block(page_num, f_handle, mem_page)
}

pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    if f_handle.mgmt_info.is_none() {
        return RC::FileHandleNotInit;
    }
    let pad = vec![0u8; PAGE_SIZE as usize];
    if let Some(mgmt) = f_handle.mgmt_info.as_mut() {
        if let Some(file) = mgmt.downcast_mut::<File>() {
            if file.seek(SeekFrom::End(0)).is_err() {
                return RC::SeekFailed;
            }
            if file.write_all(&pad).is_err() {
                return RC::WriteFailed;
            }
        }
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
    let to_add = number_of_pages - f_handle.total_num_pages;
    for _ in 0..to_add {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}
