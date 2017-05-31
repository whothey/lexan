use std::collections::{ HashSet, HashMap };
use std::hash::Hash;
use std::fmt::{ Display, Debug };
use std::mem;

pub trait Transitable: PartialEq + Eq + Hash + Clone {}
impl Transitable for char {}

/// State = true => State Accept
pub type State = bool;

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct Transition<T>(T, usize);

impl<T: Transitable> Transition<T> {
    pub fn new(by: T, dest: usize) -> Self {
        Transition(by, dest)
    }
}

pub struct Dfa<T> {
    states: Vec<State>,

    /// Index on `states` which is the initial state
    initial: usize,

    /// The current state DFA is into
    current: usize,

    transitions: HashMap<usize, HashSet<Transition<T>>>,
    alphabet: HashSet<T>
}

impl<T: Hash + Eq> Dfa<T> {
    /// Create a new Lexer with a initial state
    pub fn new() -> Self {
        Self {
            // Initial state is already created
            states: vec![false],
            alphabet: HashSet::new(),
            initial: 0,
            current: 0,
            transitions: HashMap::new()
        }
    }

    pub fn states(&self) -> &Vec<State> {
        &self.states
    }

    /// Add a new state and return its index
    pub fn add_state(&mut self, state: State) -> usize {
        self.states.push(state);
        self.states.len() - 1
    }

    pub fn set_initial(&mut self, i: usize) {
        self.initial = i;
    }

    pub fn initial(&self) -> usize {
        self.initial
    }

    pub fn rewind(&mut self) {
        self.current = self.initial;
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn set_current(&mut self, t: usize) -> Result<(), &str> {
        if t <= self.states.len() {
            self.current = t;
            Ok(())
        } else {
            Err("Non existant state")
        }
    }

    pub fn alphabet(&self) -> &HashSet<T> {
        &self.alphabet
    }

    pub fn transitions(&self) -> &HashMap<usize, HashSet<Transition<T>>> {
        &self.transitions
    }

    pub fn set_current_state_accept(&mut self, accept: bool) {
        mem::replace(self.states.get_mut(self.current).unwrap(), accept);
    }
}

impl<T: Transitable> Dfa<T> {
    /// Add a existing `Transition` to `state`
    pub fn add_transition_to(&mut self, state: &usize, trans: Transition<T>) {
        self.alphabet.insert(trans.0.clone());

        if self.transitions.contains_key(&state) {
            self.transitions.get_mut(&state).unwrap().insert(trans);
        } else {
            let mut set = HashSet::new();
            set.insert(trans);
            self.transitions.insert(*state, set);
        }
    }

    /// Create a transition between states `origin` and `dest`
    pub fn create_transition_between(&mut self, origin: &usize, dest: &usize, by: T) {
        let trans = Transition::new(by, *dest);

        self.add_transition_to(origin, trans)
    }

    /// Create a transition between the current state and `dest`
    pub fn create_transition(&mut self, by: T, dest: usize) {
        let current = self.current;
        self.create_transition_between(&current, &dest, by)
    }

    /// Create a transition between the current state and `dest` and set the current state to
    /// `dest`
    pub fn create_transition_and_walk(&mut self, by: T, dest: usize) {
        let current = self.current;
        self.create_transition_between(&current, &dest, by);
        self.current = dest;
    }
}

impl<T: Display + Debug + Eq + Hash> Dfa<T> {
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("State");

        // Header
        for a in &self.alphabet {
            csv += format!(",{}", a).as_str();
        }

        csv.push('\n');

        for (k, accept) in self.states().iter().enumerate() {
            if k == self.initial() { csv.push_str("->"); }
            if *accept { csv.push('*'); }

            csv += format!("<{}>", k).as_str();

            for a in &self.alphabet {
                match self.transitions.get(&k) {
                    Some(trans) => {
                        let mut has_states = false;

                        for t in trans {
                            if t.0 == *a {
                                // Controls the first comma
                                if ! has_states { csv.push(','); has_states = true; }
                                csv += format!("<{}>", t.1).as_str();
                            }
                        }

                        if ! has_states {
                            csv.push_str(",-");
                        }
                    },
                    None    => csv.push_str(",-")
                }
            }

            csv.push('\n');
        }

        csv
    }
}
