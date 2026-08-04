use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy,
                        get_frame_contents, get_dirty_flags, get_fix_counts};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    print!("{}", sprint_pool_content(bm));
    println!();
}

pub fn print_page_content(page: &BM_PageHandle) {
    print!("{}", sprint_page_content(page));
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix = get_fix_counts(bm);
    let mut result = String::new();
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        let dchar = if dirty.get(i).copied().unwrap_or(false) { "x" } else { " " };
        let fc = fix.get(i).copied().unwrap_or(0);
        let fr = frame.get(i).copied().unwrap_or(-1);
        result.push_str(&format!("{}[{}{}{}]", prefix, fr, dchar, fc));
    }
    result
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut result = format!("[Page {}]\n", page.page_num);
    let bytes: Vec<u8> = page.data.chars().map(|c| c as u8).collect();
    for i in 1..=PAGE_SIZE as usize {
        let b = bytes.get(i).copied().unwrap_or(0);
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
    let s = match bm.strategy {
        ReplacementStrategy::RsFifo => "FIFO".to_string(),
        ReplacementStrategy::RsLru => "LRU".to_string(),
        ReplacementStrategy::RsClock => "CLOCK".to_string(),
        ReplacementStrategy::RsLfu => "LFU".to_string(),
        ReplacementStrategy::RsLruK => "LRU-K".to_string(),
    };
    print!("{}", s);
}
