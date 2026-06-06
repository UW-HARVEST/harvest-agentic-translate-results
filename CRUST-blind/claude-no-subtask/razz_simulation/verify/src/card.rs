pub mod card {
    use std::cell::Cell;
    use std::time::SystemTime;

    pub const CLUB_BITS: u32 = 4 << 5;
    pub const ACE_BITS: u32 = 1;
    pub const J_BITS: u32 = 11;
    pub const R8_BITS: u32 = 8;
    pub const R10_BITS: u32 = 10;
    pub const HEART_BITS: u32 = 2 << 5;
    pub const R4_BITS: u32 = 4;
    pub const SPADE_BITS: u32 = 1 << 5;
    pub const Q_BITS: u32 = 12;
    pub const R7_BITS: u32 = 7;
    pub const R5_BITS: u32 = 5;
    pub const K_BITS: u32 = 13;
    pub const R3_BITS: u32 = 3;
    pub const RANK_BITS: u32 = 0x1F;
    pub const R9_BITS: u32 = 9;
    pub const SUIT_BITS: u32 = 0x7 << 5;
    pub const R6_BITS: u32 = 6;
    pub const INVALID_CARD_BITS: u32 = 0;
    pub const DIAMOND_BITS: u32 = 3 << 5;
    pub const R2_BITS: u32 = 2;
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuitRank {
        SpadeAce, Spade2, Spade3, Spade4, Spade5, Spade6, Spade7, Spade8,
        Spade9, Spade10, SpadeJ, SpadeQ, SpadeK,
        HeartAce, Heart2, Heart3, Heart4, Heart5, Heart6, Heart7, Heart8,
        Heart9, Heart10, HeartJ, HeartQ, HeartK,
        DiamondAce, Diamond2, Diamond3, Diamond4, Diamond5, Diamond6,
        Diamond7, Diamond8, Diamond9, Diamond10, DiamondJ, DiamondQ, DiamondK,
        ClubAce, Club2, Club3, Club4, Club5, Club6, Club7, Club8, Club9,
        Club10, ClubJ, ClubQ, ClubK,
        CardCount,
        InvalidCard,
    }

    const CSR_TABLE: [CardSuitRank; 52] = [
        CardSuitRank::SpadeAce, CardSuitRank::Spade2, CardSuitRank::Spade3,
        CardSuitRank::Spade4, CardSuitRank::Spade5, CardSuitRank::Spade6,
        CardSuitRank::Spade7, CardSuitRank::Spade8, CardSuitRank::Spade9,
        CardSuitRank::Spade10, CardSuitRank::SpadeJ, CardSuitRank::SpadeQ,
        CardSuitRank::SpadeK,
        CardSuitRank::HeartAce, CardSuitRank::Heart2, CardSuitRank::Heart3,
        CardSuitRank::Heart4, CardSuitRank::Heart5, CardSuitRank::Heart6,
        CardSuitRank::Heart7, CardSuitRank::Heart8, CardSuitRank::Heart9,
        CardSuitRank::Heart10, CardSuitRank::HeartJ, CardSuitRank::HeartQ,
        CardSuitRank::HeartK,
        CardSuitRank::DiamondAce, CardSuitRank::Diamond2, CardSuitRank::Diamond3,
        CardSuitRank::Diamond4, CardSuitRank::Diamond5, CardSuitRank::Diamond6,
        CardSuitRank::Diamond7, CardSuitRank::Diamond8, CardSuitRank::Diamond9,
        CardSuitRank::Diamond10, CardSuitRank::DiamondJ, CardSuitRank::DiamondQ,
        CardSuitRank::DiamondK,
        CardSuitRank::ClubAce, CardSuitRank::Club2, CardSuitRank::Club3,
        CardSuitRank::Club4, CardSuitRank::Club5, CardSuitRank::Club6,
        CardSuitRank::Club7, CardSuitRank::Club8, CardSuitRank::Club9,
        CardSuitRank::Club10, CardSuitRank::ClubJ, CardSuitRank::ClubQ,
        CardSuitRank::ClubK,
    ];

    fn csr_from_index(i: usize) -> CardSuitRank {
        if i >= 52 { CardSuitRank::InvalidCard } else { CSR_TABLE[i] }
    }

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            const CARD_STRS: [&str; 52] = [
                "SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK",
                "HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK",
                "DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK",
                "CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK",
            ];
            let idx = *self as usize;
            if idx >= 52 {
                return None;
            }
            Some(CARD_STRS[idx].to_string())
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount,
        InvalidRank,
    }

    fn rank_from_index(i: usize) -> CardRank {
        match i {
            0 => CardRank::Ace,
            1 => CardRank::R2,
            2 => CardRank::R3,
            3 => CardRank::R4,
            4 => CardRank::R5,
            5 => CardRank::R6,
            6 => CardRank::R7,
            7 => CardRank::R8,
            8 => CardRank::R9,
            9 => CardRank::R10,
            10 => CardRank::J,
            11 => CardRank::Q,
            12 => CardRank::K,
            _ => CardRank::InvalidRank,
        }
    }

    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            const RANK_STRS: [&str; 13] = [
                "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
            ];
            let idx = *self as usize;
            if idx >= 13 {
                return None;
            }
            Some(RANK_STRS[idx].to_string())
        }
        pub fn strtorank(str: &str) -> CardRank {
            let bytes = str.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let first = bytes[0];

            if first >= b'2' && first <= b'9' {
                // ACE + (digit - 1) = digit_index in our enum
                let offset = (first - b'1') as usize;
                return rank_from_index(offset);
            }

            let upper = first.to_ascii_uppercase();
            match upper {
                b'A' => CardRank::Ace,
                b'1' => {
                    if bytes.len() >= 2 && bytes[1] == b'0' {
                        CardRank::R10
                    } else {
                        CardRank::InvalidRank
                    }
                }
                b'J' => CardRank::J,
                b'Q' => CardRank::Q,
                b'K' => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuit {
        Spade, Heart, Diamond, Club,
        SuitCount,
        InvalidSuit,
    }

    fn suit_from_index(i: usize) -> CardSuit {
        match i {
            0 => CardSuit::Spade,
            1 => CardSuit::Heart,
            2 => CardSuit::Diamond,
            3 => CardSuit::Club,
            _ => CardSuit::InvalidSuit,
        }
    }

    pub struct Card {
        card: u8,
    }

    impl Clone for Card {
        fn clone(&self) -> Self {
            Card { card: self.card }
        }
    }
    impl Copy for Card {}

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let idx = csr as usize;
            if idx >= 52 {
                return Card { card: INVALID_CARD_BITS as u8 };
            }
            let suit_idx = idx / 13;
            let rank_idx = idx % 13;
            let suit_bits = ((suit_idx as u32) + 1) << 5;
            let rank_bits = (rank_idx as u32) + 1;
            Card { card: (suit_bits | rank_bits) as u8 }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let suit_idx = cs as usize;
            let rank_idx = cr as usize;
            let csr_idx = suit_idx * 13 + rank_idx;
            csr_from_index(csr_idx)
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            rank_from_index((r - 1) as usize)
        }

        pub fn get_card_suit(&self) -> CardSuit {
            let s = (self.card as u32) & SUIT_BITS;
            if s < SPADE_BITS || s > CLUB_BITS {
                return CardSuit::InvalidSuit;
            }
            suit_from_index(((s >> 5) - 1) as usize)
        }

        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let c = Self::write_card(csr);
            if c.card == INVALID_CARD_BITS as u8 {
                None
            } else {
                Some(c)
            }
        }

        pub fn strtocard(str: &str) -> Option<Self> {
            let bytes = str.as_bytes();
            if bytes.len() != 2 {
                return None;
            }

            let csr_base = match bytes[0].to_ascii_uppercase() {
                b'S' => 0usize,
                b'H' => 13usize,
                b'D' => 26usize,
                b'C' => 39usize,
                _ => return None,
            };

            let offset: usize = if bytes[1] >= b'2' && bytes[1] <= b'9' {
                (bytes[1] - b'1') as usize
            } else {
                match bytes[1].to_ascii_uppercase() {
                    b'A' => 0usize,
                    b'J' => 10usize,
                    b'Q' => 11usize,
                    b'K' => 12usize,
                    _ => return None,
                }
            };

            let csr_idx = csr_base + offset;
            Self::create_card(csr_from_index(csr_idx))
        }
    }

    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    fn empty_collection() -> CardCollection {
        CardCollection {
            prev: None,
            next: None,
            c: None,
        }
    }

    fn collection_insert(coll: &mut CardCollection, new_card: Option<Card>, sorter: CardSorter) -> bool {
        if new_card.is_none() {
            return false;
        }

        if coll.c.is_none() {
            coll.c = new_card;
            coll.next = None;
            coll.prev = None;
            return true;
        }

        let none_card: Option<Card> = None;

        // Check insertion before head.
        if sorter(&none_card, &new_card, &coll.c) != 0 {
            let old_head = std::mem::replace(coll, CardCollection {
                prev: None,
                next: None,
                c: new_card,
            });
            coll.next = Some(Box::new(old_head));
            return true;
        }

        // Walk through and find spot.
        let mut current: &mut CardCollection = coll;
        loop {
            let before_card: Option<Card> = current.c;
            let after_card: Option<Card> = current.next.as_ref().and_then(|n| n.c);

            if sorter(&before_card, &new_card, &after_card) != 0 {
                let new_node = CardCollection {
                    prev: None,
                    next: current.next.take(),
                    c: new_card,
                };
                current.next = Some(Box::new(new_node));
                return true;
            }

            if current.next.is_some() {
                current = current.next.as_mut().unwrap();
            } else {
                return false;
            }
        }
    }

    fn collection_get_at(coll: &CardCollection, idx: usize) -> Option<Card> {
        if coll.c.is_none() {
            return None;
        }
        if idx == 0 {
            return coll.c;
        }
        let mut current = coll.next.as_ref();
        let mut i = 1usize;
        while let Some(node) = current {
            if i == idx {
                return node.c;
            }
            current = node.next.as_ref();
            i += 1;
        }
        None
    }

    fn collection_remove_at(coll: &mut CardCollection, idx: usize) {
        if coll.c.is_none() {
            return;
        }
        if idx == 0 {
            if let Some(next_box) = coll.next.take() {
                *coll = *next_box;
                coll.prev = None;
            } else {
                coll.c = None;
                coll.prev = None;
            }
            return;
        }
        let mut current: &mut CardCollection = coll;
        for _ in 1..idx {
            if current.next.is_none() {
                return;
            }
            current = current.next.as_mut().unwrap();
        }
        if let Some(mut to_remove) = current.next.take() {
            current.next = to_remove.next.take();
        }
    }

    #[allow(dead_code)]
    fn collection_count(coll: &CardCollection) -> usize {
        if coll.c.is_none() {
            return 0;
        }
        let mut n = 1usize;
        let mut current = coll.next.as_ref();
        while let Some(node) = current {
            n += 1;
            current = node.next.as_ref();
        }
        n
    }

    impl CardCollection {
        pub fn insert_into_collection(mut self, c: Option<Card>, sorter: CardSorter) -> Self {
            collection_insert(&mut self, c, sorter);
            self
        }
        pub fn iterate_collection(&self) -> &Self {
            self
        }
        pub fn append_into_collection(mut self, new: Self) -> Self {
            if self.c.is_none() {
                return new;
            }
            if new.c.is_none() {
                return self;
            }
            let mut current: &mut CardCollection = &mut self;
            while current.next.is_some() {
                current = current.next.as_mut().unwrap();
            }
            current.next = Some(Box::new(new));
            self
        }
        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // Singly-linked traversal; detachment by reference is not feasible
            // through safe Rust given the &Option<Box<...>> signature. The
            // CardHand methods perform removal directly via index instead.
        }
    }

    pub struct CardHand {
        max: u8,
        len: u8,
        sorter: CardSorter,
        cards: CardCollection,
    }

    impl CardHand {
        pub fn create_hand(max: u8, sorter: CardSorter) -> Option<CardHand> {
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: empty_collection(),
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = empty_collection();
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.len >= self.max {
                return;
            }
            if c.is_none() {
                return;
            }
            let inserted = collection_insert(&mut self.cards, *c, self.sorter);
            if inserted {
                self.len += 1;
            }
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            if self.len == 0 {
                return CardRank::InvalidRank;
            }
            let mut cr = CardRank::InvalidRank;
            let mut idx = 0usize;
            while let Some(card) = collection_get_at(&self.cards, idx) {
                let this_cr = card.get_card_rank();
                if cr == CardRank::InvalidRank {
                    cr = this_cr;
                } else if this_cr > cr {
                    cr = this_cr;
                }
                idx += 1;
            }
            cr
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut idx: usize = 0;
            let mut pos: u64 = 0;
            loop {
                let card_opt = collection_get_at(&self.cards, idx);
                if card_opt.is_none() {
                    break;
                }
                let action = itr_fn(self.len as u64, pos, &card_opt);
                match action {
                    ItrAction::Continue => {
                        idx += 1;
                        pos += 1;
                    }
                    ItrAction::Break => {
                        break;
                    }
                    ItrAction::RemoveAndContinue => {
                        collection_remove_at(&mut self.cards, idx);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        // pos and idx remain unchanged.
                    }
                    ItrAction::RemoveAndBreak => {
                        collection_remove_at(&mut self.cards, idx);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        break;
                    }
                }
            }
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let mut idx: usize = 0;
            while idx < self.len as usize {
                match collection_get_at(&self.cards, idx) {
                    Some(card) => {
                        if card.get_card_suit_rank() == c {
                            collection_remove_at(&mut self.cards, idx);
                            if self.len > 0 {
                                self.len -= 1;
                            }
                        } else {
                            idx += 1;
                        }
                    }
                    None => break,
                }
            }
        }

        #[allow(non_snake_case, unused_variables)]
        pub fn remove_from_hand_under_iter(&mut self, CardCollection: &CardCollection, pos: usize) {
            if pos < self.len as usize {
                collection_remove_at(&mut self.cards, pos);
                if self.len > 0 {
                    self.len -= 1;
                }
            }
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = c as usize;
            if idx >= 52 {
                return 0;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                1
            } else {
                0
            }
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }
            let selected_card_idx = (rand_u64() % self.card_count as u64) as usize;
            let mut valid_card_idx: usize = 0;
            for i in 0..52 {
                if self.cards[i].get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_card_idx == selected_card_idx {
                        let csr = csr_from_index(i);
                        self.cards[i] = Card::write_card(csr);
                        self.card_count -= 1;
                        return Some(self.cards[i]);
                    }
                    valid_card_idx += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= 52 {
                return;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                if self.card_count > 0 {
                    self.card_count -= 1;
                }
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            let cards: [Card; 52] = [Card { card: 0 }; 52];
            Some(CardDeck {
                card_count: 52,
                cards,
            })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(_before: &Option<Card>, _new: &Option<Card>, after: &Option<Card>) -> i32 {
        if after.is_none() {
            1
        } else {
            0
        }
    }

    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let r = match new {
            Some(c) => c.get_card_rank(),
            None => CardRank::InvalidRank,
        };

        if after.is_none() {
            return 1;
        }

        let after_rank = after.as_ref().unwrap().get_card_rank();
        let before_ok = match before {
            None => true,
            Some(c) => r > c.get_card_rank(),
        };

        if before_ok && r <= after_rank {
            return 1;
        }
        0
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ItrAction {
        Continue,
        Break,
        RemoveAndContinue,
        RemoveAndBreak,
    }

    pub type CardIterator = fn(u64, u64, &Option<Card>) -> ItrAction;

    // Thread-local PRNG (simple xorshift64) used by deal_from_deck.
    thread_local! {
        static RNG_STATE: Cell<u64> = const { Cell::new(0) };
    }

    fn rand_u64() -> u64 {
        RNG_STATE.with(|s| {
            let mut x = s.get();
            if x == 0 {
                x = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|d| (d.as_nanos() as u64) ^ 0x9E3779B97F4A7C15)
                    .unwrap_or(0xDEADBEEFCAFEBABE);
                if x == 0 {
                    x = 0xCAFEBABE12345678;
                }
            }
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            s.set(x);
            x
        })
    }
}
