use crate::card::card::{Card, CardDeck, CardHand, CardRank, ItrAction, sort_card_by_rank};

pub mod razz_simulation {
    use super::*;
    use std::cell::RefCell;

    /// Number of cards each person is dealt in one round of Razz game.
    const RAZZ_CARD_IN_HAND_COUNT: u8 = 7;

    pub struct DecidedCards {
        pub my_card_count: u8,
        pub my_cards: [Option<Card>; 3],
        pub opponent_card_count: u8,
        pub opponent_cards: [Option<Card>; 7],
    }

    pub type RankListener<T> = fn(&mut T, CardRank);

    thread_local! {
        static PREV_RANK: RefCell<CardRank> = RefCell::new(CardRank::InvalidRank);
    }

    fn duplicated_rank_remover(_len: u64, pos: u64, c: &Option<Card>) -> ItrAction {
        let curr_rank = match c {
            Some(card) => card.get_card_rank(),
            None => return ItrAction::Continue,
        };
        if pos == 0 {
            PREV_RANK.with(|p| *p.borrow_mut() = curr_rank);
            return ItrAction::Continue;
        }
        let prev = PREV_RANK.with(|p| *p.borrow());
        if prev == curr_rank {
            return ItrAction::RemoveAndContinue;
        }
        PREV_RANK.with(|p| *p.borrow_mut() = curr_rank);
        ItrAction::Continue
    }

    fn length_trimmer(_len: u64, pos: u64, _c: &Option<Card>) -> ItrAction {
        if pos >= 5 {
            return ItrAction::RemoveAndContinue;
        }
        ItrAction::Continue
    }

    /// Determines the Razz rank of a hand.
    fn get_razz_rank(hand: &mut CardHand) -> CardRank {
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
            None => {
                eprintln!("Cannot create a hand");
                return 1;
            }
        };

        for _ in 0..game_count {
            let mut deck = match CardDeck::create_shuffled_deck() {
                Some(d) => d,
                None => {
                    eprintln!("Cannot create a shuffled deck");
                    return 1;
                }
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
            if let Some(c) = &decided_cards.my_cards[i] {
                deck.strip_card_from_deck(c.get_card_suit_rank());
            }
        }

        let end = decided_cards.opponent_card_count as usize;
        for i in 0..end {
            if let Some(c) = &decided_cards.opponent_cards[i] {
                deck.strip_card_from_deck(c.get_card_suit_rank());
            }
        }
    }

    pub fn complete_hand(
        my_hand: &mut CardHand,
        decided_cards: &DecidedCards,
        deck: &mut CardDeck,
    ) {
        let mut end = decided_cards.my_card_count as usize;
        for i in 0..end {
            my_hand.insert_into_hand(&decided_cards.my_cards[i]);
        }

        end = (RAZZ_CARD_IN_HAND_COUNT as usize) - end;
        for _ in 0..end {
            let c = deck.deal_from_deck();
            my_hand.insert_into_hand(&c);
        }
    }
}
