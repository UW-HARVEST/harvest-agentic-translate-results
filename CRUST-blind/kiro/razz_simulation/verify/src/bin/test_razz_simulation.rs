use razz_simulation::card::card::*;
use razz_simulation::razz_simulation::razz_simulation::*;

#[test]
fn test_strip_deck() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let decided = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Heart5),
            Card::create_card(CardSuitRank::Diamond9),
        ],
        opponent_card_count: 2,
        opponent_cards: [
            Card::create_card(CardSuitRank::Club2),
            Card::create_card(CardSuitRank::HeartK),
            None, None, None, None, None,
        ],
    };
    strip_deck(&mut deck, &decided);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::SpadeAce), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Heart5), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Diamond9), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Club2), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartK), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Spade2), 1);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::ClubK), 1);
}

#[test]
fn test_strip_deck_no_opponents() {
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let decided = DecidedCards {
        my_card_count: 2,
        my_cards: [
            Card::create_card(CardSuitRank::HeartAce),
            Card::create_card(CardSuitRank::Heart2),
            None,
        ],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    strip_deck(&mut deck, &decided);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::HeartAce), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Heart2), 0);
    assert_eq!(deck.is_card_in_deck(CardSuitRank::Heart3), 1);
}

#[test]
fn test_complete_hand() {
    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let decided = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Heart5),
            Card::create_card(CardSuitRank::Diamond9),
        ],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    strip_deck(&mut deck, &decided);
    complete_hand(&mut hand, &decided, &mut deck);
    assert_eq!(hand.count_cards_in_hand(), 7);
    assert_eq!(hand.get_max_of_hand(), 7);
}

#[test]
fn test_simulate_razz_game_returns_zero() {
    let decided = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::SpadeAce),
            Card::create_card(CardSuitRank::Spade2),
            Card::create_card(CardSuitRank::Spade3),
        ],
        opponent_card_count: 1,
        opponent_cards: [
            Card::create_card(CardSuitRank::ClubK),
            None, None, None, None, None, None,
        ],
    };
    let mut ranks: Vec<CardRank> = Vec::new();
    let result = simulate_razz_game(&decided, 5, &mut ranks, |arg, r| {
        arg.push(r);
    });
    assert_eq!(result, 0);
    assert_eq!(ranks.len(), 5);
    for &r in &ranks {
        assert!(
            r == CardRank::InvalidRank || (r >= CardRank::R5 && r <= CardRank::K),
            "unexpected rank: {:?}", r
        );
    }
}

#[test]
fn test_complete_hand_decided_cards_present() {
    let mut hand = CardHand::create_hand(7, sort_card_by_rank).unwrap();
    let mut deck = CardDeck::create_shuffled_deck().unwrap();
    let decided = DecidedCards {
        my_card_count: 3,
        my_cards: [
            Card::create_card(CardSuitRank::Spade4),
            Card::create_card(CardSuitRank::Spade5),
            Card::create_card(CardSuitRank::Spade6),
        ],
        opponent_card_count: 0,
        opponent_cards: [None, None, None, None, None, None, None],
    };
    strip_deck(&mut deck, &decided);
    complete_hand(&mut hand, &decided, &mut deck);
    assert_eq!(hand.count_cards_in_hand(), 7);
}

fn main() {}
