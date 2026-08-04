use crate::dberror::{RC, PAGE_SIZE};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};

#[allow(non_camel_case_types)]
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
    let file = match OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(true)
        .open(file_name)
    {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };
    let empty_page = vec![0u8; PAGE_SIZE as usize];
    let mut file = file;
    match file.write_all(&empty_page) {
        Ok(_) => {}
        Err(_) => return RC::WriteFailed,
    }
    let _ = file.flush();
    RC::Ok
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    let mut file = match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(f) => f,
        Err(_) => return RC::FileNotFound,
    };

    let mut header = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut header).is_err() {
        return RC::ReadFailed;
    }

    // atoi-like parsing: find length until first non-digit/null
    let mut s = String::new();
    for &b in header.iter() {
        if b == 0 {
            break;
        }
        s.push(b as char);
    }
    let total = s.trim().parse::<i32>().unwrap_or(0);

    f_handle.file_name = file_name.to_string();
    f_handle.total_num_pages = total;
    f_handle.cur_page_pos = 0;
    f_handle.mgmt_info = Some(Box::new(file));
    RC::Ok
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    // Write the totalNumPages as a string in the header
    if let Some(any_box) = f_handle.mgmt_info.as_mut() {
        if let Some(file) = any_box.downcast_mut::<File>() {
            if file.seek(SeekFrom::Start(0)).is_err() {
                return RC::SeekFailed;
            }
            let mut header = vec![0u8; PAGE_SIZE as usize];
            let pages_str = format!("{}", f_handle.total_num_pages);
            let bytes = pages_str.as_bytes();
            header[..bytes.len()].copy_from_slice(bytes);
            // C uses sprintf which adds a null terminator
            if bytes.len() < header.len() {
                header[bytes.len()] = 0;
            }
            if file.write_all(&header).is_err() {
                return RC::WriteFailed;
            }
            let _ = file.flush();
        } else {
            return RC::FileHandleNotInit;
        }
    } else {
        return RC::FileHandleNotInit;
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
    // If the file doesn't exist, treat as success since destroy was successful in spirit?
    // C returns RC_DESTROY_FAILED if remove always fails.
    if !std::path::Path::new(file_name).exists() {
        return RC::Ok;
    }
    RC::DestroyFailed
}

pub fn read_block(page_num: i32, f_handle: &mut SM_FileHandle, mem_page: &mut SM_PageHandle) -> RC {
    if page_num < 0 || page_num >= f_handle.total_num_pages {
        return RC::ReadNonExistingPage;
    }
    let any_box = match f_handle.mgmt_info.as_mut() {
        Some(b) => b,
        None => return RC::FileHandleNotInit,
    };
    let file = match any_box.downcast_mut::<File>() {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    let offset = (page_num as u64 + 1) * PAGE_SIZE as u64;
    if file.seek(SeekFrom::Start(offset)).is_err() {
        return RC::SeekFailed;
    }
    let mut buf = vec![0u8; PAGE_SIZE as usize];
    if file.read_exact(&mut buf).is_err() {
        return RC::ReadFailed;
    }
    // Convert bytes to a String using lossy conversion preserving bytes via Latin-1 mapping.
    *mem_page = bytes_to_string(&buf);
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
    let any_box = match f_handle.mgmt_info.as_mut() {
        Some(b) => b,
        None => return RC::FileNotFound,
    };
    let file = match any_box.downcast_mut::<File>() {
        Some(f) => f,
        None => return RC::FileNotFound,
    };
    let offset = (page_num as u64 + 1) * PAGE_SIZE as u64;
    let file_size = match file.seek(SeekFrom::End(0)) {
        Ok(s) => s,
        Err(_) => return RC::SeekFailed,
    };
    if offset > file_size {
        if page_num == f_handle.total_num_pages {
            // Extend the file with zeros up to offset
            let pad_len = (offset - file_size) as usize;
            let pad = vec![0u8; pad_len];
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

    let bytes = string_to_bytes(mem_page, PAGE_SIZE as usize);
    if file.write_all(&bytes).is_err() {
        return RC::WriteFailed;
    }
    let _ = file.flush();
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
    let any_box = match f_handle.mgmt_info.as_mut() {
        Some(b) => b,
        None => return RC::FileHandleNotInit,
    };
    let file = match any_box.downcast_mut::<File>() {
        Some(f) => f,
        None => return RC::FileHandleNotInit,
    };
    if file.seek(SeekFrom::End(0)).is_err() {
        return RC::SeekFailed;
    }
    let pad = vec![0u8; PAGE_SIZE as usize];
    if file.write_all(&pad).is_err() {
        return RC::WriteFailed;
    }
    let _ = file.flush();
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
    let inc = number_of_pages - active;
    for _ in 0..inc {
        let rc = append_empty_block(f_handle);
        if rc != RC::Ok {
            return rc;
        }
    }
    RC::Ok
}

// Helpers to translate between byte buffers and Strings.
// We use Latin-1 mapping (bytes 0..=255 map to chars 0..=255) so each char is one byte.
pub fn bytes_to_string(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

pub fn string_to_bytes(s: &str, len: usize) -> Vec<u8> {
    let mut buf = Vec::with_capacity(len);
    for c in s.chars() {
        if buf.len() == len {
            break;
        }
        let c_val = c as u32;
        buf.push((c_val & 0xFF) as u8);
    }
    while buf.len() < len {
        buf.push(0);
    }
    buf
}
