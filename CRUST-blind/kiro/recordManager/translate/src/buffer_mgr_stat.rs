use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy,
    get_frame_contents, get_dirty_flags, get_fix_counts};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    let s = sprint_pool_content(bm);
    print_strat(bm);
    print!(" {}}}: ", bm.num_pages);
    println!("{}", s);
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("[Page {}]", page.page_num);
    let bytes = page.data.as_bytes();
    for i in 1..=PAGE_SIZE as usize {
        if i < bytes.len() {
            print!("{:02X}", bytes[i]);
        } else {
            print!("00");
        }
        if i % 8 == 0 { print!(" "); }
        if i % 64 == 0 { println!(); }
    }
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);
    let mut result = String::new();
    for i in 0..bm.num_pages as usize {
        if i > 0 { result.push(','); }
        let fc = if i < frame_content.len() { frame_content[i] } else { -1 };
        let d = if i < dirty.len() { dirty[i] } else { false };
        let fx = if i < fix_count.len() { fix_count[i] } else { 0 };
        result.push_str(&format!("[{}{}{}]", fc, if d { "x" } else { " " }, fx));
    }
    result
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut result = format!("[Page {}]\n", page.page_num);
    let bytes = page.data.as_bytes();
    for i in 1..=PAGE_SIZE as usize {
        if i < bytes.len() {
            result.push_str(&format!("{:02X}", bytes[i]));
        } else {
            result.push_str("00");
        }
        if i % 8 == 0 { result.push(' '); }
        if i % 64 == 0 { result.push('\n'); }
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
