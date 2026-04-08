use cfsm::cfsm::{CfsmCtx, CFSM_VER_MAJOR, CFSM_VER_MINOR, CFSM_VER_PATCH};
use std::cell::Cell;

thread_local! {
    static SA_ENTER: Cell<i32> = Cell::new(0);
    static SA_LEAVE: Cell<i32> = Cell::new(0);
    static SA_PROCESS: Cell<i32> = Cell::new(0);
    static SA_EVENT: Cell<i32> = Cell::new(0);
    static SA_LAST_EVENT: Cell<i32> = Cell::new(0);
    static SB_ENTER: Cell<i32> = Cell::new(0);
    static SB_LEAVE: Cell<i32> = Cell::new(0);
    static SB_PROCESS: Cell<i32> = Cell::new(0);
    static SB_EVENT: Cell<i32> = Cell::new(0);
}

fn reset_counters() {
    SA_ENTER.with(|c| c.set(0));
    SA_LEAVE.with(|c| c.set(0));
    SA_PROCESS.with(|c| c.set(0));
    SA_EVENT.with(|c| c.set(0));
    SA_LAST_EVENT.with(|c| c.set(0));
    SB_ENTER.with(|c| c.set(0));
    SB_LEAVE.with(|c| c.set(0));
    SB_PROCESS.with(|c| c.set(0));
    SB_EVENT.with(|c| c.set(0));
}

fn state_a_on_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    let fsm = *fsm.unwrap();
    fsm.on_event = Some(state_a_on_event);
    fsm.on_leave = Some(state_a_on_leave);
    fsm.on_process = Some(state_a_on_process);
    SA_ENTER.with(|c| c.set(c.get() + 1));
}

fn state_a_on_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    SA_LEAVE.with(|c| c.set(c.get() + 1));
}

fn state_a_on_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    SA_PROCESS.with(|c| c.set(c.get() + 1));
}

fn state_a_on_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, event_id: i32) {
    SA_EVENT.with(|c| c.set(c.get() + 1));
    SA_LAST_EVENT.with(|c| c.set(event_id));
}

fn state_b_on_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    let fsm = *fsm.unwrap();
    fsm.on_event = Some(state_b_on_event);
    fsm.on_leave = Some(state_b_on_leave);
    fsm.on_process = Some(state_b_on_process);
    SB_ENTER.with(|c| c.set(c.get() + 1));
}

fn state_b_on_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    SB_LEAVE.with(|c| c.set(c.get() + 1));
}

fn state_b_on_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    SB_PROCESS.with(|c| c.set(c.get() + 1));
}

fn state_b_on_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, _event_id: i32) {
    SB_EVENT.with(|c| c.set(c.get() + 1));
}

fn state_only_on_enter(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    // Does nothing — enter-only state
}

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
    fsm.on_event = Some(state_a_on_event);
    fsm.on_process = Some(state_a_on_process);
    fsm.on_leave = Some(state_a_on_leave);

    fsm.cfsm_init(Some(Box::new(42u8)));

    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
    assert!(fsm.ctx_ptr.is_some());
    assert_eq!(**fsm.ctx_ptr.as_ref().unwrap(), 42u8);
}

#[test]
fn test_init_safe_to_use() {
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);
    // These should not panic
    fsm.cfsm_process();
    fsm.cfsm_event(0x12345678);
}

#[test]
fn test_transition_enter_only() {
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);
    fsm.cfsm_transition(Some(state_only_on_enter));
    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
}

#[test]
fn test_transition_to_null() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);
    fsm.cfsm_transition(Some(state_a_on_enter));
    // Reset counters after enter
    SA_ENTER.with(|c| c.set(0));

    fsm.cfsm_transition(None);

    // Leave should have been called once
    assert_eq!(SA_LEAVE.with(|c| c.get()), 1);
    // All handlers cleared
    assert!(fsm.on_event.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_leave.is_none());
}

#[test]
fn test_process_counting() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);
    fsm.cfsm_transition(Some(state_a_on_enter));

    assert_eq!(SA_ENTER.with(|c| c.get()), 1);
    assert_eq!(SA_LEAVE.with(|c| c.get()), 0);
    assert_eq!(SA_PROCESS.with(|c| c.get()), 0);
    assert_eq!(SA_EVENT.with(|c| c.get()), 0);

    for _ in 0..10 {
        fsm.cfsm_process();
    }

    assert_eq!(SA_ENTER.with(|c| c.get()), 1);
    assert_eq!(SA_LEAVE.with(|c| c.get()), 0);
    assert_eq!(SA_PROCESS.with(|c| c.get()), 10);
    assert_eq!(SA_EVENT.with(|c| c.get()), 0);
}

#[test]
fn test_event_counting() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);
    fsm.cfsm_transition(Some(state_a_on_enter));

    assert_eq!(SA_ENTER.with(|c| c.get()), 1);
    assert_eq!(SA_LEAVE.with(|c| c.get()), 0);
    assert_eq!(SA_PROCESS.with(|c| c.get()), 0);
    assert_eq!(SA_EVENT.with(|c| c.get()), 0);

    for i in 0..10 {
        fsm.cfsm_event(i);
    }

    assert_eq!(SA_ENTER.with(|c| c.get()), 1);
    assert_eq!(SA_LEAVE.with(|c| c.get()), 0);
    assert_eq!(SA_PROCESS.with(|c| c.get()), 0);
    assert_eq!(SA_EVENT.with(|c| c.get()), 10);
    assert_eq!(SA_LAST_EVENT.with(|c| c.get()), 9);
}

#[test]
fn test_transition_a_b_a() {
    reset_counters();
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(None);

    // Transition to A
    fsm.cfsm_transition(Some(state_a_on_enter));
    assert_eq!(SA_ENTER.with(|c| c.get()), 1);
    assert_eq!(SA_LEAVE.with(|c| c.get()), 0);
    assert_eq!(SB_ENTER.with(|c| c.get()), 0);
    assert_eq!(SB_LEAVE.with(|c| c.get()), 0);

    // Transition to B (should call A's leave, then B's enter)
    fsm.cfsm_transition(Some(state_b_on_enter));
    assert_eq!(SA_ENTER.with(|c| c.get()), 1);
    assert_eq!(SA_LEAVE.with(|c| c.get()), 1);
    assert_eq!(SB_ENTER.with(|c| c.get()), 1);
    assert_eq!(SB_LEAVE.with(|c| c.get()), 0);

    // Transition back to A (should call B's leave, then A's enter)
    fsm.cfsm_transition(Some(state_a_on_enter));
    assert_eq!(SA_ENTER.with(|c| c.get()), 2);
    assert_eq!(SA_LEAVE.with(|c| c.get()), 1);
    assert_eq!(SB_ENTER.with(|c| c.get()), 1);
    assert_eq!(SB_LEAVE.with(|c| c.get()), 1);
}

#[test]
fn test_new_creates_empty_context() {
    let fsm = CfsmCtx::<u8>::new();
    assert!(fsm.ctx_ptr.is_none());
    assert!(fsm.on_leave.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_event.is_none());
}

#[test]
fn test_init_with_instance_data() {
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(Some(Box::new(99u8)));
    assert_eq!(**fsm.ctx_ptr.as_ref().unwrap(), 99u8);
    assert!(fsm.on_leave.is_none());
    assert!(fsm.on_process.is_none());
    assert!(fsm.on_event.is_none());
}

#[test]
fn test_init_with_none_clears_ctx_ptr() {
    let mut fsm = CfsmCtx::<u8>::new();
    fsm.cfsm_init(Some(Box::new(99u8)));
    fsm.cfsm_init(None);
    assert!(fsm.ctx_ptr.is_none());
}

fn main() {}
