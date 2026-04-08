use cfsm::cfsm::{CfsmCtx, CFSM_VER_MAJOR, CFSM_VER_MINOR, CFSM_VER_PATCH};
use std::cell::Cell;

/// Counters for tracking state operation calls, mirroring the C test's StateOperationCounter.
struct Counters {
    enter: Cell<i32>,
    leave: Cell<i32>,
    process: Cell<i32>,
    event: Cell<i32>,
}

impl Counters {
    fn new() -> Self {
        Counters {
            enter: Cell::new(0),
            leave: Cell::new(0),
            process: Cell::new(0),
            event: Cell::new(0),
        }
    }
}

// We use two static counters for state A and state B, matching the C test pattern.
// Thread-local to avoid test interference.
thread_local! {
    static STATE_A: Counters = Counters::new();
    static STATE_B: Counters = Counters::new();
}

fn reset_counters() {
    STATE_A.with(|c| { c.enter.set(0); c.leave.set(0); c.process.set(0); c.event.set(0); });
    STATE_B.with(|c| { c.enter.set(0); c.leave.set(0); c.process.set(0); c.event.set(0); });
}

// State A handlers
fn state_a_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    if let Some(f) = fsm {
        f.on_event = Some(state_a_event);
        f.on_leave = Some(state_a_leave);
        f.on_process = Some(state_a_process);
    }
    STATE_A.with(|c| c.enter.set(c.enter.get() + 1));
}

fn state_a_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    STATE_A.with(|c| c.leave.set(c.leave.get() + 1));
}

fn state_a_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    STATE_A.with(|c| c.process.set(c.process.get() + 1));
}

fn state_a_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, _event_id: i32) {
    STATE_A.with(|c| c.event.set(c.event.get() + 1));
}

// State B handlers
fn state_b_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    if let Some(f) = fsm {
        f.on_event = Some(state_b_event);
        f.on_leave = Some(state_b_leave);
        f.on_process = Some(state_b_process);
    }
    STATE_B.with(|c| c.enter.set(c.enter.get() + 1));
}

fn state_b_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    STATE_B.with(|c| c.leave.set(c.leave.get() + 1));
}

fn state_b_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    STATE_B.with(|c| c.process.set(c.process.get() + 1));
}

fn state_b_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, _event_id: i32) {
    STATE_B.with(|c| c.event.set(c.event.get() + 1));
}

// Enter-only state (sets no handlers)
fn state_only_enter(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {}

#[test]
fn test_version_constants() {
    assert_eq!(CFSM_VER_MAJOR, 0);
    assert_eq!(CFSM_VER_MINOR, 3);
    assert_eq!(CFSM_VER_PATCH, 0);
}

#[test]
fn test_init_clears_handlers() {
    let mut fsm = CfsmCtx::<u8>::new();
    // Set some handlers to non-None
    fsm.on_leave = Some(state_a_leave);
    fsm.on_process = Some(state_a_process);
    fsm.on_event = Some(state_a_event);

    fsm.cfsm_init(Some(Box::new(42u8)));

    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
    assert_eq!(*fsm.ctx_ptr.unwrap(), 42u8);
}

#[test]
fn test_init_with_none_instance_data() {
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    assert!(fsm.ctx_ptr.is_none());
    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
}

#[test]
fn test_init_is_safe_to_use() {
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    // Should not crash — process and event are no-ops when handlers are None
    fsm.cfsm_process();
    fsm.cfsm_event(0x12345678);
}

#[test]
fn test_transition_enter_only() {
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_only_enter));

    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
}

#[test]
fn test_transition_with_none_enter() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    // Transition to state A first
    fsm.cfsm_transition(Some(state_a_enter));
    STATE_A.with(|c| assert_eq!(c.enter.get(), 1));

    // Transition with None — should call onLeave, clear handlers, no enter
    fsm.cfsm_transition(None);
    STATE_A.with(|c| assert_eq!(c.leave.get(), 1));

    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
}

#[test]
fn test_process_delegates() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_a_enter));

    STATE_A.with(|c| {
        assert_eq!(c.enter.get(), 1);
        assert_eq!(c.leave.get(), 0);
        assert_eq!(c.process.get(), 0);
        assert_eq!(c.event.get(), 0);
    });

    for _ in 0..10 {
        fsm.cfsm_process();
    }

    STATE_A.with(|c| {
        assert_eq!(c.enter.get(), 1);
        assert_eq!(c.leave.get(), 0);
        assert_eq!(c.process.get(), 10);
        assert_eq!(c.event.get(), 0);
    });
}

#[test]
fn test_event_delegates() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    fsm.cfsm_transition(Some(state_a_enter));

    for i in 0..10 {
        fsm.cfsm_event(i);
    }

    STATE_A.with(|c| {
        assert_eq!(c.enter.get(), 1);
        assert_eq!(c.leave.get(), 0);
        assert_eq!(c.process.get(), 0);
        assert_eq!(c.event.get(), 10);
    });
}

#[test]
fn test_transition_a_b_a() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    // Transition to A
    fsm.cfsm_transition(Some(state_a_enter));

    assert!(fsm.on_event.is_some());
    assert!(fsm.on_process.is_some());
    assert!(fsm.on_leave.is_some());

    STATE_A.with(|c| {
        assert_eq!(c.enter.get(), 1);
        assert_eq!(c.leave.get(), 0);
        assert_eq!(c.process.get(), 0);
        assert_eq!(c.event.get(), 0);
    });
    STATE_B.with(|c| {
        assert_eq!(c.enter.get(), 0);
        assert_eq!(c.leave.get(), 0);
    });

    // Transition A → B
    fsm.cfsm_transition(Some(state_b_enter));

    STATE_A.with(|c| {
        assert_eq!(c.enter.get(), 1);
        assert_eq!(c.leave.get(), 1);
    });
    STATE_B.with(|c| {
        assert_eq!(c.enter.get(), 1);
        assert_eq!(c.leave.get(), 0);
    });

    // Transition B → A
    fsm.cfsm_transition(Some(state_a_enter));

    STATE_A.with(|c| {
        assert_eq!(c.enter.get(), 2);
        assert_eq!(c.leave.get(), 1);
        assert_eq!(c.process.get(), 0);
        assert_eq!(c.event.get(), 0);
    });
    STATE_B.with(|c| {
        assert_eq!(c.enter.get(), 1);
        assert_eq!(c.leave.get(), 1);
        assert_eq!(c.process.get(), 0);
        assert_eq!(c.event.get(), 0);
    });
}

#[test]
fn test_new_creates_empty_context() {
    let fsm = CfsmCtx::<u8>::new();
    assert!(fsm.ctx_ptr.is_none());
    assert!(fsm.on_leave.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_event.is_none());
}

fn main() {}
