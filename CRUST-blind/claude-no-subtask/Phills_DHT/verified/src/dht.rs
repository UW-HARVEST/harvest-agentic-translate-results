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
                lower_bound: -1,
                higher_bound: -1,
            }
        }
        pub fn dht_is_initialised(&self) -> bool {
            self.lower_bound >= 0 && self.higher_bound >= 0
        }
        pub fn dht_init_table(&mut self, new_lower_bound: u32, new_upper_bound: u32, migrate: bool) -> bool {
            assert!(new_upper_bound > new_lower_bound);

            let new_hash_table_size = (new_upper_bound - new_lower_bound) as usize;

            // Allocate the new table, initialising every slot to None.
            let mut new_hash_table: Vec<Option<T>> = Vec::with_capacity(new_hash_table_size);
            for _ in 0..new_hash_table_size {
                new_hash_table.push(None);
            }

            // If a table is already installed, possibly migrate items.
            if self.dht_is_initialised() {
                let old_lower_bound = self.lower_bound as u32;
                // Take ownership of the old items so we can move them.
                let old_items: Vec<Option<T>> = std::mem::take(&mut self.hash_table);

                for (i, item_opt) in old_items.into_iter().enumerate() {
                    if let Some(item) = item_opt {
                        let absolute_position = (i as u32) + old_lower_bound;

                        if migrate
                            && absolute_position >= new_lower_bound
                            && absolute_position < new_upper_bound
                        {
                            let new_position = (absolute_position - new_lower_bound) as usize;
                            new_hash_table[new_position] = Some(item);
                        }
                        // else: item is "forgotten" (would be remapped to a peer in the
                        // original C implementation); we simply drop it here.
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
