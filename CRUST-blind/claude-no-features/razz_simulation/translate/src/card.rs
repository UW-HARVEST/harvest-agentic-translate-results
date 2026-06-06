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

    fn csr_from_index(i: u32) -> CardSuitRank {
        use CardSuitRank::*;
        match i {
            0 => SpadeAce, 1 => Spade2, 2 => Spade3, 3 => Spade4, 4 => Spade5,
            5 => Spade6, 6 => Spade7, 7 => Spade8, 8 => Spade9, 9 => Spade10,
            10 => SpadeJ, 11 => SpadeQ, 12 => SpadeK,
            13 => HeartAce, 14 => Heart2, 15 => Heart3, 16 => Heart4, 17 => Heart5,
            18 => Heart6, 19 => Heart7, 20 => Heart8, 21 => Heart9, 22 => Heart10,
            23 => HeartJ, 24 => HeartQ, 25 => HeartK,
            26 => DiamondAce, 27 => Diamond2, 28 => Diamond3, 29 => Diamond4,
            30 => Diamond5, 31 => Diamond6, 32 => Diamond7, 33 => Diamond8,
            34 => Diamond9, 35 => Diamond10, 36 => DiamondJ, 37 => DiamondQ,
            38 => DiamondK,
            39 => ClubAce, 40 => Club2, 41 => Club3, 42 => Club4, 43 => Club5,
            44 => Club6, 45 => Club7, 46 => Club8, 47 => Club9, 48 => Club10,
            49 => ClubJ, 50 => ClubQ, 51 => ClubK,
            _ => InvalidCard,
        }
    }

    impl CardSuitRank {
        pub fn cardtostr(&self) -> Option<String> {
            let csr_idx = *self as u32;
            if csr_idx >= 52 {
                return None;
            }
            let suit_idx = csr_idx / 13;
            let rank_idx = csr_idx % 13;
            let suit_str = match suit_idx {
                0 => "S",
                1 => "H",
                2 => "D",
                3 => "C",
                _ => return None,
            };
            let rank_str = match rank_idx {
                0 => "A",
                1 => "2",
                2 => "3",
                3 => "4",
                4 => "5",
                5 => "6",
                6 => "7",
                7 => "8",
                8 => "9",
                9 => "10",
                10 => "J",
                11 => "Q",
                12 => "K",
                _ => return None,
            };
            Some(format!("{}{}", suit_str, rank_str))
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
            match self {
                CardRank::Ace => Some("A".to_string()),
                CardRank::R2 => Some("2".to_string()),
                CardRank::R3 => Some("3".to_string()),
                CardRank::R4 => Some("4".to_string()),
                CardRank::R5 => Some("5".to_string()),
                CardRank::R6 => Some("6".to_string()),
                CardRank::R7 => Some("7".to_string()),
                CardRank::R8 => Some("8".to_string()),
                CardRank::R9 => Some("9".to_string()),
                CardRank::R10 => Some("10".to_string()),
                CardRank::J => Some("J".to_string()),
                CardRank::Q => Some("Q".to_string()),
                CardRank::K => Some("K".to_string()),
                _ => None,
            }
        }
        pub fn strtorank(str: &str) -> CardRank {
            let chars: Vec<char> = str.chars().collect();
            if chars.is_empty() {
                return CardRank::InvalidRank;
            }
            let first = chars[0];
            if ('2'..='9').contains(&first) {
                let offset = first as u32 - '1' as u32; // '2'->1, '9'->8
                return rank_from_index(offset);
            }
            match first.to_ascii_uppercase() {
                'A' => CardRank::Ace,
                '1' => {
                    if chars.len() >= 2 && chars[1] == '0' {
                        CardRank::R10
                    } else {
                        CardRank::InvalidRank
                    }
                }
                'J' => CardRank::J,
                'Q' => CardRank::Q,
                'K' => CardRank::K,
                _ => CardRank::InvalidRank,
            }
        }
    }

    fn rank_from_index(i: u32) -> CardRank {
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
            let csr_idx = csr as u32;
            if csr_idx >= 52 {
                return Card { card: INVALID_CARD_BITS as u8 };
            }
            let suit_idx = csr_idx / 13;
            let rank_idx = csr_idx % 13;
            let suit_bits = match suit_idx {
                0 => SPADE_BITS,
                1 => HEART_BITS,
                2 => DIAMOND_BITS,
                3 => CLUB_BITS,
                _ => return Card { card: INVALID_CARD_BITS as u8 },
            };
            let rank_bits = rank_idx + ACE_BITS; // rank index 0 -> ACE_BITS=1
            Card { card: (suit_bits | rank_bits) as u8 }
        }

        pub fn get_card_suit_rank(&self) -> CardSuitRank {
            let cs = self.get_card_suit();
            let cr = self.get_card_rank();
            if cs == CardSuit::InvalidSuit || cr == CardRank::InvalidRank {
                return CardSuitRank::InvalidCard;
            }
            let cs_idx = cs as u32;
            let cr_idx = cr as u32;
            csr_from_index(cs_idx * 13 + cr_idx)
        }

        pub fn get_card_rank(&self) -> CardRank {
            let r = (self.card as u32) & RANK_BITS;
            if r < ACE_BITS || r > K_BITS {
                return CardRank::InvalidRank;
            }
            rank_from_index(r - 1)
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
            let c = Self::write_card(csr);
            if c.card == INVALID_CARD_BITS as u8 {
                None
            } else {
                Some(c)
            }
        }

        pub fn strtocard(str: &str) -> Option<Self> {
            let chars: Vec<char> = str.chars().collect();
            if chars.len() != 2 {
                return None;
            }
            let suit_offset: u32 = match chars[0].to_ascii_uppercase() {
                'S' => 0,
                'H' => 13,
                'D' => 26,
                'C' => 39,
                _ => return None,
            };
            let rank_offset: u32 = if ('2'..='9').contains(&chars[1]) {
                chars[1] as u32 - '1' as u32 // '2'->1, ..., '9'->8
            } else {
                match chars[1].to_ascii_uppercase() {
                    'A' => 0,
                    'J' => 10,
                    'Q' => 11,
                    'K' => 12,
                    _ => return None,
                }
            };
            let csr_idx = suit_offset + rank_offset;
            Self::create_card(csr_from_index(csr_idx))
        }
    }

    pub struct CardCollection {
        #[allow(dead_code)]
        prev: Option<Box<CardCollection>>,
        next: Option<Box<CardCollection>>,
        c: Option<Card>,
    }

    impl CardCollection {
        fn new_empty() -> Self {
            CardCollection { prev: None, next: None, c: None }
        }

        fn into_vec(self) -> Vec<Card> {
            let mut v = Vec::new();
            let mut cur: Option<CardCollection> = Some(self);
            while let Some(node) = cur {
                if let Some(card) = node.c {
                    v.push(card);
                }
                cur = node.next.map(|b| *b);
            }
            v
        }

        fn from_vec(v: Vec<Card>) -> Self {
            if v.is_empty() {
                return Self::new_empty();
            }
            let mut head: Option<Box<CardCollection>> = None;
            for card in v.into_iter().rev() {
                let new_node = Box::new(CardCollection {
                    prev: None,
                    next: head.take(),
                    c: Some(card),
                });
                head = Some(new_node);
            }
            *head.unwrap()
        }

        pub fn insert_into_collection(self, c: Option<Card>, sorter: CardSorter) -> Self {
            let card = match c {
                Some(c) => c,
                None => return self,
            };
            let mut v = self.into_vec();
            let n = v.len();
            let cc: Option<Card> = Some(card);

            if n == 0 {
                v.push(card);
                return Self::from_vec(v);
            }

            // Try before head
            let head_opt: Option<Card> = Some(v[0]);
            if sorter(&None, &cc, &head_opt) != 0 {
                v.insert(0, card);
                return Self::from_vec(v);
            }

            // Try between elements
            if n > 1 {
                for i in 0..n - 1 {
                    let before: Option<Card> = Some(v[i]);
                    let after: Option<Card> = Some(v[i + 1]);
                    if sorter(&before, &cc, &after) != 0 {
                        v.insert(i + 1, card);
                        return Self::from_vec(v);
                    }
                }
            }

            // Try at end
            let last: Option<Card> = Some(v[n - 1]);
            if sorter(&last, &cc, &None) != 0 {
                v.push(card);
            }

            Self::from_vec(v)
        }

        pub fn iterate_collection(&self) -> &Self {
            self
        }

        pub fn append_into_collection(self, new: Self) -> Self {
            let mut v1 = self.into_vec();
            let v2 = new.into_vec();
            v1.extend(v2);
            Self::from_vec(v1)
        }

        pub fn detach_from_collection(&mut self, _entry: &Option<Box<CardCollection>>) {
            // The Rust signature takes a reference to an Option<Box<...>>, which makes
            // it impossible to identify which entry to detach without pointer comparison.
            // This is left as a no-op. Removal is performed through CardHand methods.
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
                cards: CardCollection::new_empty(),
            })
        }

        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = CardCollection::new_empty();
        }

        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.len >= self.max {
                return;
            }
            let card_to_insert = match c {
                Some(c) => *c,
                None => return,
            };
            let cards = std::mem::replace(&mut self.cards, CardCollection::new_empty());
            self.cards = cards.insert_into_collection(Some(card_to_insert), self.sorter);
            self.len += 1;
        }

        pub fn count_cards_in_hand(&self) -> u64 {
            self.len as u64
        }

        pub fn get_max_of_hand(&self) -> u64 {
            self.max as u64
        }

        pub fn get_max_rank_of_hand(&self) -> CardRank {
            let mut max_rank = CardRank::InvalidRank;
            // Walk linked list via &self
            let mut cur: Option<&CardCollection> = Some(&self.cards);
            while let Some(node) = cur {
                if let Some(card) = &node.c {
                    let r = card.get_card_rank();
                    if r != CardRank::InvalidRank {
                        if max_rank == CardRank::InvalidRank || r > max_rank {
                            max_rank = r;
                        }
                    }
                }
                cur = node.next.as_deref();
            }
            max_rank
        }

        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let cards = std::mem::replace(&mut self.cards, CardCollection::new_empty());
            let mut v: Vec<Card> = cards.into_vec();
            let mut pos: u64 = 0;
            let mut i: usize = 0;
            let mut stopped = false;
            while !stopped && i < v.len() {
                let card_opt: Option<Card> = Some(v[i]);
                let len = v.len() as u64;
                let action = itr_fn(len, pos, &card_opt);
                let mut advance = true;
                let mut increment_pos = true;
                match action {
                    ItrAction::Continue => {}
                    ItrAction::Break => {
                        stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        v.remove(i);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        advance = false;
                        increment_pos = false;
                    }
                    ItrAction::RemoveAndBreak => {
                        v.remove(i);
                        if self.len > 0 {
                            self.len -= 1;
                        }
                        stopped = true;
                        advance = false;
                    }
                }
                if advance {
                    i += 1;
                }
                if increment_pos {
                    pos += 1;
                }
            }
            self.cards = CardCollection::from_vec(v);
        }

        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let cards = std::mem::replace(&mut self.cards, CardCollection::new_empty());
            let v: Vec<Card> = cards.into_vec();
            let mut new_v = Vec::with_capacity(v.len());
            for card in v {
                if card.get_card_suit_rank() != c {
                    new_v.push(card);
                } else if self.len > 0 {
                    self.len -= 1;
                }
            }
            self.cards = CardCollection::from_vec(new_v);
        }

        #[allow(non_snake_case)]
        pub fn remove_from_hand_under_iter(&mut self, CardCollection: &CardCollection, pos: usize) {
            let _ = CardCollection;
            let cards = std::mem::replace(&mut self.cards, CardCollection::new_empty());
            let mut v: Vec<Card> = cards.into_vec();
            if pos < v.len() {
                v.remove(pos);
                if self.len > 0 {
                    self.len -= 1;
                }
            }
            self.cards = CardCollection::from_vec(v);
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
            if self.cards[idx].card == INVALID_CARD_BITS as u8 {
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
            let selected: u64 = rng.gen_range(0..self.card_count as u64);
            let mut valid_idx: u64 = 0;
            for i in 0..(CardSuitRank::CardCount as usize) {
                if self.cards[i].card == INVALID_CARD_BITS as u8 {
                    if valid_idx == selected {
                        let csr = csr_from_index(i as u32);
                        let new_card = Card::write_card(csr);
                        self.cards[i] = Card { card: new_card.card };
                        self.card_count -= 1;
                        return Some(Card { card: new_card.card });
                    }
                    valid_idx += 1;
                }
            }
            None
        }

        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= 52 {
                return;
            }
            if self.cards[idx].card == INVALID_CARD_BITS as u8 {
                let new_card = Card::write_card(c);
                self.cards[idx] = Card { card: new_card.card };
                if self.card_count > 0 {
                    self.card_count -= 1;
                }
            }
        }

        pub fn create_shuffled_deck() -> Option<CardDeck> {
            let cards: [Card; CardSuitRank::CardCount as usize] =
                [Card { card: INVALID_CARD_BITS as u8 }; CardSuitRank::CardCount as usize];
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards,
            })
        }
    }

    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;

    pub fn sort_card_after(_before: &Option<Card>, _new: &Option<Card>, after: &Option<Card>) -> i32 {
        if after.is_none() { 1 } else { 0 }
    }

    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let r = match new {
            Some(c) => c.get_card_rank(),
            None => return 0,
        };
        if after.is_none() {
            return 1;
        }
        let after_r = after.as_ref().unwrap().get_card_rank();
        let before_ok = match before {
            None => true,
            Some(c) => r > c.get_card_rank(),
        };
        if before_ok && r <= after_r {
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
}
