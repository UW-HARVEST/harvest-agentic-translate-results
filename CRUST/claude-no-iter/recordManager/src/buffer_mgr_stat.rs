use crate::buffer_mgr::{
    self, BM_BufferPool, BM_PageHandle, ReplacementStrategy,
};
use crate::storage_mgr::PAGE_SIZE;

pub fn print_pool_content(bm: &BM_BufferPool) {
    print!("{{");
    print_strat(bm);
    print!(" {}}}: ", bm.num_pages);
    let frame_content = buffer_mgr::get_frame_contents(bm);
    let dirty = buffer_mgr::get_dirty_flags(bm);
    let fix_count = buffer_mgr::get_fix_counts(bm);
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        print!(
            "{}[{}{}{}]",
            prefix,
            frame_content.get(i).copied().unwrap_or(-1),
            if dirty.get(i).copied().unwrap_or(false) {
                "x"
            } else {
                " "
            },
            fix_count.get(i).copied().unwrap_or(0)
        );
    }
    println!();
}

pub fn print_page_content(page: &BM_PageHandle) {
    println!("[Page {}]", page.page_num);
    let bytes: Vec<u8> = page.data.chars().map(|c| (c as u32 & 0xFF) as u8).collect();
    for i in 1..=PAGE_SIZE {
        let b = bytes.get(i).copied().unwrap_or(0);
        let space = if i % 8 == 0 { "" } else { "" };
        let nl = if i % 64 == 0 { "\n" } else { "" };
        print!("{:02X}{}{}", b, space, nl);
    }
}

pub fn sprint_pool_content(bm: &BM_BufferPool) -> String {
    let mut message = String::new();
    let frame_content = buffer_mgr::get_frame_contents(bm);
    let dirty = buffer_mgr::get_dirty_flags(bm);
    let fix_count = buffer_mgr::get_fix_counts(bm);
    for i in 0..bm.num_pages as usize {
        let prefix = if i == 0 { "" } else { "," };
        message.push_str(&format!(
            "{}[{}{}{}]",
            prefix,
            frame_content.get(i).copied().unwrap_or(-1),
            if dirty.get(i).copied().unwrap_or(false) {
                "x"
            } else {
                " "
            },
            fix_count.get(i).copied().unwrap_or(0)
        ));
    }
    message
}

pub fn sprint_page_content(page: &BM_PageHandle) -> String {
    let mut message = String::new();
    message.push_str(&format!("[Page {}]\n", page.page_num));
    let bytes: Vec<u8> = page.data.chars().map(|c| (c as u32 & 0xFF) as u8).collect();
    for i in 1..=PAGE_SIZE {
        let b = bytes.get(i).copied().unwrap_or(0);
        let nl = if i % 64 == 0 { "\n" } else { "" };
        message.push_str(&format!("{:02X}{}", b, nl));
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
