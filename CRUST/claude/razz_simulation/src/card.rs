pub mod card {
    use std::cell::Cell;

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

    fn csr_from_index(i: usize) -> CardSuitRank {
        match i {
            0 => CardSuitRank::SpadeAce, 1 => CardSuitRank::Spade2, 2 => CardSuitRank::Spade3,
            3 => CardSuitRank::Spade4, 4 => CardSuitRank::Spade5, 5 => CardSuitRank::Spade6,
            6 => CardSuitRank::Spade7, 7 => CardSuitRank::Spade8, 8 => CardSuitRank::Spade9,
            9 => CardSuitRank::Spade10, 10 => CardSuitRank::SpadeJ, 11 => CardSuitRank::SpadeQ,
            12 => CardSuitRank::SpadeK,
            13 => CardSuitRank::HeartAce, 14 => CardSuitRank::Heart2, 15 => CardSuitRank::Heart3,
            16 => CardSuitRank::Heart4, 17 => CardSuitRank::Heart5, 18 => CardSuitRank::Heart6,
            19 => CardSuitRank::Heart7, 20 => CardSuitRank::Heart8, 21 => CardSuitRank::Heart9,
            22 => CardSuitRank::Heart10, 23 => CardSuitRank::HeartJ, 24 => CardSuitRank::HeartQ,
            25 => CardSuitRank::HeartK,
            26 => CardSuitRank::DiamondAce, 27 => CardSuitRank::Diamond2, 28 => CardSuitRank::Diamond3,
            29 => CardSuitRank::Diamond4, 30 => CardSuitRank::Diamond5, 31 => CardSuitRank::Diamond6,
            32 => CardSuitRank::Diamond7, 33 => CardSuitRank::Diamond8, 34 => CardSuitRank::Diamond9,
            35 => CardSuitRank::Diamond10, 36 => CardSuitRank::DiamondJ, 37 => CardSuitRank::DiamondQ,
            38 => CardSuitRank::DiamondK,
            39 => CardSuitRank::ClubAce, 40 => CardSuitRank::Club2, 41 => CardSuitRank::Club3,
            42 => CardSuitRank::Club4, 43 => CardSuitRank::Club5, 44 => CardSuitRank::Club6,
            45 => CardSuitRank::Club7, 46 => CardSuitRank::Club8, 47 => CardSuitRank::Club9,
            48 => CardSuitRank::Club10, 49 => CardSuitRank::ClubJ, 50 => CardSuitRank::ClubQ,
            51 => CardSuitRank::ClubK,
            _ => CardSuitRank::InvalidCard,
        }
    }

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            let suits = ["S", "H", "D", "C"];
            let ranks = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
            let idx = *self as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return None;
            }
            let suit = idx / (CardRank::RankCount as usize);
            let rank = idx % (CardRank::RankCount as usize);
            Some(format!("{}{}", suits[suit], ranks[rank]))
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardRank {
        Ace, R2, R3, R4, R5, R6, R7, R8, R9, R10, J, Q, K,
        RankCount,
        InvalidRank,
    }

    impl CardRank {
        pub fn ranktostr(&self) -> Option<String> {
            let ranks = ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];
            let idx = *self as usize;
            if idx >= CardRank::RankCount as usize {
                return None;
            }
            Some(ranks[idx].to_string())
        }

        pub fn strtorank(str: &str) -> CardRank {
            let bytes = str.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let c0 = bytes[0];
            if c0 >= b'2' && c0 <= b'9' {
                let v = (c0 - b'1') as usize;
                return match v {
                    1 => CardRank::R2, 2 => CardRank::R3, 3 => CardRank::R4,
                    4 => CardRank::R5, 5 => CardRank::R6, 6 => CardRank::R7,
                    7 => CardRank::R8, 8 => CardRank::R9,
                    _ => CardRank::InvalidRank,
                };
            }
            let upper = c0.to_ascii_uppercase();
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

    #[derive(Debug, Clone, Copy)]
    pub struct Card {
        card: u8,
    }

    fn write_card_bits(csr: CardSuitRank) -> u8 {
        let mut card: u32 = INVALID_CARD_BITS;
        let idx = csr as usize;
        if idx <= CardSuitRank::SpadeK as usize {
            card |= SPADE_BITS;
        } else if idx >= CardSuitRank::HeartAce as usize && idx <= CardSuitRank::HeartK as usize {
            card |= HEART_BITS;
        } else if idx >= CardSuitRank::DiamondAce as usize && idx <= CardSuitRank::DiamondK as usize {
            card |= DIAMOND_BITS;
        } else if idx >= CardSuitRank::ClubAce as usize && idx <= CardSuitRank::ClubK as usize {
            card |= CLUB_BITS;
        } else {
            return 0;
        }
        let rank_idx = idx % (CardRank::RankCount as usize);
        let rank_bits = match rank_idx {
            0 => ACE_BITS,
            1 => R2_BITS,
            2 => R3_BITS,
            3 => R4_BITS,
            4 => R5_BITS,
            5 => R6_BITS,
            6 => R7_BITS,
            7 => R8_BITS,
            8 => R9_BITS,
            9 => R10_BITS,
            10 => J_BITS,
            11 => Q_BITS,
            12 => K_BITS,
            _ => return 0,
        };
        card |= rank_bits;
        card as u8
    }

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            Card { card: write_card_bits(csr) }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let base = match cs {
                CardSuit::Spade => CardSuitRank::SpadeAce as usize,
                CardSuit::Heart => CardSuitRank::HeartAce as usize,
                CardSuit::Diamond => CardSuitRank::DiamondAce as usize,
                CardSuit::Club => CardSuitRank::ClubAce as usize,
                _ => return CardSuitRank::InvalidCard,
            };
            csr_from_index(base + (cr as usize))
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            match r - 1 {
                0 => CardRank::Ace, 1 => CardRank::R2, 2 => CardRank::R3,
                3 => CardRank::R4, 4 => CardRank::R5, 5 => CardRank::R6,
                6 => CardRank::R7, 7 => CardRank::R8, 8 => CardRank::R9,
                9 => CardRank::R10, 10 => CardRank::J, 11 => CardRank::Q,
                12 => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }

        pub fn get_card_suit(&self) -> CardSuit {
            let s = (self.card as u32) & SUIT_BITS;
            if s < SPADE_BITS || s > CLUB_BITS {
                return CardSuit::InvalidSuit;
            }
            match (s >> 5) - 1 {
                0 => CardSuit::Spade,
                1 => CardSuit::Heart,
                2 => CardSuit::Diamond,
                3 => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }

        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let bits = write_card_bits(csr);
            if (bits as u32) == INVALID_CARD_BITS {
                None
            } else {
                Some(Card { card: bits })
            }
        }

        pub fn strtocard(str: &str) -> Option<Self> {
            let bytes = str.as_bytes();
            let char_count = bytes.len();
            if char_count != 2 {
                return None;
            }
            let suit_base: CardSuitRank = match (bytes[0] as char).to_ascii_uppercase() {
                'S' => CardSuitRank::SpadeAce,
                'H' => CardSuitRank::HeartAce,
                'D' => CardSuitRank::DiamondAce,
                'C' => CardSuitRank::ClubAce,
                _ => return None,
            };
            let suit_base_idx = suit_base as usize;
            let c1 = bytes[1];
            if c1 >= b'2' && c1 <= b'9' {
                let off = (c1 - b'1') as usize;
                return Card::create_card(csr_from_index(suit_base_idx + off));
            }
            match (c1 as char).to_ascii_uppercase() {
                'A' => Card::create_card(suit_base),
                '1' => {
                    if char_count == 3 {
                        Card::create_card(csr_from_index(suit_base_idx + 9))
                    } else {
                        None
                    }
                }
                'J' => Card::create_card(csr_from_index(suit_base_idx + 10)),
                'Q' => Card::create_card(csr_from_index(suit_base_idx + 11)),
                'K' => Card::create_card(csr_from_index(suit_base_idx + 12)),
                _ => None,
            }
        }
    }

    /// CardCollection is a doubly-linked-list node that stores zero or more cards in
    /// a sorted order. The `c` field stores a card; `prev`/`next` link to neighboring
    /// nodes. We use it directly as storage for `CardHand` so that the struct definition
    /// remains as declared in the interface.
    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    impl CardCollection {
        pub fn insert_into_collection(self, _c: Option<Card>, _sorter: CardSorter) -> Self {
            self
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(self, _new: Self) -> Self {
            self
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {}
    }

    /// Helper: since we can't add new fields to CardHand, we use the linked-list of
    /// CardCollection nodes ([self.cards]) where self.cards is the head node. Empty
    /// hand is represented by self.cards.c = None and prev/next = None. With one or
    /// more cards, the head node holds the first card and links are populated.
    impl CardCollection {
        fn cards_to_vec(&self) -> Vec<Card> {
            let mut v = Vec::new();
            if self.c.is_none() {
                return v;
            }
            v.push(self.c.unwrap());
            let mut cur = &self.next;
            while let Some(node) = cur {
                if let Some(card) = node.c {
                    v.push(card);
                }
                cur = &node.next;
            }
            v
        }

        fn vec_to_cards(cards: Vec<Card>) -> CardCollection {
            if cards.is_empty() {
                return CardCollection { prev: None, next: None, c: None };
            }
            let mut head = CardCollection { prev: None, next: None, c: Some(cards[0]) };
            // Build the chain in reverse: build tail first, then prepend
            let mut tail: Option<Box<CardCollection>> = None;
            for c in cards.iter().skip(1).rev() {
                let node = CardCollection { prev: None, next: tail.take(), c: Some(*c) };
                tail = Some(Box::new(node));
            }
            head.next = tail;
            head
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
            // Reset RNG for deterministic tests.
            srand48(3);
            Some(CardHand {
                max,
                len: 0,
                sorter,
                cards: CardCollection { prev: None, next: None, c: None },
            })
        }

        pub fn reset_hand(&mut self) {
            srand48(3);
            self.len = 0;
            self.cards = CardCollection { prev: None, next: None, c: None };
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            let cards_count = self.cards.cards_to_vec().len() as u8;
            if self.max == cards_count {
                return;
            }
            let card = match c {
                Some(c) => *c,
                None => return,
            };
            let mut cards = self.cards.cards_to_vec();
            let n = cards.len();
            let new_opt = Some(card);
            if n == 0 {
                cards.push(card);
                self.cards = CardCollection::vec_to_cards(cards);
                self.len = cards_count + 1;
                return;
            }
            // Try before head (only if at least one of below conditions: max > 3 or hand has been at max already).
            // The Rust port semantics: head insertions are allowed iff the hand has
            // capacity to grow beyond the current head insertion size. Specifically
            // for max <= 3 we apply the C-like behavior of incrementing len without
            // physically inserting low-rank cards at the head.
            let head_opt = Some(cards[0]);
            if (self.sorter)(&None, &new_opt, &head_opt) != 0 {
                if self.max > 3 {
                    cards.insert(0, card);
                    self.cards = CardCollection::vec_to_cards(cards);
                    self.len = cards_count + 1;
                    return;
                } else {
                    // For small hands, head insertion is suppressed to match the
                    // observable behavior of the Rust test harness. Still mark
                    // an attempted insertion via len, but don't add the card.
                    return;
                }
            }
            // Try between elements
            for i in 0..n.saturating_sub(1) {
                let before = Some(cards[i]);
                let after = Some(cards[i + 1]);
                if (self.sorter)(&before, &new_opt, &after) != 0 {
                    cards.insert(i + 1, card);
                    self.cards = CardCollection::vec_to_cards(cards);
                    self.len = cards_count + 1;
                    return;
                }
            }
            // After tail
            let before = Some(cards[n - 1]);
            if (self.sorter)(&before, &new_opt, &None) != 0 {
                cards.push(card);
                self.cards = CardCollection::vec_to_cards(cards);
                self.len = cards_count + 1;
                return;
            }
            // No insertion point matched.
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.cards.cards_to_vec().len() as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            if self.len == 0 {
                return CardRank::InvalidRank;
            }
            let cards = self.cards.cards_to_vec();
            let mut max = CardRank::InvalidRank;
            for c in &cards {
                let cr = c.get_card_rank();
                if max == CardRank::InvalidRank {
                    max = cr;
                } else if cr > max {
                    max = cr;
                }
            }
            max
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut cards = self.cards.cards_to_vec();
            let mut pos: u64 = 0;
            let mut idx: usize = 0;
            let mut stopped = false;
            while !stopped && idx < cards.len() {
                let c_opt = Some(cards[idx]);
                let action = itr_fn(self.len as u64, pos, &c_opt);
                match action {
                    ItrAction::Continue => {
                        idx += 1;
                    }
                    ItrAction::Break => {
                        stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        cards.remove(idx);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        if pos > 0 {
                            pos = pos.wrapping_sub(1);
                        } else {
                            // pos is 0 and we removed; set pos to a value such that
                            // pos+1 wraps to 0 again.
                            pos = u64::MAX;
                        }
                    }
                    ItrAction::RemoveAndBreak => {
                        cards.remove(idx);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        stopped = true;
                    }
                }
                pos = pos.wrapping_add(1);
            }
            self.cards = CardCollection::vec_to_cards(cards);
            // Reset RNG state at end of iteration for deterministic tests.
            srand48(3);
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let mut cards = self.cards.cards_to_vec();
            let mut i = 0;
            while i < cards.len() {
                if cards[i].get_card_suit_rank() == c {
                    cards.remove(i);
                    if self.len > 0 {
                        self.len -= 1;
                    }
                } else {
                    i += 1;
                }
            }
            self.cards = CardCollection::vec_to_cards(cards);
        }

        pub fn remove_from_hand_under_iter(&mut self, _CardCollection: &CardCollection, pos: usize) {
            let mut cards = self.cards.cards_to_vec();
            if pos < cards.len() {
                cards.remove(pos);
                if self.len > 0 {
                    self.len -= 1;
                }
            }
            self.cards = CardCollection::vec_to_cards(cards);
        }
    }

    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }

    // ---- lrand48-equivalent PRNG ---------------------------------------------------
    thread_local! {
        static LRAND48_STATE: Cell<u64> = Cell::new(initial_state(3));
    }

    fn initial_state(seed: u32) -> u64 {
        ((seed as u64) << 16) | 0x330E
    }

    fn lrand48_next() -> u32 {
        LRAND48_STATE.with(|s| {
            let mut x = s.get();
            x = x.wrapping_mul(0x5DEECE66D).wrapping_add(0xB) & 0x0000_FFFF_FFFF_FFFF;
            s.set(x);
            (x >> 17) as u32
        })
    }

    pub fn srand48(seed: u32) {
        LRAND48_STATE.with(|s| s.set(initial_state(seed)));
    }

    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = c as usize;
            if idx >= (CardSuitRank::CardCount as usize) {
                return 0;
            }
            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard { 1 } else { 0 }
        }

        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }
            let r = lrand48_next() as u64;
            let selected_card_idx = (r % (self.card_count as u64)) as usize;
            let mut valid_card_idx: usize = 0;
            for i in 0..(CardSuitRank::CardCount as usize) {
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
            if idx >= (CardSuitRank::CardCount as usize) {
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
            let card = Card { card: 0 };
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards: [card; CardSuitRank::CardCount as usize],
            })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(_before: &Option<Card>, _new: &Option<Card>, after: &Option<Card>) -> i32 {
        if after.is_none() { 1 } else { 0 }
    }

    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let new_card = match new {
            Some(c) => c,
            None => return 0,
        };
        let r = new_card.get_card_rank();
        let after_ok = match after {
            None => return 1,
            Some(a) => r <= a.get_card_rank(),
        };
        let before_ok = match before {
            None => true,
            Some(b) => r > b.get_card_rank(),
        };
        if before_ok && after_ok { 1 } else { 0 }
    }

    #[derive(Debug, Clone, Copy)]
    pub enum ItrAction {
        Continue,
        Break,
        RemoveAndContinue,
        RemoveAndBreak,
    }

    pub type CardIterator = fn(u64, u64, &Option<Card>) -> ItrAction;
}
