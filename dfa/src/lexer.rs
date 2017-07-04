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

    pub fn state_accept(&self, index: usize) -> bool {
        if let Some(state) = self.states.get(index) {
            *state
        } else {
            false
        }
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

impl<T: Transitable + Debug> Dfa<T> {
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

    /// Check all non-deterministic transitions of `index` and organize them as:
    /// {
    ///     char1: {dest1, dest2},
    ///     char2: {dest4, dest1, dest3},
    ///     char3: {dest4, dest2}
    /// }
    pub fn ndt_of(&self, index: &usize) -> HashMap<T, HashSet<usize>> {
        let mut ndt = HashMap::new();

        for c in &self.alphabet {
            let mut multiple = HashSet::new();

            for t in self.transitions[index].iter() {
                if &t.0 == c {
                    multiple.insert(t.1.clone());
                }
            }

            if multiple.len() > 1 {
                ndt.insert(c.clone(), multiple);
            }
        }

        ndt
    }

    /// Check all non-deterministic states and map them to:
    /// state_index1 == dest1 (both are indexes of DFA)
    /// {
    ///     state_index1: {
    ///         char: {dest1, dest2},
    ///         char2: {dest1, dest2},
    ///     },
    ///     state_index2: {
    ///         char: {dest1, dest2}
    ///     },
    ///     state_indexX: ndt_of(state_indexX)
    /// }
    pub fn non_determinist_states(&self) -> HashMap<usize, HashMap<T, HashSet<usize>>> {
        let mut ndet = HashMap::new();

        for s in self.transitions.keys() {
            let ndt = self.ndt_of(s);

            if ndt.len() > 0 {
                ndet.insert(s.clone(), ndt);
            }
        }

        return ndet;
    }

    pub fn determinize(&mut self) {
        let non_deterministic = self.non_determinist_states();

        // (state index, {T: state index})
        for (s, by) in non_deterministic {
            // (c, T)
            for (c, to) in by.iter() {
                // Now only create an non-accept state
                let newstate = self.add_state(false);

                for t in to.iter() {
                    // Vec of non-det transitions
                    let mut ndtrans = Vec::new();

                    if let Some(ts) = self.transitions.get_mut(t) {
                        // The deterministic transitions in this state, will return to `ts` again
                        // after `drain`
                        let mut dets = HashSet::new();

                        for d in ts.drain() {
                            if d.0 == *c {
                                // Wipe out non-deterministic transitions to Vec
                                ndtrans.push(d);
                            } else {
                                // Hold deterministic ones
                                dets.insert(d);
                            }
                        }

                        // Put deterministic transitions back
                        mem::replace(ts, dets);
                    }

                    // In each ND-Transition, create a transition to the new state
                    self.create_transition_between(&t, &newstate, c.clone());

                    // Add relationed states transitions
                    let trans = {
                        let mut v = Vec::new();

                        for t in ndtrans.into_iter() {
                            let dummie = HashSet::new();
                            let sdest = self.transitions.get(&t.1).unwrap_or(&dummie);

                            for dt in sdest {
                                if dt.0 == t.0 {
                                    v.push(dt.1);
                                }
                            }
                        }

                        v
                    };

                    for dt in &trans {
                        self.create_transition_between(&newstate, &dt, c.clone());
                    }
                }
            }
        }
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
