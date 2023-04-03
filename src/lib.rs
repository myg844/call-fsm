extern crate alloc;
extern crate core;

use core::fmt::{Display, Formatter};

#[macro_export]
macro_rules! declare_data_type {
    ($dt:ty) => {
        type DataType = $dt;
    }
}

#[macro_export]
macro_rules! declare_state_machine {
    ($name:ident, $data: ident, $num_states:expr) => {
        let mut $name: StateMachine<DataType> = StateMachine::new($data, $num_states);
    }
}

#[macro_export]
macro_rules! new_state {
    ($sm:ident, $name:ident, $init:expr, $exec:expr) => {
        let $name: State<DataType> = State::new(
            stringify!($name),
            $init,
            $exec);
        let $name = $sm.add_state($name).expect("Failed to add state");
    }
}

#[macro_export]
macro_rules! new_transition {
    ($sm:ident, $src:ident, $dst: ident, $check:expr, $done:expr) => {
        let _t: Transition<DataType> = Transition::new(
            concat!(stringify!($src), "__", stringify!($dst)),
            $src,
            $dst,
            $check,
            $done);
        $sm.add_transition(_t, $src, $dst).expect("Failed to add transition");
    }
}

pub type FsmResult = Result<(), FsmError>;

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum FsmError {
    StateIndexOutOfBounds,
    TransitionIndexOutOfBounds,
    MaxNumberOfStatesExceeded,
    AddTransitionSrcDstStatesEqual,
    StateIsEmpty,
    TransitionIsEmpty,
}

impl Display for FsmError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub type StateCallback<T> = dyn Fn(&State<T>, &mut T) -> Result<(), FsmError>;
pub type TransCheckCallback<T> = dyn Fn(&Transition<T>, &T) -> bool;
pub type TransDoneCallback<T> = dyn Fn(&Transition<T>, &mut T) -> Result<(), FsmError>;
pub type ErrorCallback<T> = dyn Fn(FsmError, &mut T) -> Option<Destination>;

pub enum Destination {
    Index(usize),
    Name(String),
}

pub struct StateMachine<T: 'static + Clone> {
    data: T,

    states: Vec<Option<State<T>>>,
    num_states: usize,

    transitions: Vec<Vec<Option<Transition<T>>>>,
    active_state: Option<usize>,
    active_state_initialized: bool,

    error: Option<(&'static ErrorCallback<T>, &'static ErrorCallback<T>)>,
}

impl<T: Clone> StateMachine<T> {
    pub fn new(data: T, max_states: usize) -> StateMachine<T> {
        StateMachine {
            data,
            states: vec![None; max_states],
            num_states: 0,
            transitions: vec![vec![None; max_states]; max_states],
            active_state: None,
            active_state_initialized: false,
            error: None
        }
    }

    pub fn state(&self, index: usize) -> Result<&State<T>, FsmError> {
        if index >= self.num_states {
            Err(FsmError::StateIndexOutOfBounds)
        } else if let Some(ref state) = self.states[index] {
            Ok(state)
        } else {
            Err(FsmError::StateIsEmpty)
        }
    }

    pub fn state_by_name(&self, name: String) -> Option<usize> {
        for (i, s) in self.states.iter().enumerate() {
            if let Some(state) = s {
                if name == state.name {
                    return Some(i);
                }
            }
        }
        None
    }

    pub fn mut_state(&mut self, index: usize) -> Result<&mut State<T>, FsmError> {
        if index >= self.num_states {
            Err(FsmError::StateIndexOutOfBounds)
        } else if let Some(ref mut state) = self.states[index] {
            Ok(state)
        } else {
            Err(FsmError::StateIsEmpty)
        }
    }

    pub fn transition(&self, src: usize, dst: usize) -> Result<&Transition<T>, FsmError> {
        if src >= self.num_states || dst >= self.num_states {
            Err(FsmError::TransitionIndexOutOfBounds)
        } else if let Some(ref transition) = self.transitions[src][dst] {
            Ok(transition)
        } else {
            Err(FsmError::TransitionIsEmpty)
        }
    }

    pub fn active_transitions(&self, src: usize) -> Result<&[Option<Transition<T>>], FsmError> {
        if src >= self.num_states {
            Err(FsmError::TransitionIndexOutOfBounds)
        } else {
            Ok(&self.transitions[src][..])
        }
    }

    pub fn add_state(&mut self, s: State<T>) -> Result<usize, FsmError> {
        if self.num_states >= self.states.capacity() {
            Err(FsmError::MaxNumberOfStatesExceeded)
        } else {
            self.states[self.num_states] = Some(s);
            let index = self.num_states;
            self.num_states += 1;
            Ok(index)
        }
    }

    pub fn add_transition(&mut self, t: Transition<T>, src: usize, dst: usize) -> Result<(), FsmError>{
        if src >= self.num_states || dst >= self.num_states {
            Err(FsmError::TransitionIndexOutOfBounds)
        } else if src == dst {
            Err(FsmError::AddTransitionSrcDstStatesEqual)
        } else {
            self.transitions[src][dst] = Some(t);
            Ok(())
        }
    }

    pub fn set_active_state(&mut self, s: usize) -> Result<(), FsmError> {
        match self.state(s) {
            Ok(_) => {
                self.active_state = Some(s);
                Ok(())
            },
            Err(e) => Err(e),
        }

    }

    pub fn set_error_callbacks(&mut self, init: &'static ErrorCallback<T>, exec: &'static ErrorCallback<T>) {
        self.error = Some((init, exec))
    }

    pub fn run(&mut self) {
        if let Some(active_state_index) = self.active_state {
            let active_state = self.state(active_state_index).expect("Failed to acquire active state").to_owned();

            // Initialize state if needed
            if !&self.active_state_initialized {
                if let Err(e) = active_state.do_init(&mut self.data) {
                    self.do_error_callback(e);
                    return;
                }
            }

            self.active_state_initialized = true;

            if let Err(e) = active_state.do_exec(&mut self.data) {
                self.do_error_callback(e);
                return;
            }

            let mut next_state_index = active_state_index;
            let next_state_trans = self.active_transitions(active_state_index).expect("Failed to acquire active transitions");
            let mut check = false;

            // Check transitions
            for t in next_state_trans {
                if let Some(transition) = t {
                    let transition = transition.to_owned();
                    check = transition.do_check(&self.data);
                    if check {
                        next_state_index = transition.dst;
                        match transition.do_done(&mut self.data) {
                            Err(e) => {
                                self.do_error_callback(e);
                                return;
                            },
                            Ok(_) => break
                        }
                    }
                }
            }

            if !check {
                // No transition check returned true, stay in the same active state
                return;
            }

            // Some transition check returned true, move to dst state
            self.active_state = Some(next_state_index);
            self.active_state_initialized = false;
        }

        // for s in &mut self.states {
        //     if let Some(state) = s {
        //         state.do_init().unwrap();
        //         state.do_exec().unwrap();
        //     }
        // }
        // for trans_src in &self.transitions {
        //     for trans_dst in trans_src {
        //         if let Some(trans) = trans_dst {
        //             trans.do_check();
        //             trans.do_done().unwrap();
        //         }
        //     }
        // }
    }

    fn do_error_callback(&mut self, error: FsmError) {
        println!("Error state: {}", error);
        if let Some((callback_init, callback_exec)) = self.error {
            callback_init(error, &mut self.data);
            if let Some(next_state) = callback_exec(error, &mut self.data) {
                match next_state {
                    Destination::Index(next_state_index) => {
                        if next_state_index < self.num_states {
                            self.active_state = Some(next_state_index);
                            self.active_state_initialized = false;
                        }
                    },
                    Destination::Name(next_state_name) => {
                        if let Some(next_state_index) = self.state_by_name(next_state_name) {
                            self.active_state = Some(next_state_index);
                            self.active_state_initialized = false;
                        }
                    }
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct State<T: 'static> {
    pub name: String,
    pub init: &'static StateCallback<T>,
    pub exec: &'static StateCallback<T>,
}

impl<T> State<T> {
    pub fn new<'b>(name: impl Into<alloc::borrow::Cow<'b, str>>,
                   init: &'static StateCallback<T>,
                   exec: &'static StateCallback<T>
    ) -> State<T> {
        State { name: name.into().into_owned(), init, exec }
    }

    pub fn do_init(&self, data: &mut T) -> Result<(), FsmError> {
        (self.init)(self, data)
    }

    pub fn do_exec(&self, data: &mut T) -> Result<(), FsmError> {
        (self.exec)(self, data)
    }
}

#[derive(Clone)]
pub struct Transition<T: 'static + Clone> {
    pub name: String,
    pub src: usize,
    pub dst: usize,
    pub check: &'static TransCheckCallback<T>,
    pub done: &'static TransDoneCallback<T>,
}

impl<T: Clone> Transition<T> {
    pub fn new<'b>(name: impl Into<alloc::borrow::Cow<'b, str>>,
                   src: usize,
                   dst: usize,
                   check: &'static TransCheckCallback<T>,
                   done: &'static TransDoneCallback<T>
    ) -> Transition<T> {
        Transition {
            name: name.into().into_owned(),
            src, dst,
            check, done }
    }

    pub fn do_check(&self, data: &T) -> bool {
        (self.check)(self, data)
    }

    pub fn do_done(&self, data: &mut T) -> Result<(), FsmError> {
        (self.done)(self, data)
    }
}