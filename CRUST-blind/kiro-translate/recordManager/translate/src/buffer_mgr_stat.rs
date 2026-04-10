use crate::buffer_mgr::{BM_BufferPool, BM_PageHandle, ReplacementStrategy,
    get_frame_contents, get_dirty_flags, get_fix_counts};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    let frame = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix = get_fix_counts(bm);

    print!("{{");
    print_strat(bm);
    print!(" {}}}: ", bm.num_pages);

    for i in 0..bm.num_pages as usize {
        let f = if i < frame.len() { frame[i] } else { -1 };
        let d = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let fc = if i < fix.len() { fix[i] } else { 0 };
        if i == 0 {
            print!("[{}{}{}]", f, d, fc);
        } else {
            print!(",[{}{}{}]", f, d, fc);
        }
    }
    println!();
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("[Page {}]", page.page_num);
    let chars: Vec<char> = page.data.chars().collect();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < chars.len() { chars[i] as u8 } else { 0 };
        print!("{:02X}", b);
        if i % 8 == 0 { print!(" "); }
        if i % 64 == 0 { println!(); }
    }
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let frame = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix = get_fix_counts(bm);
    let mut result = String::new();

    for i in 0..bm.num_pages as usize {
        let f = if i < frame.len() { frame[i] } else { -1 };
        let d = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let fc = if i < fix.len() { fix[i] } else { 0 };
        if i == 0 {
            result.push_str(&format!("[{}{}{}]", f, d, fc));
        } else {
            result.push_str(&format!(",[{}{}{}]", f, d, fc));
        }
    }
    result
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut result = format!("[Page {}]\n", page.page_num);
    let chars: Vec<char> = page.data.chars().collect();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < chars.len() { chars[i] as u8 } else { 0 };
        result.push_str(&format!("{:02X}", b));
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
