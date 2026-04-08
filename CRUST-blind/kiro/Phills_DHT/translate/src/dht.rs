pub mod dht {
    pub struct HASHTABLE<T> {
        pub hash_table: Vec<Option<T>>, 
        pub lower_bound: i32, 
        pub higher_bound: i32,
    }
    impl <T> HASHTABLE<T> {
        pub fn dht_init() -> HASHTABLE<T> {
            HASHTABLE {
                hash_table: Vec::new(),
                lower_bound: 0,
                higher_bound: 0,
            }
        }
        pub fn dht_is_initialised(&self) -> bool {
            !self.hash_table.is_empty()
        }
        pub fn dht_init_table(&mut self, new_lower_bound: u32, new_upper_bound: u32, migrate: bool) -> bool {
            assert!(new_upper_bound > new_lower_bound);

            let new_size = (new_upper_bound - new_lower_bound) as usize;
            let mut new_table: Vec<Option<T>> = (0..new_size).map(|_| None).collect();

            if self.dht_is_initialised() {
                let old_lb = self.lower_bound as u32;
                let old_table: Vec<Option<T>> = self.hash_table.drain(..).collect();
                for (i, item) in old_table.into_iter().enumerate() {
                    if item.is_some() {
                        let abs_pos = i as u32 + old_lb;
                        if migrate && abs_pos >= new_lower_bound && abs_pos < new_upper_bound {
                            new_table[(abs_pos - new_lower_bound) as usize] = item;
                        }
                    }
                }
            }

            self.hash_table = new_table;
            self.lower_bound = new_lower_bound as i32;
            self.higher_bound = new_upper_bound as i32;
            true
        }
        pub fn dht_get_lower_bound(&self) -> u32 {
            assert!(self.dht_is_initialised());
            self.lower_bound as u32
        }
        pub fn dht_get_upper_bound(&self) -> u32 {
            assert!(self.dht_is_initialised());
            self.higher_bound as u32
        }
        pub fn dht_get_size(&self) -> u32 {
            assert!(self.dht_is_initialised());
            (self.higher_bound - self.lower_bound) as u32
        }
        pub fn dht_read(&self, location: u32) -> &Option<T> {
            assert!(self.dht_is_initialised());
            assert!(location >= self.lower_bound as u32);
            assert!(location < self.higher_bound as u32);
            let idx = (location - self.lower_bound as u32) as usize;
            &self.hash_table[idx]
        }
        pub fn dht_write(&mut self, location: u32, data: T) {
            assert!(self.dht_is_initialised());
            assert!(location >= self.lower_bound as u32);
            assert!(location < self.higher_bound as u32);
            let idx = (location - self.lower_bound as u32) as usize;
            self.hash_table[idx] = Some(data);
        }
    }
}
