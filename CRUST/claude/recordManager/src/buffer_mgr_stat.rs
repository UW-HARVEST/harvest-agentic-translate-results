use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy};
use crate::buffer_mgr::{
    get_dirty_flags, get_fix_counts, get_frame_contents,
};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    println!("{}", sprint_pool_content(bm));
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("{}", sprint_page_content(page));
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);

    let mut result = String::new();
    for i in 0..bm.num_pages as usize {
        if i != 0 {
            result.push(',');
        }
        let dirty_char = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let fc = if i < fix_count.len() { fix_count[i] } else { 0 };
        let pn = if i < frame_content.len() {
            frame_content[i]
        } else {
            -1
        };
        result.push_str(&format!("[{}{}{}]", pn, dirty_char, fc));
    }
    result
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut result = String::new();
    result.push_str(&format!("[Page {}]\n", page.page_num));
    let bytes: Vec<u8> = page.data.chars().map(|c| (c as u32) as u8).collect();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < bytes.len() { bytes[i] } else { 0 };
        result.push_str(&format!("{:02X}", b));
        if i % 8 == 0 {
            result.push(' ');
        }
        if i % 64 == 0 {
            result.push('\n');
        }
    }
    result
}

pub fn print_strat(bm: &BM_BufferPool) {
    match bm.strategy {
        ReplacementStrategy::RsFifo => print!("FIFO"),
        ReplacementStrategy::RsLru => print!("LRU"),
        ReplacementStrategy::RsClock => print!("CLOCK"),
        ReplacementStrategy::RsLfu => print!("LFU"),
        ReplacementStrategy::RsLruK => print!("LRU-K"),
    }
}
