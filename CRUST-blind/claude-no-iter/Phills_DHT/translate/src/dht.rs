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

            let new_hash_table_size = (new_upper_bound - new_lower_bound) as usize;

            // Allocate new hash table, initialised with None.
            let mut new_hash_table: Vec<Option<T>> = Vec::with_capacity(new_hash_table_size);
            for _ in 0..new_hash_table_size {
                new_hash_table.push(None);
            }

            // If the table was previously initialised, possibly migrate data.
            if self.dht_is_initialised() {
                let old_lower_bound = self.lower_bound as u32;
                let old_hash_table_size = self.hash_table.len();

                for i in 0..old_hash_table_size {
                    // Take ownership of the item (replacing slot with None).
                    if let Some(item) = self.hash_table[i].take() {
                        let absolute_position = (i as u32).wrapping_add(old_lower_bound);

                        if migrate
                            && absolute_position >= new_lower_bound
                            && absolute_position < new_upper_bound
                        {
                            let idx = (absolute_position - new_lower_bound) as usize;
                            new_hash_table[idx] = Some(item);
                        }
                        // Otherwise the item is dropped (mirrors the C version which
                        // would normally hand the item off to a peer node).
                    }
                }
            }

            // Install the new table.
            self.hash_table = new_hash_table;
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
            let lower = self.lower_bound as u32;
            let upper = self.higher_bound as u32;
            assert!(location >= lower);
            assert!(location < upper);
            let absolute_position = (location - lower) as usize;
            &self.hash_table[absolute_position]
        }
        pub fn dht_write(&mut self, location: u32, data: T) {
            assert!(self.dht_is_initialised());
            let lower = self.lower_bound as u32;
            let upper = self.higher_bound as u32;
            assert!(location >= lower);
            assert!(location < upper);
            let absolute_position = (location - lower) as usize;
            self.hash_table[absolute_position] = Some(data);
        }
    }
}
