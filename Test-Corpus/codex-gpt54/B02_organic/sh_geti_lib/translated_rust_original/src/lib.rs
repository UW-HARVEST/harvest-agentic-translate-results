use std::collections::HashMap;
use std::ffi::c_int;
use std::io::{self, Write};

#[derive(Clone, Copy)]
enum StringStorageMode {
    Strdup,
    Arena,
}

#[derive(Default)]
struct StringArena {
    storage: Vec<String>,
}

impl StringArena {
    fn stralloc(&mut self, value: &str) -> String {
        let owned = value.to_owned();
        self.storage.push(owned.clone());
        owned
    }

    fn strreset(&mut self) {
        self.storage.clear();
    }
}

struct Entry {
    key: String,
    value: c_int,
}

struct StrMap {
    mode: StringStorageMode,
    default_value: c_int,
    entries: Vec<Entry>,
    indices: HashMap<String, usize>,
    arena: StringArena,
}

impl StrMap {
    fn new(mode: StringStorageMode) -> Self {
        Self {
            mode,
            default_value: 0,
            entries: Vec::new(),
            indices: HashMap::new(),
            arena: StringArena::default(),
        }
    }

    fn shgeti(&self, key: &str) -> c_int {
        self.indices
            .get(key)
            .map(|index| *index as c_int)
            .unwrap_or(-1)
    }

    fn shdefault(&mut self, value: c_int) {
        self.default_value = value;
    }

    fn shput(&mut self, key: &str, value: c_int) {
        if let Some(&index) = self.indices.get(key) {
            self.entries[index].value = value;
            return;
        }

        let owned_key = match self.mode {
            StringStorageMode::Strdup => key.to_owned(),
            StringStorageMode::Arena => self.arena.stralloc(key),
        };
        let index = self.entries.len();
        self.entries.push(Entry {
            key: owned_key.clone(),
            value,
        });
        self.indices.insert(owned_key, index);
    }

    fn shget(&self, key: &str) -> c_int {
        self.indices
            .get(key)
            .map(|&index| self.entries[index].value)
            .unwrap_or(self.default_value)
    }

    fn shdel(&mut self, key: &str) {
        let Some(index) = self.indices.remove(key) else {
            return;
        };

        let last_index = self.entries.len() - 1;
        self.entries.swap_remove(index);
        if index != last_index {
            let moved_key = self.entries[index].key.clone();
            self.indices.insert(moved_key, index);
        }
    }

    fn shlen(&self) -> c_int {
        self.entries.len() as c_int
    }

    fn shfree(&mut self) {
        self.entries.clear();
        self.indices.clear();
        self.arena.strreset();
    }
}

fn c_assert(condition: bool) {
    if !condition {
        std::process::abort();
    }
}

fn strkey(n: c_int) -> String {
    format!("test_{n}")
}

fn print_entry(key: &str, value: c_int) {
    let mut stdout = io::stdout().lock();
    let _ = writeln!(stdout, "{key} {value}");
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    let mut strmap: Option<StrMap> = None;
    let mut sa = StringArena::default();

    let mut i = 0;
    while i < num {
        let key = strkey(i);
        let _ = sa.stralloc(&key);
        i += 1;
    }
    sa.strreset();

    let mut j = 0;
    while j < 2 {
        c_assert(strmap.as_ref().map_or(-1, |map| map.shgeti("foo")) == -1);
        strmap = Some(StrMap::new(if j == 0 {
            StringStorageMode::Strdup
        } else {
            StringStorageMode::Arena
        }));
        c_assert(strmap.as_ref().map_or(-1, |map| map.shgeti("foo")) == -1);
        strmap.as_mut().unwrap().shdefault(-2);
        c_assert(strmap.as_ref().map_or(-1, |map| map.shgeti("foo")) == -1);

        i = 0;
        while i < num {
            let key = strkey(i);
            strmap.as_mut().unwrap().shput(&key, i * 3);
            i += 2;
        }

        let len = strmap.as_ref().unwrap().shlen();
        let mut z = 0;
        while z < len {
            let entry = &strmap.as_ref().unwrap().entries[z as usize];
            print_entry(&entry.key, entry.value);
            z += 1;
        }

        i = 0;
        while i < num {
            let key = strkey(i);
            if (i & 1) != 0 {
                c_assert(strmap.as_ref().unwrap().shget(&key) == -2);
            } else {
                c_assert(strmap.as_ref().unwrap().shget(&key) == i * 3);
            }
            i += 1;
        }

        i = 2;
        while i < num {
            let key = strkey(i);
            strmap.as_mut().unwrap().shdel(&key);
            i += 4;
        }

        i = 0;
        while i < num {
            let key = strkey(i);
            if (i & 3) != 0 {
                c_assert(strmap.as_ref().unwrap().shget(&key) == -2);
            } else {
                c_assert(strmap.as_ref().unwrap().shget(&key) == i * 3);
            }
            i += 1;
        }

        i = 0;
        while i < num {
            let key = strkey(i);
            strmap.as_mut().unwrap().shdel(&key);
            i += 1;
        }

        i = 0;
        while i < num {
            let key = strkey(i);
            c_assert(strmap.as_ref().unwrap().shget(&key) == -2);
            i += 1;
        }

        strmap.as_mut().unwrap().shfree();
        strmap = None;
        j += 1;
    }
}
