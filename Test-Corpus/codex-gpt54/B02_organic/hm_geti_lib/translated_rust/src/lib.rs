use std::ffi::c_int;

#[derive(Clone, Copy)]
struct Entry {
    key: c_int,
    value: c_int,
}

struct IntMap {
    entries: Vec<Entry>,
    default_value: c_int,
}

impl IntMap {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
            default_value: 0,
        }
    }

    fn hmgeti(&self, key: c_int) -> isize {
        self.entries
            .iter()
            .position(|entry| entry.key == key)
            .map_or(-1, |index| index as isize)
    }

    fn hmdefault(&mut self, value: c_int) {
        self.default_value = value;
    }

    fn hmget(&self, key: c_int) -> c_int {
        self.entries
            .iter()
            .find(|entry| entry.key == key)
            .map_or(self.default_value, |entry| entry.value)
    }

    fn hmput(&mut self, key: c_int, value: c_int) {
        if let Some(entry) = self.entries.iter_mut().find(|entry| entry.key == key) {
            entry.value = value;
            return;
        }

        self.entries.push(Entry { key, value });
    }

    fn hmdel(&mut self, key: c_int) -> c_int {
        if let Some(index) = self.entries.iter().position(|entry| entry.key == key) {
            self.entries.swap_remove(index);
            1
        } else {
            0
        }
    }
}

#[inline]
fn c_assert(condition: bool) {
    if !condition {
        std::process::abort();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    let mut intmap = IntMap::new();
    let mut i: c_int = 1;

    c_assert(intmap.hmgeti(i) == -1);
    intmap.hmdefault(-2);
    c_assert(intmap.hmgeti(i) == -1);
    c_assert(intmap.hmget(i) == -2);

    i = 0;
    while i < num {
        intmap.hmput(i, i.wrapping_mul(5));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            c_assert(intmap.hmget(i) == -2);
        } else {
            c_assert(intmap.hmget(i) == i.wrapping_mul(5));
        }

        if (i & 1) != 0 {
            c_assert(intmap.hmget(i) == -2);
        } else {
            c_assert(intmap.hmget(i) == i.wrapping_mul(5));
        }

        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        intmap.hmput(i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            c_assert(intmap.hmget(i) == -2);
        } else {
            c_assert(intmap.hmget(i) == i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 2;
    while i < num {
        intmap.hmdel(i);
        i = i.wrapping_add(4);
    }

    i = 0;
    while i < num {
        if (i & 3) != 0 {
            c_assert(intmap.hmget(i) == -2);
        } else {
            c_assert(intmap.hmget(i) == i.wrapping_mul(3));
        }
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        intmap.hmdel(i);
        i = i.wrapping_add(1);
    }

    i = 0;
    while i < num {
        c_assert(intmap.hmget(i) == -2);
        i = i.wrapping_add(1);
    }

    intmap = IntMap::new();

    i = 0;
    while i < num {
        intmap.hmput(i, i.wrapping_mul(3));
        i = i.wrapping_add(2);
    }

    let _ = intmap;
}
