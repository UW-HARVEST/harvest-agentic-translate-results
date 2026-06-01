use crate::dberror::RC;

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
    use std::fs::File;
    use std::io::Write;
    let page_size = crate::dberror::PAGE_SIZE as usize;
    match File::create(file_name) {
        Ok(mut file) => {
            let buf = vec![0u8; page_size];
            match file.write_all(&buf) {
                Ok(_) => RC::Ok,
                Err(_) => RC::WriteFailed,
            }
        }
        Err(_) => RC::FileNotFound,
    }
}

pub fn open_page_file(file_name: &str, f_handle: &mut SM_FileHandle) -> RC {
    use std::fs::OpenOptions;
    use std::io::Read;
    let page_size = crate::dberror::PAGE_SIZE as usize;
    match OpenOptions::new().read(true).write(true).open(file_name) {
        Ok(mut file) => {
            let mut buf = vec![0u8; page_size];
            if file.read_exact(&mut buf).is_err() {
                return RC::ReadFailed;
            }
            f_handle.file_name = file_name.to_string();
            // total_num_pages from atoi-style parsing
            let mut s = String::new();
            for &b in &buf {
                if b == 0 { break; }
                s.push(b as char);
            }
            f_handle.total_num_pages = s.parse().unwrap_or(0);
            f_handle.cur_page_pos = 0;
            f_handle.mgmt_info = Some(Box::new(file));
            RC::Ok
        }
        Err(_) => RC::FileNotFound,
    }
}

pub fn close_page_file(f_handle: &mut SM_FileHandle) -> RC {
    f_handle.mgmt_info = None;
    RC::Ok
}

pub fn destroy_page_file(file_name: &str) -> RC {
    match std::fs::remove_file(file_name) {
        Ok(_) => RC::Ok,
        Err(_) => RC::DestroyFailed,
    }
}

pub fn read_block(_page_num: i32, _f_handle: &mut SM_FileHandle, _mem_page: &mut SM_PageHandle) -> RC {
    RC::Ok
}

pub fn get_block_pos(f_handle: &SM_FileHandle) -> i32 {
    f_handle.cur_page_pos
}

pub fn read_first_block(_f_handle: &mut SM_FileHandle, _mem_page: &mut SM_PageHandle) -> RC {
    RC::Ok
}

pub fn read_previous_block(_f_handle: &mut SM_FileHandle, _mem_page: &mut SM_PageHandle) -> RC {
    RC::Ok
}

pub fn read_current_block(_f_handle: &mut SM_FileHandle, _mem_page: &mut SM_PageHandle) -> RC {
    RC::Ok
}

pub fn read_next_block(_f_handle: &mut SM_FileHandle, _mem_page: &mut SM_PageHandle) -> RC {
    RC::Ok
}

pub fn read_last_block(_f_handle: &mut SM_FileHandle, _mem_page: &mut SM_PageHandle) -> RC {
    RC::Ok
}

pub fn write_block(_page_num: i32, _f_handle: &mut SM_FileHandle, _mem_page: &SM_PageHandle) -> RC {
    RC::Ok
}

pub fn write_current_block(_f_handle: &mut SM_FileHandle, _mem_page: &SM_PageHandle) -> RC {
    RC::Ok
}

pub fn append_empty_block(f_handle: &mut SM_FileHandle) -> RC {
    f_handle.total_num_pages += 1;
    f_handle.cur_page_pos = f_handle.total_num_pages - 1;
    RC::Ok
}

pub fn ensure_capacity(number_of_pages: i32, f_handle: &mut SM_FileHandle) -> RC {
    if number_of_pages <= f_handle.total_num_pages {
        return RC::Ok;
    }
    let to_add = number_of_pages - f_handle.total_num_pages;
    for _ in 0..to_add {
        let rc = append_empty_block(f_handle);
        if !matches!(rc, RC::Ok) {
            return rc;
        }
    }
    RC::Ok
}
