use cfsm::cfsm::{CfsmCtx, CFSM_VER_MAJOR, CFSM_VER_MINOR, CFSM_VER_PATCH};
use std::sync::atomic::{AtomicI32, Ordering};

// State A counters
static STATE_A_ENTER: AtomicI32 = AtomicI32::new(0);
static STATE_A_LEAVE: AtomicI32 = AtomicI32::new(0);
static STATE_A_PROCESS: AtomicI32 = AtomicI32::new(0);
static STATE_A_EVENT: AtomicI32 = AtomicI32::new(0);
static STATE_A_LAST_EVENT: AtomicI32 = AtomicI32::new(0);

// State B counters
static STATE_B_ENTER: AtomicI32 = AtomicI32::new(0);
static STATE_B_LEAVE: AtomicI32 = AtomicI32::new(0);
static STATE_B_PROCESS: AtomicI32 = AtomicI32::new(0);
static STATE_B_EVENT: AtomicI32 = AtomicI32::new(0);

// Plain enter that sets no handlers
static PLAIN_ENTER_CALLS: AtomicI32 = AtomicI32::new(0);

fn reset_all() {
    STATE_A_ENTER.store(0, Ordering::SeqCst);
    STATE_A_LEAVE.store(0, Ordering::SeqCst);
    STATE_A_PROCESS.store(0, Ordering::SeqCst);
    STATE_A_EVENT.store(0, Ordering::SeqCst);
    STATE_A_LAST_EVENT.store(0, Ordering::SeqCst);
    STATE_B_ENTER.store(0, Ordering::SeqCst);
    STATE_B_LEAVE.store(0, Ordering::SeqCst);
    STATE_B_PROCESS.store(0, Ordering::SeqCst);
    STATE_B_EVENT.store(0, Ordering::SeqCst);
    PLAIN_ENTER_CALLS.store(0, Ordering::SeqCst);
}

// =========== State A handlers ===========
fn state_a_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    if let Some(mut boxed) = fsm {
        boxed.on_event = Some(state_a_event);
        boxed.on_leave = Some(state_a_leave);
        boxed.on_process = Some(state_a_process);
    }
    STATE_A_ENTER.fetch_add(1, Ordering::SeqCst);
}

fn state_a_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, event_id: i32) {
    STATE_A_EVENT.fetch_add(1, Ordering::SeqCst);
    STATE_A_LAST_EVENT.store(event_id, Ordering::SeqCst);
}

fn state_a_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    // Match the C test: lastEventId is incremented (set to current process count - 1)
    // After ith process call (1-indexed i), processCalls becomes i, lastEventId becomes i-1
    // Wait - the C test asserts state_A.lastEventId == i (0..10) AFTER cfsm_process is called.
    // i goes from 0..10. After cfsm_process inside loop iteration i, expects lastEventId == i.
    // So in the process function, lastEventId is set to processCalls (current count, incrementing first).
    let prev = STATE_A_PROCESS.fetch_add(1, Ordering::SeqCst);
    STATE_A_LAST_EVENT.store(prev, Ordering::SeqCst);
}

fn state_a_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    STATE_A_LEAVE.fetch_add(1, Ordering::SeqCst);
}

// =========== State B handlers ===========
fn state_b_enter(fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    if let Some(mut boxed) = fsm {
        boxed.on_event = Some(state_b_event);
        boxed.on_leave = Some(state_b_leave);
        boxed.on_process = Some(state_b_process);
    }
    STATE_B_ENTER.fetch_add(1, Ordering::SeqCst);
}

fn state_b_event(_fsm: Option<Box<&mut CfsmCtx<u8>>>, _event_id: i32) {
    STATE_B_EVENT.fetch_add(1, Ordering::SeqCst);
}

fn state_b_process(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    STATE_B_PROCESS.fetch_add(1, Ordering::SeqCst);
}

fn state_b_leave(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    STATE_B_LEAVE.fetch_add(1, Ordering::SeqCst);
}

// Plain enter func — sets no handlers
fn state_only_on_enter(_fsm: Option<Box<&mut CfsmCtx<u8>>>) {
    PLAIN_ENTER_CALLS.fetch_add(1, Ordering::SeqCst);
}

fn fn_addr<T>(f: Option<fn(Option<Box<&mut CfsmCtx<T>>>)>) -> usize {
    f.map(|p| p as usize).unwrap_or(0)
}

fn fn_addr_event<T>(f: Option<fn(Option<Box<&mut CfsmCtx<T>>>, i32)>) -> usize {
    f.map(|p| p as usize).unwrap_or(0)
}

#[test]
fn test_cfsm_version() {
    // Match the Rust constant values (header has 0.3.0)
    assert_eq!(CFSM_VER_MAJOR, 0);
    assert_eq!(CFSM_VER_MINOR, 3);
    assert_eq!(CFSM_VER_PATCH, 0);
}

#[test]
fn test_cfsm_new_initializes_all_none() {
    let ctx: CfsmCtx<u8> = CfsmCtx::new();
    assert!(ctx.ctx_ptr.is_none());
    assert!(ctx.on_leave.is_none());
    assert!(ctx.on_process.is_none());
    assert!(ctx.on_event.is_none());
}

#[test]
fn test_cfsm_init_should_clear_handler() {
    // Pre-populate handlers and instance data, then verify cfsm_init clears handlers
    // and updates instance data, mirroring C's test_cfsm_init_should_clear_handler.
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();

    // Pre-set handlers (simulating "corrupt content" in C with memset(-1))
    ctx.on_event = Some(state_a_event);
    ctx.on_leave = Some(state_a_leave);
    ctx.on_process = Some(state_a_process);
    ctx.ctx_ptr = Some(Box::new(0u8));

    // Reinit with new instance data (42)
    ctx.cfsm_init(Some(Box::new(42u8)));

    // All handlers should be cleared
    assert!(ctx.on_event.is_none());
    assert!(ctx.on_process.is_none());
    assert!(ctx.on_leave.is_none());

    // Instance data should be set to 42
    assert!(ctx.ctx_ptr.is_some());
    assert_eq!(*ctx.ctx_ptr.as_ref().unwrap().as_ref(), 42u8);
}

#[test]
fn test_cfsm_init_with_none_instance_data() {
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    // populate first
    ctx.ctx_ptr = Some(Box::new(99u8));
    ctx.on_event = Some(state_a_event);

    ctx.cfsm_init(None);

    assert!(ctx.ctx_ptr.is_none());
    assert!(ctx.on_event.is_none());
    assert!(ctx.on_leave.is_none());
    assert!(ctx.on_process.is_none());
}

#[test]
fn test_cfsm_init_is_safe_to_use() {
    // After init with no handlers, calling process and event should not crash.
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);

    // Should not crash:
    ctx.cfsm_process();
    ctx.cfsm_event(0x12345678);

    // No handlers should ever be set
    assert!(ctx.on_event.is_none());
    assert!(ctx.on_process.is_none());
    assert!(ctx.on_leave.is_none());
    assert!(ctx.ctx_ptr.is_none());
}

#[test]
fn test_cfsm_transition_should_set_enter_handler_only() {
    // Calling cfsm_transition with a function that sets no handlers should leave them all None.
    reset_all();
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);

    ctx.cfsm_transition(Some(state_only_on_enter));

    assert!(ctx.on_event.is_none());
    assert!(ctx.on_process.is_none());
    assert!(ctx.on_leave.is_none());
    assert_eq!(PLAIN_ENTER_CALLS.load(Ordering::SeqCst), 1);
}

#[test]
fn test_cfsm_transition_with_none_clears_handlers() {
    // Passing None as enterFunc should run leave handler then clear all handlers
    // (mirrors the documented behavior in C).
    reset_all();
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);

    // Enter state A first
    ctx.cfsm_transition(Some(state_a_enter));
    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 1);
    assert!(ctx.on_event.is_some());
    assert!(ctx.on_process.is_some());
    assert!(ctx.on_leave.is_some());

    // Now transition to None — should call leave and clear handlers
    ctx.cfsm_transition(None);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 1);
    assert!(ctx.on_event.is_none());
    assert!(ctx.on_process.is_none());
    assert!(ctx.on_leave.is_none());
}

#[test]
fn test_cfsm_process_calls_handler() {
    // Mirrors C test_cfs_process: enter A, then 10 process cycles
    reset_all();
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);

    ctx.cfsm_transition(Some(state_a_enter));

    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_LAST_EVENT.load(Ordering::SeqCst), 0);

    for i in 0..10 {
        ctx.cfsm_process();
        assert_eq!(STATE_A_LAST_EVENT.load(Ordering::SeqCst), i);
    }
    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 10);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 0);
}

#[test]
fn test_cfsm_event_calls_handler() {
    // Mirrors C test_cfs_signalEvent
    reset_all();
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);

    ctx.cfsm_transition(Some(state_a_enter));
    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 0);

    for i in 0..10 {
        ctx.cfsm_event(i);
    }
    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 10);
    assert_eq!(STATE_A_LAST_EVENT.load(Ordering::SeqCst), 9);
}

#[test]
fn test_cfsm_transition_a_b_a() {
    // Mirrors C test_cfs_transition_A_B_A
    reset_all();
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);

    // Transition to A
    ctx.cfsm_transition(Some(state_a_enter));

    assert_eq!(fn_addr_event(ctx.on_event), state_a_event as usize);
    assert_eq!(fn_addr(ctx.on_process), state_a_process as usize);
    assert_eq!(fn_addr(ctx.on_leave), state_a_leave as usize);

    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 0);

    assert_eq!(STATE_B_ENTER.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_B_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_B_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_B_EVENT.load(Ordering::SeqCst), 0);

    // Transition to B
    ctx.cfsm_transition(Some(state_b_enter));

    assert_eq!(fn_addr_event(ctx.on_event), state_b_event as usize);
    assert_eq!(fn_addr(ctx.on_process), state_b_process as usize);
    assert_eq!(fn_addr(ctx.on_leave), state_b_leave as usize);

    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 0);

    assert_eq!(STATE_B_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_B_LEAVE.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_B_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_B_EVENT.load(Ordering::SeqCst), 0);

    // Transition back to A
    ctx.cfsm_transition(Some(state_a_enter));

    assert_eq!(fn_addr_event(ctx.on_event), state_a_event as usize);
    assert_eq!(fn_addr(ctx.on_process), state_a_process as usize);
    assert_eq!(fn_addr(ctx.on_leave), state_a_leave as usize);

    assert_eq!(STATE_A_ENTER.load(Ordering::SeqCst), 2);
    assert_eq!(STATE_A_LEAVE.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 0);

    assert_eq!(STATE_B_ENTER.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_B_LEAVE.load(Ordering::SeqCst), 1);
    assert_eq!(STATE_B_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_B_EVENT.load(Ordering::SeqCst), 0);
}

#[test]
fn test_cfsm_event_passes_event_id() {
    // Verify that the event id is passed unchanged to the handler
    reset_all();
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);
    ctx.cfsm_transition(Some(state_a_enter));

    ctx.cfsm_event(0x12345678);
    assert_eq!(STATE_A_LAST_EVENT.load(Ordering::SeqCst), 0x12345678);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 1);

    ctx.cfsm_event(-42);
    assert_eq!(STATE_A_LAST_EVENT.load(Ordering::SeqCst), -42);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 2);
}

#[test]
fn test_cfsm_process_no_handler_does_not_crash() {
    // After transition to a state that does not set on_process, process should be a no-op.
    reset_all();
    let mut ctx: CfsmCtx<u8> = CfsmCtx::new();
    ctx.cfsm_init(None);
    ctx.cfsm_transition(Some(state_only_on_enter));

    // No handlers were set — process and event should be silent
    ctx.cfsm_process();
    ctx.cfsm_event(7);

    assert_eq!(STATE_A_PROCESS.load(Ordering::SeqCst), 0);
    assert_eq!(STATE_A_EVENT.load(Ordering::SeqCst), 0);
}

fn main() {}
