use razz_simulation::card::card::{Card, CardDeck, CardHand, CardRank, CardSuitRank, sort_card_by_rank};
use razz_simulation::razz_simulation::razz_simulation::{
    complete_hand, simulate_razz_game, strip_deck, DecidedCards,
};

fn make_decided_cards_one_my_card() -> DecidedCards {
    DecidedCards {
        my_card_count: 1,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            None,
            None,
        ],
        opponent_card_count: 1,
        opponent_cards: [
            Card::create_card(CardSuitRank::HeartK),
            None, None, None, None, None, None,
        ],
    }
}

#[test]
fn test_strip_deck_removes_decided() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    let dc = make_decided_cards_one_my_card();
    strip_deck(&mut d, &dc);
    assert_eq!(d.is_card_in_deck(CardSuitRank::SpadeAce), 0);
    assert_eq!(d.is_card_in_deck(CardSuitRank::HeartK), 0);
    // Other cards still there
    assert_ne!(d.is_card_in_deck(CardSuitRank::Diamond5), 0);
}

#[test]
fn test_complete_hand_fills_to_seven() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Heart2),
            Card::create_card(CardSuitRank::Diamond3),
        ],
        opponent_card_count: 0,
        opponent_cards: [None; 7],
    };
    // Strip my cards from deck before completing
    strip_deck(&mut deck, &dc);
    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    complete_hand(&mut hand, &dc, &mut deck);
    assert_eq!(hand.count_cards_in_hand(), 7);
    assert_eq!(hand.get_max_of_hand(), 7);
}

#[test]
fn test_complete_hand_zero_decided() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let dc = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None; 7],
    };
    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    complete_hand(&mut hand, &dc, &mut deck);
    assert_eq!(hand.count_cards_in_hand(), 7);
}

#[test]
fn test_simulate_razz_game_returns_zero() {
    let dc = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Heart2),
            Card::create_card(CardSuitRank::Diamond3),
        ],
        opponent_card_count: 0,
        opponent_cards: [None; 7],
    };

    let mut count: u64 = 0;
    fn listener(arg: &mut u64, _r: CardRank) {
        *arg += 1;
    }
    let result = simulate_razz_game(&dc, 100, &mut count, listener);
    assert_eq!(result, 0);
    assert_eq!(count, 100);
}

#[test]
fn test_simulate_razz_game_listener_called_once_per_game() {
    let dc = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None; 7],
    };
    let mut count: u64 = 0;
    fn listener(arg: &mut u64, _r: CardRank) {
        *arg += 1;
    }
    let result = simulate_razz_game(&dc, 50, &mut count, listener);
    assert_eq!(result, 0);
    assert_eq!(count, 50);
}

#[test]
fn test_simulate_razz_game_zero_count() {
    let dc = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None; 7],
    };
    let mut count: u64 = 0;
    fn listener(arg: &mut u64, _r: CardRank) {
        *arg += 1;
    }
    let result = simulate_razz_game(&dc, 0, &mut count, listener);
    assert_eq!(result, 0);
    assert_eq!(count, 0);
}

#[test]
fn test_simulate_razz_rank_in_valid_range() {
    // For any non-Invalid rank, should be between R5 and K (per game logic)
    let dc = DecidedCards {
        my_card_count: 0,
        my_cards: [None, None, None],
        opponent_card_count: 0,
        opponent_cards: [None; 7],
    };
    struct Bucket {
        valid: bool,
    }
    let mut bucket = Bucket { valid: true };
    fn listener(arg: &mut Bucket, r: CardRank) {
        if r != CardRank::InvalidRank {
            // Razz rank must be R5..=K
            if !(r >= CardRank::R5 && r <= CardRank::K) {
                arg.valid = false;
            }
        }
    }
    let result = simulate_razz_game(&dc, 200, &mut bucket, listener);
    assert_eq!(result, 0);
    assert!(bucket.valid);
}

#[test]
fn test_strip_deck_with_opponent_cards() {
    let mut d = CardDeck::create_shuffled_deck().unwrap();
    let dc = DecidedCards {
        my_card_count: 2,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Heart2),
            None,
        ],
        opponent_card_count: 3,
        opponent_cards: [
            Card::create_card(CardSuitRank::Club5),
            Card::create_card(CardSuitRank::Diamond6),
            Card::create_card(CardSuitRank::SpadeK),
            None, None, None, None,
        ],
    };
    strip_deck(&mut d, &dc);
    assert_eq!(d.is_card_in_deck(CardSuitRank::SpadeAce), 0);
    assert_eq!(d.is_card_in_deck(CardSuitRank::Heart2), 0);
    assert_eq!(d.is_card_in_deck(CardSuitRank::Club5), 0);
    assert_eq!(d.is_card_in_deck(CardSuitRank::Diamond6), 0);
    assert_eq!(d.is_card_in_deck(CardSuitRank::SpadeK), 0);
    // Untouched
    assert_ne!(d.is_card_in_deck(CardSuitRank::Heart3), 0);
}

fn main() {}
