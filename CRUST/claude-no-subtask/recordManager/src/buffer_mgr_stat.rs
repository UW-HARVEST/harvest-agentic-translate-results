use crate::buffer_mgr::{
    self, BM_BufferPool, BM_PageHandle, ReplacementStrategy,
};
use crate::dberror::PAGE_SIZE;

fn print_strat_to_string(bm: &BM_BufferPool) -> String {
    match bm.strategy {
        ReplacementStrategy::RsFifo => "FIFO".to_string(),
        ReplacementStrategy::RsLru => "LRU".to_string(),
        ReplacementStrategy::RsClock => "CLOCK".to_string(),
        ReplacementStrategy::RsLfu => "LFU".to_string(),
        ReplacementStrategy::RsLruK => "LRU-K".to_string(),
    }
}

pub fn print_pool_content(bm: &BM_BufferPool) {
    let frame_content = buffer_mgr::get_frame_contents(bm);
    let dirty = buffer_mgr::get_dirty_flags(bm);
    let fix_count = buffer_mgr::get_fix_counts(bm);
    print!("{{");
    print!("{}", print_strat_to_string(bm));
    print!(" {}}}: ", bm.num_pages);
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        let dirty_marker = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let fc = if i < fix_count.len() { fix_count[i] } else { 0 };
        let fc_pgnum = if i < frame_content.len() {
            frame_content[i]
        } else {
            -1
        };
        print!("{}[{}{}{}]", prefix, fc_pgnum, dirty_marker, fc);
    }
    println!();
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("[Page {}]", page.page_num);
    let bytes = page.data.as_bytes();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < bytes.len() { bytes[i] } else { 0 };
        print!("{:02X}", b);
        if i % 8 == 0 {
            print!(" ");
        }
        if i % 64 == 0 {
            println!();
        }
    }
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame_content = buffer_mgr::get_frame_contents(bm);
    let dirty = buffer_mgr::get_dirty_flags(bm);
    let fix_count = buffer_mgr::get_fix_counts(bm);
    let mut message = String::new();
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        let dirty_marker = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let fc = if i < fix_count.len() { fix_count[i] } else { 0 };
        let fc_pgnum = if i < frame_content.len() {
            frame_content[i]
        } else {
            -1
        };
        message.push_str(&format!("{}[{}{}{}]", prefix, fc_pgnum, dirty_marker, fc));
    }
    message
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut message = String::new();
    message.push_str(&format!("[Page {}]\n", page.page_num));
    let bytes = page.data.as_bytes();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < bytes.len() { bytes[i] } else { 0 };
        message.push_str(&format!("{:02X}", b));
        if i % 8 == 0 {
            message.push(' ');
        }
        if i % 64 == 0 {
            message.push('\n');
        }
    }
    message
}

pub fn print_strat(bm: &BM_BufferPool) {
    print!("{}", print_strat_to_string(bm));
}
