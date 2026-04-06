use std::time::{Duration, Instant};
use roaring_bitmap::rset::RSet;

pub fn nanoseconds() -> u64 {
    let now = Instant::now();
    now.elapsed().as_nanos() as u64
}

const TIMES: usize = 2048;

macro_rules! bench {
    ($info:expr, $body:block) => {{
        let mut best_time: u64 = u64::MAX;
        for _ in 0..TIMES {
            let start = Instant::now();
            $body
            let elapsed = start.elapsed().as_nanos() as u64;
            if elapsed < best_time {
                best_time = elapsed;
            }
        }
        println!("{}: {} ns", $info, best_time);
    }};
}

pub fn main() {
    let mut set = RSet::import(&[], 4096);
    let mut set_b = RSet::import(&[], 4096);
    let mut result = RSet::import(&[], 4096);

    set.truncate();
    for i in 0..32768u16 {
        assert!(set.add(i * 2));
    }
    bench!("Contains bitset", { assert!(set.contains(10000)); });

    set.truncate();
    for i in 0..4095u16 {
        assert!(set.add(i));
    }
    bench!("Contains array", { assert!(set.contains(4000)); });

    set.truncate();
    for i in 0..32768u16 {
        assert!(set.add(i * 2));
    }
    bench!("Invert bitset", {
        set.invert(&mut result);
        assert_eq!(result.cardinality(), 32768);
    });

    set.truncate();
    for i in 0..4095u16 {
        assert!(set.add(i * 2));
    }
    bench!("Invert array", {
        set.invert(&mut result);
        assert_eq!(result.cardinality(), 61441);
    });

    set.truncate();
    set_b.truncate();
    for i in 0..4095u16 {
        assert!(set.add(i * 2));
        assert!(set_b.add(i * 3));
    }
    bench!("Intersection with arrays", {
        set.intersection(&set_b, &mut result);
        assert_eq!(result.cardinality(), 1365);
    });

    set.truncate();
    set_b.truncate();
    for i in 0..20000u16 {
        assert!(set.add(i * 2));
        assert!(set_b.add(i * 3));
    }
    bench!("Intersection with bitsets", {
        set.intersection(&set_b, &mut result);
        assert_eq!(result.cardinality(), 6667);
    });

    bench!("Fill ascending", {
        set.truncate();
        for i in 0..=65535u16 {
            assert!(set.add(i));
        }
        assert_eq!(set.cardinality(), 65536);
    });

    bench!("Fill descending", {
        set.truncate();
        for i in (0..=65535u16).rev() {
            assert!(set.add(i));
        }
        assert_eq!(set.cardinality(), 65536);
    });

    bench!("Fill optimal", {
        set.truncate();
        for i in 0..32768u16 {
            assert!(set.add(i));
        }
        for i in (32768..=65535u16).rev() {
            assert!(set.add(i));
        }
        assert_eq!(set.cardinality(), 65536);
    });
}
