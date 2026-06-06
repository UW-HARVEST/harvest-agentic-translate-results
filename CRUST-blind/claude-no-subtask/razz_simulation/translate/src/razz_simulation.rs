use crate::card::card::{Card, CardDeck, CardHand, CardRank};
pub mod razz_simulation {
    use super::*;
    use crate::card::card::{
        sort_card_by_rank, CardSuitRank, ItrAction,
    };
    use std::cell::Cell;

    pub struct DecidedCards {
        pub my_card_count: u8,
        pub my_cards: [Option<Card>; 3],
        pub opponent_card_count: u8,
        pub opponent_cards: [Option<Card>; 7],
    }

    pub type RankListener<T> = fn(&mut T, CardRank);

    /// The number of cards in a Razz hand at the end of a round.
    const RAZZ_CARD_IN_HAND_COUNT: u8 = 7;

    thread_local! {
        static PREV_RANK: Cell<CardRank> = const { Cell::new(CardRank::InvalidRank) };
    }

    fn duplicated_rank_remover(_len: u64, pos: u64, c: &Option<Card>) -> ItrAction {
        let curr_rank = match c {
            Some(card) => card.get_card_rank(),
            None => CardRank::InvalidRank,
        };

        if pos == 0 {
            PREV_RANK.with(|r| r.set(curr_rank));
            return ItrAction::Continue;
        }

        let prev = PREV_RANK.with(|r| r.get());
        if prev == curr_rank {
            return ItrAction::RemoveAndContinue;
        }

        PREV_RANK.with(|r| r.set(curr_rank));
        ItrAction::Continue
    }

    fn length_trimmer(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
        if pos >= 5 {
            ItrAction::RemoveAndContinue
        } else {
            ItrAction::Continue
        }
    }

    fn get_razz_rank(hand: &mut CardHand) -> CardRank {
        // Reset the static prev_rank tracker for this new iteration.
        PREV_RANK.with(|r| r.set(CardRank::InvalidRank));
        hand.iterate_hand(duplicated_rank_remover);

        let cards_count = hand.count_cards_in_hand();
        if cards_count < 5 {
            return CardRank::InvalidRank;
        }

        hand.iterate_hand(length_trimmer);
        hand.get_max_rank_of_hand()
    }

    pub fn simulate_razz_game<T>(
        decided_cards: &DecidedCards,
        game_count: u64,
        arg: &mut T,
        listener: RankListener<T>,
    ) -> i32 {
        let mut my_hand = match CardHand::create_hand(RAZZ_CARD_IN_HAND_COUNT, sort_card_by_rank) {
            Some(h) => h,
            None => return 1,
        };

        for _ in 0..game_count {
            let mut deck = match CardDeck::create_shuffled_deck() {
                Some(d) => d,
                None => return 1,
            };
            strip_deck(&mut deck, decided_cards);

            complete_hand(&mut my_hand, decided_cards, &mut deck);
            let r = get_razz_rank(&mut my_hand);
            listener(arg, r);

            my_hand.reset_hand();
        }

        0
    }

    pub fn strip_deck(deck: &mut CardDeck, decided_cards: &DecidedCards) {
        let end = decided_cards.my_card_count as usize;
        for i in 0..end {
            if let Some(card) = decided_cards.my_cards[i] {
                let csr = card.get_card_suit_rank();
                if csr != CardSuitRank::InvalidCard {
                    deck.strip_card_from_deck(csr);
                }
            }
        }

        let end = decided_cards.opponent_card_count as usize;
        for i in 0..end {
            if let Some(card) = decided_cards.opponent_cards[i] {
                let csr = card.get_card_suit_rank();
                if csr != CardSuitRank::InvalidCard {
                    deck.strip_card_from_deck(csr);
                }
            }
        }
    }

    pub fn complete_hand(my_hand: &mut CardHand, decided_cards: &DecidedCards, deck: &mut CardDeck) {
        let end = decided_cards.my_card_count as usize;
        for i in 0..end {
            my_hand.insert_into_hand(&decided_cards.my_cards[i]);
        }

        let remaining = (RAZZ_CARD_IN_HAND_COUNT as usize).saturating_sub(end);
        for _ in 0..remaining {
            let dealt = deck.deal_from_deck();
            my_hand.insert_into_hand(&dealt);
        }
    }
}
