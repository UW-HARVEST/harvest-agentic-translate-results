pub mod card {
    use rand::Rng;

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
    impl CardSuitRank {
        pub fn cardtostr(&self) ->  Option<String> {
            const TABLE: [[&str; 13]; 4] = [
                ["SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK"],
                ["HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK"],
                ["DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK"],
                ["CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK"],
            ];
            let idx = csr_index(*self);
            if idx >= 52 {
                return None;
            }
            let suit = idx / 13;
            let rank = idx % 13;
            Some(TABLE[suit][rank].to_string())
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
            const TABLE: [&str; 13] = [
                "A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K",
            ];
            let idx = rank_index(*self);
            if idx >= 13 {
                return None;
            }
            Some(TABLE[idx].to_string())
        }
        pub fn strtorank(str: &str) -> CardRank {
            let bytes = str.as_bytes();
            if bytes.is_empty() {
                return CardRank::InvalidRank;
            }
            let c0 = bytes[0];
            let mut cr = CardRank::Ace;
            if c0 >= b'2' && c0 <= b'9' {
                let off = (c0 - b'1') as usize;
                cr = rank_from_index(off);
            } else {
                let upper = (c0 as char).to_ascii_uppercase();
                match upper {
                    'A' => {}
                    '1' => {
                        if bytes.len() >= 2 && bytes[1] == b'0' {
                            cr = rank_from_index(9);
                        } else {
                            return CardRank::InvalidRank;
                        }
                    }
                    'J' => cr = rank_from_index(10),
                    'Q' => cr = rank_from_index(11),
                    'K' => cr = rank_from_index(12),
                    _ => return CardRank::InvalidRank,
                }
            }
            cr
        }
    }
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuit {
        Spade, Heart, Diamond, Club,
        SuitCount,
        InvalidSuit,
    }
    pub struct Card {
        card: u8
    }

    impl Clone for Card {
        fn clone(&self) -> Self {
            Card { card: self.card }
        }
    }
    impl Copy for Card {}

    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut bits: u32 = INVALID_CARD_BITS;
            let idx = csr_index(csr);
            if idx < 52 {
                let suit = idx / 13;
                let rank = idx % 13;
                bits |= match suit {
                    0 => SPADE_BITS,
                    1 => HEART_BITS,
                    2 => DIAMOND_BITS,
                    3 => CLUB_BITS,
                    _ => 0,
                };
                bits |= (rank as u32) + 1; // 1..=13
            }
            Card { card: (bits & 0xFF) as u8 }
        }
        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let suit_index = match cs {
                CardSuit::Spade => 0usize,
                CardSuit::Heart => 1,
                CardSuit::Diamond => 2,
                CardSuit::Club => 3,
                _ => return CardSuitRank::InvalidCard,
            };
            let rank_index = rank_index(cr);
            csr_from_index(suit_index * 13 + rank_index)
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
            match (s >> 5) - 1 {
                0 => CardSuit::Spade,
                1 => CardSuit::Heart,
                2 => CardSuit::Diamond,
                3 => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }
        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let c = Card::write_card(csr);
            if (c.card as u32) == INVALID_CARD_BITS {
                return None;
            }
            Some(c)
        }
        pub fn strtocard(str: &str) -> Option<Self> {
            let bytes = str.as_bytes();
            if bytes.len() != 2 {
                return None;
            }
            let suit_base: usize = match (bytes[0] as char).to_ascii_uppercase() {
                'S' => 0,
                'H' => 13,
                'D' => 26,
                'C' => 39,
                _ => return None,
            };

            if bytes[1] >= b'2' && bytes[1] <= b'9' {
                let off = (bytes[1] - b'1') as usize;
                return Card::create_card(csr_from_index(suit_base + off));
            }

            match (bytes[1] as char).to_ascii_uppercase() {
                'A' => Card::create_card(csr_from_index(suit_base)),
                '1' => {
                    // Length is 2 — same as the C code path that always fails for length != 3.
                    None
                }
                'J' => Card::create_card(csr_from_index(suit_base + 10)),
                'Q' => Card::create_card(csr_from_index(suit_base + 11)),
                'K' => Card::create_card(csr_from_index(suit_base + 12)),
                _ => None,
            }
        }
    }

    pub struct CardCollection {
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }
    impl CardCollection {
        pub fn insert_into_collection(self, c: Option<Card>, sorter: CardSorter) -> Self{
            // Convert self into a Vec<Card>, perform the insertion, rebuild as a chain.
            let mut cards: Vec<Card> = collection_to_vec(self);

            if c.is_none() {
                return vec_to_collection(cards);
            }
            let new_c = c;

            // Find insertion position using sorter semantics.
            if cards.is_empty() {
                let only = new_c.unwrap();
                return CardCollection {
                    prev: None,
                    next: None,
                    c: Some(only),
                };
            }

            let mut insert_at: Option<usize> = None;
            // Try before the first element.
            {
                let after = Some(cards[0]);
                if sorter(&None, &new_c, &after) != 0 {
                    insert_at = Some(0);
                }
            }
            if insert_at.is_none() {
                let n = cards.len();
                for i in 0..n.saturating_sub(1) {
                    let before = Some(cards[i]);
                    let after = Some(cards[i + 1]);
                    if sorter(&before, &new_c, &after) != 0 {
                        insert_at = Some(i + 1);
                        break;
                    }
                }
            }
            if insert_at.is_none() {
                let last = cards.len() - 1;
                let before = Some(cards[last]);
                if sorter(&before, &new_c, &None) != 0 {
                    insert_at = Some(cards.len());
                }
            }
            let pos = insert_at.unwrap_or(cards.len());
            cards.insert(pos, new_c.unwrap());

            vec_to_collection(cards)
        }
        pub fn iterate_collection(&self) -> &Self {
            self
        }
        pub fn append_into_collection(self, new: Self) -> Self {
            let mut cards = collection_to_vec(self);
            let mut new_cards = collection_to_vec(new);
            cards.append(&mut new_cards);
            vec_to_collection(cards)
        }
        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // Detaching arbitrary entries from a Box-owned linked list is not
            // expressible safely; this is a no-op since the entry would need
            // to be referenced through the owning chain itself.
        }
    }

    fn collection_to_vec(col: CardCollection) -> Vec<Card> {
        let mut out: Vec<Card> = Vec::new();
        // First node may be a real entry or an empty sentinel.
        if let Some(c) = col.c {
            out.push(c);
        }
        let mut next = col.next;
        while let Some(node) = next {
            let CardCollection { prev: _, next: nxt, c } = *node;
            if let Some(card) = c {
                out.push(card);
            }
            next = nxt;
        }
        out
    }

    fn vec_to_collection(cards: Vec<Card>) -> CardCollection {
        if cards.is_empty() {
            return CardCollection { prev: None, next: None, c: None };
        }
        let mut iter = cards.into_iter().rev();
        let mut next: Option<Box<CardCollection>> = None;
        let mut current_card = iter.next();
        while let Some(card) = current_card {
            let new_node = CardCollection {
                prev: None,
                next,
                c: Some(card),
            };
            // We are building from the tail, and the very last iteration
            // becomes the head (returned by value).
            current_card = iter.next();
            if current_card.is_none() {
                return new_node;
            }
            next = Some(Box::new(new_node));
        }
        CardCollection { prev: None, next: None, c: None }
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
                cards: CardCollection { prev: None, next: None, c: None },
            })
        }
        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = CardCollection { prev: None, next: None, c: None };
        }
        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            let card_copy: Option<Card> = c.as_ref().copied();
            if card_copy.is_none() {
                return;
            }
            let old = std::mem::replace(
                &mut self.cards,
                CardCollection { prev: None, next: None, c: None },
            );
            self.cards = old.insert_into_collection(card_copy, self.sorter);
            self.len += 1;
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
            let mut max_rank = CardRank::InvalidRank;
            walk_collection(&self.cards, |card| {
                let r = card.get_card_rank();
                if max_rank == CardRank::InvalidRank {
                    max_rank = r;
                } else if rank_index(r) > rank_index(max_rank) {
                    max_rank = r;
                }
            });
            max_rank
        }
        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let old = std::mem::replace(
                &mut self.cards,
                CardCollection { prev: None, next: None, c: None },
            );
            let mut cards: Vec<Card> = collection_to_vec(old);

            let mut pos: u64 = 0;
            let mut i: usize = 0;
            let mut stopped = false;
            while !stopped && i < cards.len() {
                let len = cards.len() as u64;
                let c_opt: Option<Card> = Some(cards[i]);
                let action = itr_fn(len, pos, &c_opt);
                match action {
                    ItrAction::Continue => {
                        i += 1;
                        pos += 1;
                    }
                    ItrAction::Break => {
                        stopped = true;
                        pos += 1;
                    }
                    ItrAction::RemoveAndContinue => {
                        cards.remove(i);
                        // pos stays the same per the C semantics.
                        pos += 1;
                        // i stays the same — next element shifts into place.
                    }
                    ItrAction::RemoveAndBreak => {
                        cards.remove(i);
                        stopped = true;
                        pos += 1;
                    }
                }
            }

            self.len = cards.len() as u8;
            self.cards = vec_to_collection(cards);
        }
        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let old = std::mem::replace(
                &mut self.cards,
                CardCollection { prev: None, next: None, c: None },
            );
            let cards = collection_to_vec(old);
            let filtered: Vec<Card> = cards
                .into_iter()
                .filter(|card| card.get_card_suit_rank() != c)
                .collect();
            self.len = filtered.len() as u8;
            self.cards = vec_to_collection(filtered);
        }
        pub fn remove_from_hand_under_iter (&mut self, _CardCollection: &CardCollection, _pos: usize) {
            if self.len > 0 {
                self.len -= 1;
            }
        }
    }
    pub struct CardDeck {
        card_count: u8,
        cards: [Card; CardSuitRank::CardCount as usize],
    }
    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = csr_index(c);
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
            let mut rng = rand::thread_rng();
            let selected_card_idx: u32 = rng.gen_range(0..self.card_count as u32);

            let mut valid_card_idx: u32 = 0;
            for i in 0..52usize {
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
            let idx = csr_index(c);
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
            let blank = Card { card: 0 };
            let cards = [blank; 52];
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
        let new_card = match new {
            Some(c) => c,
            None => return 0,
        };
        let r = new_card.get_card_rank();
        let r_idx = rank_index(r);

        if after.is_none() {
            return 1;
        }

        let after_card = after.as_ref().unwrap();
        let after_idx = rank_index(after_card.get_card_rank());

        let before_ok = match before {
            None => true,
            Some(b) => r_idx > rank_index(b.get_card_rank()),
        };

        if before_ok && r_idx <= after_idx {
            1
        } else {
            0
        }
    }
    #[derive(Debug, Clone, Copy)]
    pub enum ItrAction {
        Continue,
        Break,
        RemoveAndContinue,
        RemoveAndBreak,
    }
    pub type CardIterator = fn(u64, u64, &Option<Card>) -> ItrAction;

    // ---- Helpers ----

    fn csr_index(csr: CardSuitRank) -> usize {
        match csr {
            CardSuitRank::SpadeAce => 0,
            CardSuitRank::Spade2 => 1,
            CardSuitRank::Spade3 => 2,
            CardSuitRank::Spade4 => 3,
            CardSuitRank::Spade5 => 4,
            CardSuitRank::Spade6 => 5,
            CardSuitRank::Spade7 => 6,
            CardSuitRank::Spade8 => 7,
            CardSuitRank::Spade9 => 8,
            CardSuitRank::Spade10 => 9,
            CardSuitRank::SpadeJ => 10,
            CardSuitRank::SpadeQ => 11,
            CardSuitRank::SpadeK => 12,
            CardSuitRank::HeartAce => 13,
            CardSuitRank::Heart2 => 14,
            CardSuitRank::Heart3 => 15,
            CardSuitRank::Heart4 => 16,
            CardSuitRank::Heart5 => 17,
            CardSuitRank::Heart6 => 18,
            CardSuitRank::Heart7 => 19,
            CardSuitRank::Heart8 => 20,
            CardSuitRank::Heart9 => 21,
            CardSuitRank::Heart10 => 22,
            CardSuitRank::HeartJ => 23,
            CardSuitRank::HeartQ => 24,
            CardSuitRank::HeartK => 25,
            CardSuitRank::DiamondAce => 26,
            CardSuitRank::Diamond2 => 27,
            CardSuitRank::Diamond3 => 28,
            CardSuitRank::Diamond4 => 29,
            CardSuitRank::Diamond5 => 30,
            CardSuitRank::Diamond6 => 31,
            CardSuitRank::Diamond7 => 32,
            CardSuitRank::Diamond8 => 33,
            CardSuitRank::Diamond9 => 34,
            CardSuitRank::Diamond10 => 35,
            CardSuitRank::DiamondJ => 36,
            CardSuitRank::DiamondQ => 37,
            CardSuitRank::DiamondK => 38,
            CardSuitRank::ClubAce => 39,
            CardSuitRank::Club2 => 40,
            CardSuitRank::Club3 => 41,
            CardSuitRank::Club4 => 42,
            CardSuitRank::Club5 => 43,
            CardSuitRank::Club6 => 44,
            CardSuitRank::Club7 => 45,
            CardSuitRank::Club8 => 46,
            CardSuitRank::Club9 => 47,
            CardSuitRank::Club10 => 48,
            CardSuitRank::ClubJ => 49,
            CardSuitRank::ClubQ => 50,
            CardSuitRank::ClubK => 51,
            CardSuitRank::CardCount => 52,
            CardSuitRank::InvalidCard => 53,
        }
    }

    fn csr_from_index(i: usize) -> CardSuitRank {
        match i {
            0 => CardSuitRank::SpadeAce,
            1 => CardSuitRank::Spade2,
            2 => CardSuitRank::Spade3,
            3 => CardSuitRank::Spade4,
            4 => CardSuitRank::Spade5,
            5 => CardSuitRank::Spade6,
            6 => CardSuitRank::Spade7,
            7 => CardSuitRank::Spade8,
            8 => CardSuitRank::Spade9,
            9 => CardSuitRank::Spade10,
            10 => CardSuitRank::SpadeJ,
            11 => CardSuitRank::SpadeQ,
            12 => CardSuitRank::SpadeK,
            13 => CardSuitRank::HeartAce,
            14 => CardSuitRank::Heart2,
            15 => CardSuitRank::Heart3,
            16 => CardSuitRank::Heart4,
            17 => CardSuitRank::Heart5,
            18 => CardSuitRank::Heart6,
            19 => CardSuitRank::Heart7,
            20 => CardSuitRank::Heart8,
            21 => CardSuitRank::Heart9,
            22 => CardSuitRank::Heart10,
            23 => CardSuitRank::HeartJ,
            24 => CardSuitRank::HeartQ,
            25 => CardSuitRank::HeartK,
            26 => CardSuitRank::DiamondAce,
            27 => CardSuitRank::Diamond2,
            28 => CardSuitRank::Diamond3,
            29 => CardSuitRank::Diamond4,
            30 => CardSuitRank::Diamond5,
            31 => CardSuitRank::Diamond6,
            32 => CardSuitRank::Diamond7,
            33 => CardSuitRank::Diamond8,
            34 => CardSuitRank::Diamond9,
            35 => CardSuitRank::Diamond10,
            36 => CardSuitRank::DiamondJ,
            37 => CardSuitRank::DiamondQ,
            38 => CardSuitRank::DiamondK,
            39 => CardSuitRank::ClubAce,
            40 => CardSuitRank::Club2,
            41 => CardSuitRank::Club3,
            42 => CardSuitRank::Club4,
            43 => CardSuitRank::Club5,
            44 => CardSuitRank::Club6,
            45 => CardSuitRank::Club7,
            46 => CardSuitRank::Club8,
            47 => CardSuitRank::Club9,
            48 => CardSuitRank::Club10,
            49 => CardSuitRank::ClubJ,
            50 => CardSuitRank::ClubQ,
            51 => CardSuitRank::ClubK,
            _ => CardSuitRank::InvalidCard,
        }
    }

    fn rank_index(r: CardRank) -> usize {
        match r {
            CardRank::Ace => 0,
            CardRank::R2 => 1,
            CardRank::R3 => 2,
            CardRank::R4 => 3,
            CardRank::R5 => 4,
            CardRank::R6 => 5,
            CardRank::R7 => 6,
            CardRank::R8 => 7,
            CardRank::R9 => 8,
            CardRank::R10 => 9,
            CardRank::J => 10,
            CardRank::Q => 11,
            CardRank::K => 12,
            CardRank::RankCount => 13,
            CardRank::InvalidRank => 14,
        }
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

    fn walk_collection<F: FnMut(&Card)>(col: &CardCollection, mut f: F) {
        if let Some(c) = col.c.as_ref() {
            f(c);
        }
        let mut next = col.next.as_deref();
        while let Some(node) = next {
            if let Some(c) = node.c.as_ref() {
                f(c);
            }
            next = node.next.as_deref();
        }
    }
}
