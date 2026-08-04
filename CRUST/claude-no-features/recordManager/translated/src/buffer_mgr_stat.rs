use crate::buffer_mgr::{self, BM_BufferPool, BM_PageHandle, ReplacementStrategy};
use crate::dberror::PAGE_SIZE;

fn strat_str(s: &ReplacementStrategy) -> &'static str {
    match s {
        ReplacementStrategy::RsFifo => "FIFO",
        ReplacementStrategy::RsLru => "LRU",
        ReplacementStrategy::RsClock => "CLOCK",
        ReplacementStrategy::RsLfu => "LFU",
        ReplacementStrategy::RsLruK => "LRU-K",
    }
}

pub fn print_pool_content(bm: &BM_BufferPool) {
    let frame = buffer_mgr::get_frame_contents(bm);
    let dirty = buffer_mgr::get_dirty_flags(bm);
    let fix = buffer_mgr::get_fix_counts(bm);
    print!("{{");
    print_strat(bm);
    print!(" {}}}: ", bm.num_pages);
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        let d = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let fc = if i < fix.len() { fix[i] } else { 0 };
        let fr = if i < frame.len() { frame[i] } else { 0 };
        print!("{}[{}{}{}]", prefix, fr, d, fc);
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
    let frame = buffer_mgr::get_frame_contents(bm);
    let dirty = buffer_mgr::get_dirty_flags(bm);
    let fix = buffer_mgr::get_fix_counts(bm);
    let mut s = String::new();
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        let d = if i < dirty.len() && dirty[i] { "x" } else { " " };
        let fc = if i < fix.len() { fix[i] } else { 0 };
        let fr = if i < frame.len() { frame[i] } else { 0 };
        s.push_str(&format!("{}[{}{}{}]", prefix, fr, d, fc));
    }
    s
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut s = String::new();
    s.push_str(&format!("[Page {}]\n", page.page_num));
    let bytes = page.data.as_bytes();
    for i in 1..=PAGE_SIZE as usize {
        let b = if i < bytes.len() { bytes[i] } else { 0 };
        s.push_str(&format!("{:02X}", b));
        if i % 8 == 0 {
            s.push(' ');
        }
        if i % 64 == 0 {
            s.push('\n');
        }
    }
    s
}

pub fn print_strat(bm: &BM_BufferPool) {
    print!("{}", strat_str(&bm.strategy));
}
