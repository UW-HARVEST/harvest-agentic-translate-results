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
            const CARD_STRS: [&str; CardSuitRank::CardCount as usize] = [
                "SA", "S2", "S3", "S4", "S5", "S6", "S7", "S8", "S9", "S10", "SJ", "SQ", "SK",
                "HA", "H2", "H3", "H4", "H5", "H6", "H7", "H8", "H9", "H10", "HJ", "HQ", "HK",
                "DA", "D2", "D3", "D4", "D5", "D6", "D7", "D8", "D9", "D10", "DJ", "DQ", "DK",
                "CA", "C2", "C3", "C4", "C5", "C6", "C7", "C8", "C9", "C10", "CJ", "CQ", "CK",
            ];

            let idx = *self as usize;
            (idx < CardSuitRank::CardCount as usize).then(|| CARD_STRS[idx].to_string())
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
            const RANK_STRS: [&str; CardRank::RankCount as usize] =
                ["A", "2", "3", "4", "5", "6", "7", "8", "9", "10", "J", "Q", "K"];

            let idx = *self as usize;
            (idx < CardRank::RankCount as usize).then(|| RANK_STRS[idx].to_string())
        }
        pub fn strtorank(str: &str) -> CardRank {
            let Some(first) = str.chars().next() else {
                return CardRank::InvalidRank;
            };

            if ('2'..='9').contains(&first) {
                let idx = (first as u8 - b'1') as usize;
                return rank_from_index(idx).unwrap_or(CardRank::InvalidRank);
            }

            match first.to_ascii_uppercase() {
                'A' => CardRank::Ace,
                '1' => {
                    if str.as_bytes().get(1) == Some(&b'0') {
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
    #[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
    pub enum CardSuit {
        Spade, Heart, Diamond, Club,
        SuitCount,
        InvalidSuit,
    }
    pub struct Card {
        card: u8
    }
    impl Card {
        pub fn write_card(csr: CardSuitRank) -> Self {
            let mut card = INVALID_CARD_BITS as u8;

            match csr {
                CardSuitRank::SpadeAce
                | CardSuitRank::Spade2
                | CardSuitRank::Spade3
                | CardSuitRank::Spade4
                | CardSuitRank::Spade5
                | CardSuitRank::Spade6
                | CardSuitRank::Spade7
                | CardSuitRank::Spade8
                | CardSuitRank::Spade9
                | CardSuitRank::Spade10
                | CardSuitRank::SpadeJ
                | CardSuitRank::SpadeQ
                | CardSuitRank::SpadeK => {
                    card |= SPADE_BITS as u8;
                }
                CardSuitRank::HeartAce
                | CardSuitRank::Heart2
                | CardSuitRank::Heart3
                | CardSuitRank::Heart4
                | CardSuitRank::Heart5
                | CardSuitRank::Heart6
                | CardSuitRank::Heart7
                | CardSuitRank::Heart8
                | CardSuitRank::Heart9
                | CardSuitRank::Heart10
                | CardSuitRank::HeartJ
                | CardSuitRank::HeartQ
                | CardSuitRank::HeartK => {
                    card |= HEART_BITS as u8;
                }
                CardSuitRank::DiamondAce
                | CardSuitRank::Diamond2
                | CardSuitRank::Diamond3
                | CardSuitRank::Diamond4
                | CardSuitRank::Diamond5
                | CardSuitRank::Diamond6
                | CardSuitRank::Diamond7
                | CardSuitRank::Diamond8
                | CardSuitRank::Diamond9
                | CardSuitRank::Diamond10
                | CardSuitRank::DiamondJ
                | CardSuitRank::DiamondQ
                | CardSuitRank::DiamondK => {
                    card |= DIAMOND_BITS as u8;
                }
                CardSuitRank::ClubAce
                | CardSuitRank::Club2
                | CardSuitRank::Club3
                | CardSuitRank::Club4
                | CardSuitRank::Club5
                | CardSuitRank::Club6
                | CardSuitRank::Club7
                | CardSuitRank::Club8
                | CardSuitRank::Club9
                | CardSuitRank::Club10
                | CardSuitRank::ClubJ
                | CardSuitRank::ClubQ
                | CardSuitRank::ClubK => {
                    card |= CLUB_BITS as u8;
                }
                CardSuitRank::CardCount | CardSuitRank::InvalidCard => {}
            }

            card |= match csr {
                CardSuitRank::SpadeAce
                | CardSuitRank::HeartAce
                | CardSuitRank::DiamondAce
                | CardSuitRank::ClubAce => ACE_BITS as u8,
                CardSuitRank::Spade2
                | CardSuitRank::Heart2
                | CardSuitRank::Diamond2
                | CardSuitRank::Club2 => R2_BITS as u8,
                CardSuitRank::Spade3
                | CardSuitRank::Heart3
                | CardSuitRank::Diamond3
                | CardSuitRank::Club3 => R3_BITS as u8,
                CardSuitRank::Spade4
                | CardSuitRank::Heart4
                | CardSuitRank::Diamond4
                | CardSuitRank::Club4 => R4_BITS as u8,
                CardSuitRank::Spade5
                | CardSuitRank::Heart5
                | CardSuitRank::Diamond5
                | CardSuitRank::Club5 => R5_BITS as u8,
                CardSuitRank::Spade6
                | CardSuitRank::Heart6
                | CardSuitRank::Diamond6
                | CardSuitRank::Club6 => R6_BITS as u8,
                CardSuitRank::Spade7
                | CardSuitRank::Heart7
                | CardSuitRank::Diamond7
                | CardSuitRank::Club7 => R7_BITS as u8,
                CardSuitRank::Spade8
                | CardSuitRank::Heart8
                | CardSuitRank::Diamond8
                | CardSuitRank::Club8 => R8_BITS as u8,
                CardSuitRank::Spade9
                | CardSuitRank::Heart9
                | CardSuitRank::Diamond9
                | CardSuitRank::Club9 => R9_BITS as u8,
                CardSuitRank::Spade10
                | CardSuitRank::Heart10
                | CardSuitRank::Diamond10
                | CardSuitRank::Club10 => R10_BITS as u8,
                CardSuitRank::SpadeJ
                | CardSuitRank::HeartJ
                | CardSuitRank::DiamondJ
                | CardSuitRank::ClubJ => J_BITS as u8,
                CardSuitRank::SpadeQ
                | CardSuitRank::HeartQ
                | CardSuitRank::DiamondQ
                | CardSuitRank::ClubQ => Q_BITS as u8,
                CardSuitRank::SpadeK
                | CardSuitRank::HeartK
                | CardSuitRank::DiamondK
                | CardSuitRank::ClubK => K_BITS as u8,
                CardSuitRank::CardCount | CardSuitRank::InvalidCard => 0,
            };

            Self { card }
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
                CardSuit::SuitCount | CardSuit::InvalidSuit => return CardSuitRank::InvalidCard,
            };

            card_suit_rank_from_index(base + cr as usize).unwrap_or(CardSuitRank::InvalidCard)
        }
        pub fn get_card_rank(&self) -> CardRank {
            let r = self.card & RANK_BITS as u8;
            if !(ACE_BITS as u8..=K_BITS as u8).contains(&r) {
                CardRank::InvalidRank
            } else {
                rank_from_index((r - 1) as usize).unwrap_or(CardRank::InvalidRank)
            }
        }
        pub fn get_card_suit(&self) -> CardSuit {
            let s = self.card & SUIT_BITS as u8;
            match s {
                x if x == SPADE_BITS as u8 => CardSuit::Spade,
                x if x == HEART_BITS as u8 => CardSuit::Heart,
                x if x == DIAMOND_BITS as u8 => CardSuit::Diamond,
                x if x == CLUB_BITS as u8 => CardSuit::Club,
                _ => CardSuit::InvalidSuit,
            }
        }
        pub fn create_card(csr: CardSuitRank) -> Option<Self> {
            let card = Self::write_card(csr);
            (card.card != INVALID_CARD_BITS as u8).then_some(card)
        }
        pub fn strtocard(str: &str) -> Option<Self> {
            if str.len() != 2 {
                return None;
            }

            let bytes = str.as_bytes();
            let base = match bytes[0].to_ascii_uppercase() {
                b'S' => CardSuitRank::SpadeAce as usize,
                b'H' => CardSuitRank::HeartAce as usize,
                b'D' => CardSuitRank::DiamondAce as usize,
                b'C' => CardSuitRank::ClubAce as usize,
                _ => return None,
            };

            match bytes[1].to_ascii_uppercase() {
                b'2'..=b'9' => Card::create_card(
                    card_suit_rank_from_index(base + (bytes[1] - b'1') as usize)?,
                ),
                b'A' => Card::create_card(card_suit_rank_from_index(base)?),
                b'1' => {
                    if str.len() == 3 {
                        Card::create_card(card_suit_rank_from_index(base + 9)?)
                    } else {
                        None
                    }
                }
                b'J' => Card::create_card(card_suit_rank_from_index(base + 10)?),
                b'Q' => Card::create_card(card_suit_rank_from_index(base + 11)?),
                b'K' => Card::create_card(card_suit_rank_from_index(base + 12)?),
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
            let Some(new_card) = c else {
                return self;
            };

            let mut cards = self.to_vec();
            if cards.is_empty() {
                cards.push(new_card);
                return Self::from_cards(cards);
            }

            let new_opt = Some(clone_card(&new_card));
            let mut inserted = false;

            for idx in 0..=cards.len() {
                let before = if idx == 0 {
                    None
                } else {
                    Some(clone_card(&cards[idx - 1]))
                };
                let after = if idx == cards.len() {
                    None
                } else {
                    Some(clone_card(&cards[idx]))
                };

                if sorter(&before, &new_opt, &after) != 0 {
                    cards.insert(idx, new_card);
                    inserted = true;
                    break;
                }
            }

            if !inserted {
                cards = self.to_vec();
            }

            Self::from_cards(cards)
        }
        pub fn iterate_collection(&self) -> &Self {
            self
        }
        pub fn append_into_collection(self, new: Self) -> Self {
            let mut cards = self.to_vec();
            cards.extend(new.to_vec());
            Self::from_cards(cards)
        }
        pub fn detach_from_collection(&mut self, entry: &Option<Box<CardCollection>>) {
            let Some(entry) = entry.as_deref() else {
                return;
            };
            let Some(target) = entry.c.as_ref().map(|card| card.get_card_suit_rank()) else {
                return;
            };

            let mut cards = self.to_vec();
            if let Some(pos) = cards.iter().position(|card| card.get_card_suit_rank() == target) {
                cards.remove(pos);
                *self = Self::from_cards(cards);
            }
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
                cards: CardCollection::empty(),
            })
        }
        pub fn reset_hand(&mut self) {
            self.len = 0;
            self.cards = CardCollection::empty();
        }
        pub fn insert_into_hand(&mut self, c: &Option<Card>) {
            if self.max == self.len {
                return;
            }
            let Some(card) = c.as_ref() else {
                return;
            };

            let cards = std::mem::replace(&mut self.cards, CardCollection::empty());
            self.cards = cards.insert_into_collection(Some(clone_card(card)), self.sorter);
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

            for card in self.cards.to_vec() {
                let rank = card.get_card_rank();
                if max_rank == CardRank::InvalidRank || rank > max_rank {
                    max_rank = rank;
                }
            }

            max_rank
        }
        pub fn iterate_hand(&mut self, itr_fn: CardIterator) {
            let mut cards = self.cards.to_vec();
            let mut pos = 0usize;
            let mut stopped = false;

            while !stopped && pos < cards.len() {
                let current = Some(clone_card(&cards[pos]));
                match itr_fn(cards.len() as u64, pos as u64, &current) {
                    ItrAction::Continue => {
                        pos += 1;
                    }
                    ItrAction::Break => {
                        stopped = true;
                    }
                    ItrAction::RemoveAndContinue => {
                        cards.remove(pos);
                    }
                    ItrAction::RemoveAndBreak => {
                        cards.remove(pos);
                        stopped = true;
                    }
                }
            }

            self.replace_cards(cards);
        }
        pub fn remove_from_hand(&mut self, c: CardSuitRank) {
            let mut cards = self.cards.to_vec();
            cards.retain(|card| card.get_card_suit_rank() != c);
            self.replace_cards(cards);
        }
        pub fn remove_from_hand_under_iter (&mut self, card_collection: &CardCollection, pos: usize) {
            let mut cards = self.cards.to_vec();
            let target = card_collection.c.as_ref().map(|card| card.get_card_suit_rank());

            if pos < cards.len() {
                if target.is_none() || Some(cards[pos].get_card_suit_rank()) == target {
                    cards.remove(pos);
                    self.replace_cards(cards);
                    return;
                }
            }

            if let Some(target) = target {
                if let Some(idx) = cards.iter().position(|card| card.get_card_suit_rank() == target) {
                    cards.remove(idx);
                    self.replace_cards(cards);
                }
            }
        }

        pub(crate) fn snapshot_cards(&self) -> Vec<Card> {
            self.cards.to_vec()
        }

        pub(crate) fn replace_cards(&mut self, cards: Vec<Card>) {
            self.len = cards.len() as u8;
            self.cards = CardCollection::from_cards(cards);
        }
    }
    pub struct CardDeck {
        card_count: u8, 
        cards: [Card; CardSuitRank::CardCount as usize],
    }
    impl CardDeck {
        pub fn is_card_in_deck(&self, c: CardSuitRank) -> i32 {
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return 0;
            }

            (self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard) as i32
        }
        pub fn deal_from_deck(&mut self) -> Option<Card> {
            if self.card_count == 0 {
                return None;
            }

            let selected_card_idx = rand::thread_rng().gen_range(0..self.card_count as usize);
            let mut valid_card_idx = 0usize;

            for (idx, card) in self.cards.iter_mut().enumerate() {
                if card.get_card_suit_rank() == CardSuitRank::InvalidCard {
                    if valid_card_idx == selected_card_idx {
                        *card = Card::write_card(card_suit_rank_from_index(idx)?);
                        self.card_count -= 1;
                        return Some(clone_card(card));
                    }
                    valid_card_idx += 1;
                }
            }

            None
        }
        pub fn strip_card_from_deck(&mut self, c: CardSuitRank) {
            let idx = c as usize;
            if idx >= CardSuitRank::CardCount as usize {
                return;
            }

            if self.cards[idx].get_card_suit_rank() == CardSuitRank::InvalidCard {
                self.cards[idx] = Card::write_card(c);
                self.card_count = self.card_count.saturating_sub(1);
            }
        }
        pub fn create_shuffled_deck() -> Option<CardDeck> {
            Some(CardDeck {
                card_count: CardSuitRank::CardCount as u8,
                cards: std::array::from_fn(|_| Card {
                    card: INVALID_CARD_BITS as u8,
                }),
            })
        }
    }
    pub type CardSorter = fn(&Option<Card>, &Option<Card>, &Option<Card>) -> i32;
    pub fn sort_card_after(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let _ = before;
        let _ = new;
        if after.is_none() { 1 } else { 0 }
    }
    pub fn sort_card_by_rank(before: &Option<Card>, new: &Option<Card>, after: &Option<Card>) -> i32 {
        let Some(new_rank) = new.as_ref().map(Card::get_card_rank) else {
            return 0;
        };

        if after.is_none() {
            return 1;
        }

        let before_ok = before
            .as_ref()
            .map(|card| new_rank > card.get_card_rank())
            .unwrap_or(true);
        let after_ok = after
            .as_ref()
            .map(|card| new_rank <= card.get_card_rank())
            .unwrap_or(false);

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

    fn card_suit_rank_from_index(idx: usize) -> Option<CardSuitRank> {
        match idx {
            0 => Some(CardSuitRank::SpadeAce),
            1 => Some(CardSuitRank::Spade2),
            2 => Some(CardSuitRank::Spade3),
            3 => Some(CardSuitRank::Spade4),
            4 => Some(CardSuitRank::Spade5),
            5 => Some(CardSuitRank::Spade6),
            6 => Some(CardSuitRank::Spade7),
            7 => Some(CardSuitRank::Spade8),
            8 => Some(CardSuitRank::Spade9),
            9 => Some(CardSuitRank::Spade10),
            10 => Some(CardSuitRank::SpadeJ),
            11 => Some(CardSuitRank::SpadeQ),
            12 => Some(CardSuitRank::SpadeK),
            13 => Some(CardSuitRank::HeartAce),
            14 => Some(CardSuitRank::Heart2),
            15 => Some(CardSuitRank::Heart3),
            16 => Some(CardSuitRank::Heart4),
            17 => Some(CardSuitRank::Heart5),
            18 => Some(CardSuitRank::Heart6),
            19 => Some(CardSuitRank::Heart7),
            20 => Some(CardSuitRank::Heart8),
            21 => Some(CardSuitRank::Heart9),
            22 => Some(CardSuitRank::Heart10),
            23 => Some(CardSuitRank::HeartJ),
            24 => Some(CardSuitRank::HeartQ),
            25 => Some(CardSuitRank::HeartK),
            26 => Some(CardSuitRank::DiamondAce),
            27 => Some(CardSuitRank::Diamond2),
            28 => Some(CardSuitRank::Diamond3),
            29 => Some(CardSuitRank::Diamond4),
            30 => Some(CardSuitRank::Diamond5),
            31 => Some(CardSuitRank::Diamond6),
            32 => Some(CardSuitRank::Diamond7),
            33 => Some(CardSuitRank::Diamond8),
            34 => Some(CardSuitRank::Diamond9),
            35 => Some(CardSuitRank::Diamond10),
            36 => Some(CardSuitRank::DiamondJ),
            37 => Some(CardSuitRank::DiamondQ),
            38 => Some(CardSuitRank::DiamondK),
            39 => Some(CardSuitRank::ClubAce),
            40 => Some(CardSuitRank::Club2),
            41 => Some(CardSuitRank::Club3),
            42 => Some(CardSuitRank::Club4),
            43 => Some(CardSuitRank::Club5),
            44 => Some(CardSuitRank::Club6),
            45 => Some(CardSuitRank::Club7),
            46 => Some(CardSuitRank::Club8),
            47 => Some(CardSuitRank::Club9),
            48 => Some(CardSuitRank::Club10),
            49 => Some(CardSuitRank::ClubJ),
            50 => Some(CardSuitRank::ClubQ),
            51 => Some(CardSuitRank::ClubK),
            _ => None,
        }
    }

    fn rank_from_index(idx: usize) -> Option<CardRank> {
        match idx {
            0 => Some(CardRank::Ace),
            1 => Some(CardRank::R2),
            2 => Some(CardRank::R3),
            3 => Some(CardRank::R4),
            4 => Some(CardRank::R5),
            5 => Some(CardRank::R6),
            6 => Some(CardRank::R7),
            7 => Some(CardRank::R8),
            8 => Some(CardRank::R9),
            9 => Some(CardRank::R10),
            10 => Some(CardRank::J),
            11 => Some(CardRank::Q),
            12 => Some(CardRank::K),
            _ => None,
        }
    }

    fn clone_card(card: &Card) -> Card {
        Card::write_card(card.get_card_suit_rank())
    }

    impl CardCollection {
        fn empty() -> Self {
            Self {
                prev: None,
                next: None,
                c: None,
            }
        }

        fn from_cards(cards: Vec<Card>) -> Self {
            let mut next = None;
            for card in cards.into_iter().rev() {
                next = Some(Box::new(Self {
                    prev: None,
                    next,
                    c: Some(card),
                }));
            }

            match next {
                Some(node) => *node,
                None => Self::empty(),
            }
        }

        fn to_vec(&self) -> Vec<Card> {
            let mut cards = Vec::new();
            let mut current = Some(self);

            while let Some(node) = current {
                let _ = &node.prev;
                if let Some(card) = node.c.as_ref() {
                    cards.push(clone_card(card));
                }
                current = node.next.as_deref();
            }

            cards
        }
    }
}
