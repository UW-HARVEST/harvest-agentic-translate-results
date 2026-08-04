pub const CFSM_VER_MAJOR: usize = 0;
pub const CFSM_VER_MINOR: usize = 3;
pub const CFSM_VER_PATCH: usize = 0;
pub struct CfsmCtx<T>
{
    pub ctx_ptr: Option<Box<T>>,
    pub on_leave: Option<fn(fsm: Option<Box<&mut CfsmCtx<T>>>)>, 
    pub on_process: Option<fn(fsm: Option<Box<&mut CfsmCtx<T>>>)>,
    pub on_event: Option<fn(fsm: Option<Box<&mut CfsmCtx<T>>>, event_id: i32)>,
}
impl<T> CfsmCtx<T> {
    pub fn new() -> Self {
        Self {
            ctx_ptr: None,
            on_leave: None,
            on_process: None,
            on_event: None,
        }
    }
    pub fn cfsm_init(&mut self, instance_data: Option<Box<T>>) {
        self.ctx_ptr = instance_data;
        self.on_leave = None;
        self.on_process = None;
        self.on_event = None;
    }
    pub fn cfsm_transition(&mut self, enter_func: Option<fn(fsm: Option<Box<&mut CfsmCtx<T>>>)>) {
        if let Some(on_leave) = self.on_leave {
            on_leave(Some(Box::new(self)));
        }

        self.on_event = None;
        self.on_leave = None;
        self.on_process = None;

        if let Some(enter_func) = enter_func {
            enter_func(Some(Box::new(self)));
        }
    }
    pub fn cfsm_process(&mut self) {
        if let Some(on_process) = self.on_process {
            on_process(Some(Box::new(self)));
        }
    }
    pub fn cfsm_event(&mut self, event_id: i32) {
        if let Some(on_event) = self.on_event {
            on_event(Some(Box::new(self)), event_id);
        }
    }
}
