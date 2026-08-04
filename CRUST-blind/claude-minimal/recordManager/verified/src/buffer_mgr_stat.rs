use crate::buffer_mgr::{
    BM_BufferPool, BM_PageHandle, ReplacementStrategy,
    get_frame_contents, get_dirty_flags, get_fix_counts,
};
use crate::dberror::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);

    print!("{{");
    print_strat(bm);
    print!(" {}}}: ", bm.num_pages);

    for i in 0..bm.num_pages as usize {
        let page = if i < frame_content.len() { frame_content[i] } else { -1 };
        let is_dirty = if i < dirty.len() { dirty[i] } else { false };
        let fc = if i < fix_count.len() { fix_count[i] } else { 0 };
        print!(
            "{}[{}{}{}]",
            if i == 0 { "" } else { "," },
            page,
            if is_dirty { "x" } else { " " },
            fc
        );
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
    let frame_content = get_frame_contents(bm);
    let dirty = get_dirty_flags(bm);
    let fix_count = get_fix_counts(bm);

    let mut message = String::new();
    for i in 0..bm.num_pages as usize {
        let page = if i < frame_content.len() { frame_content[i] } else { -1 };
        let is_dirty = if i < dirty.len() { dirty[i] } else { false };
        let fc = if i < fix_count.len() { fix_count[i] } else { 0 };
        message.push_str(&format!(
            "{}[{}{}{}]",
            if i == 0 { "" } else { "," },
            page,
            if is_dirty { "x" } else { " " },
            fc
        ));
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
    match bm.strategy {
        ReplacementStrategy::RsFifo => print!("FIFO"),
        ReplacementStrategy::RsLru => print!("LRU"),
        ReplacementStrategy::RsClock => print!("CLOCK"),
        ReplacementStrategy::RsLfu => print!("LFU"),
        ReplacementStrategy::RsLruK => print!("LRU-K"),
    }
}
