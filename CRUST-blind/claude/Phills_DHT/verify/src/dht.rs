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

            // Allocate the new hash table with all None entries.
            let mut new_hash_table: Vec<Option<T>> = Vec::with_capacity(new_hash_table_size);
            for _ in 0..new_hash_table_size {
                new_hash_table.push(None);
            }

            // If currently initialised, possibly migrate entries to the new table.
            if self.dht_is_initialised() {
                let old_hash_table_size = self.dht_get_size() as usize;
                let old_lower_bound = self.lower_bound as u32;

                for i in 0..old_hash_table_size {
                    // Take ownership of the existing item, leaving None behind.
                    let item = std::mem::replace(&mut self.hash_table[i], None);

                    if let Some(value) = item {
                        let absolute_position = (i as u32).wrapping_add(old_lower_bound);

                        if migrate
                            && absolute_position >= new_lower_bound
                            && absolute_position < new_upper_bound
                        {
                            let new_index = (absolute_position - new_lower_bound) as usize;
                            new_hash_table[new_index] = Some(value);
                        }
                        // else: forgotten, would be remapped to a peer in a real DHT.
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
            assert!(location >= self.lower_bound as u32);
            assert!(location < self.higher_bound as u32);

            let absolute_position = location - self.lower_bound as u32;
            &self.hash_table[absolute_position as usize]
        }
        pub fn dht_write(&mut self, location: u32, data: T) {
            assert!(self.dht_is_initialised());
            assert!(location >= self.lower_bound as u32);
            assert!(location < self.higher_bound as u32);

            let absolute_position = location - self.lower_bound as u32;
            self.hash_table[absolute_position as usize] = Some(data);
        }
    }
}
