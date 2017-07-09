use std::collections::{ HashSet, HashMap, VecDeque };
use std::hash::Hash;
use std::fmt::{ Display, Debug };
use std::mem;

pub trait Transitable: PartialEq + Eq + Hash + Clone {}
impl Transitable for char {}

/// State = true => State Accept
pub type State = bool;

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct Transition<T>(T, usize);

impl<T: Transitable> Transition<T> {
    pub fn new(by: T, dest: usize) -> Self {
        Transition(by, dest)
    }
}

pub struct Dfa<T> {
    states: HashMap<usize, State>,

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
            states: {
                let mut hm = HashMap::new();
                hm.insert(0, false);

                hm
            },
            alphabet: HashSet::new(),
            initial: 0,
            current: 0,
            transitions: HashMap::new()
        }
    }

    pub fn states(&self) -> &HashMap<usize, State> {
        &self.states
    }

    /// Add a new state and return its index
    pub fn add_state(&mut self, state: State) -> usize {
        let index = self.states.len();

        self.states.insert(index, state);

        index
    }

    pub fn set_initial(&mut self, i: usize) {
        self.initial = i;
    }

    pub fn initial(&self) -> &usize {
        &self.initial
    }

    pub fn rewind(&mut self) {
        self.current = self.initial;
    }

    pub fn current(&self) -> usize {
        self.current
    }

    pub fn state_accept(&self, index: usize) -> bool {
        if let Some(state) = self.states.get(&index) {
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
        self.states.insert(self.current, accept);
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

    /// Removes a state from DFA, returns an Option with informations if state was accepting and
    /// its transitions
    pub fn remove_state(&mut self, index: usize) -> Option<(bool, Option<HashSet<Transition<T>>>)> {
        if self.states.contains_key(&index) {
            Some((self.states.remove(&index).unwrap(), self.transitions.remove(&index)))
        } else {
            None
        }
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
    pub fn non_determinist_states(&self) -> Option<HashMap<usize, HashMap<T, HashSet<usize>>>> {
        let mut ndet = HashMap::new();

        for s in self.transitions.keys() {
            let ndt = self.ndt_of(s);

            if ndt.len() > 0 {
                ndet.insert(s.clone(), ndt);
            }
        }

        return if ndet.len() > 0 {
            Some(ndet)
        } else {
            None
        }
    }

    /// Remove non-deterministic states from the DFA
    pub fn determinize(&mut self) {
        let mut state_map: HashMap<usize, HashSet<usize>> = HashMap::new();

        while let Some(non_deterministic) = self.non_determinist_states() {
            println!("Statemap Init: {:?}", state_map);
            // Map the new created states and their new transitions
            let mut new_states: HashMap<usize, Vec<_>> = HashMap::new();

            // {usize => {T => usize [dest]}}
            for (s, by) in non_deterministic {
                // {T => usize}
                // First, for each non-deterministic transition, map a new state
                for (c, to) in by.iter() {
                    let mut trans_to: HashSet<_> = HashSet::new();
                    let mut has_equivalent: Option<usize> = None;
                    let mut ndtrans = Vec::new(); // Vec of non-det transitions

                    // Check if there is any equivalent determinized transition created
                    for (ns, mapped) in &state_map {
                        if mapped == to {
                            println!("EQUIV {}<-->{}", s, ns);
                            has_equivalent = Some(ns.to_owned());
                            break;
                        }
                    }

                    // If some of mapped transitions are equivalent, then use this state as target
                    // to the non-deterministic transition, else create and map the new transition
                    let newstate = if let Some(st) = has_equivalent { st } else {
                        let mut accept = false;

                        // Check if any target states from transitions accept
                        for target in to.iter() {
                            if self.state_accept(target.to_owned()) {
                                accept = true;
                                break;
                            }
                        }

                        let index = self.add_state(accept);

                        {
                            let mut hm = HashSet::new();

                            for t in to {
                                // If target states are created by minimization, then get its
                                // original transitions, or simply insert the state
                                if state_map.contains_key(t) {
                                    hm = hm.union(&state_map[t]).cloned().collect();
                                } else {
                                    hm.insert(t.to_owned());
                                }
                            }

                            state_map.insert(index, hm);
                        }

                        //state_map.insert(index, to.clone());

                        index
                    };

                    if let Some(ts) = self.transitions.get_mut(&s) {
                        let mut dets = HashSet::new();

                        for d in ts.drain() {
                            if d.0 == c.to_owned() {
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
                    self.create_transition_between(&s, &newstate, c.clone());
                    new_states.insert(newstate, ndtrans);
                }
            }

            // After all states are mapped then we could create their transitions, else
            // inconsistent transitions may be mapped makeing determinization worthless
            for (ns, ts) in new_states {
                let new_state_transitions = {
                    let mut trans = Vec::new();

                    for ndt in ts {
                        // Add relationed states transitions
                        if let Some(ts) = self.transitions.get(&ndt.1) {
                            for t in ts {
                                trans.push(t.clone());
                            }
                        }
                    }

                    trans
                };

                for dt in new_state_transitions.into_iter() {
                    self.add_transition_to(&ns, dt);
                }
            }
            println!("Statemap end: {:?}", state_map);
        }
    }

    // Would be great to use an "Iterator" to BFS
    pub fn get_unreachable_states(&self) -> Vec<usize> {
        let mut unreached: Vec<usize> = (0..self.states.len()).collect();
        let mut current: usize;
        let mut next = VecDeque::new();
        
        next.push_back(self.initial().to_owned());

        // "BFS"
        while unreached.len() > 0 && next.len() > 0 {
            current = next.pop_front().unwrap();

            for ts in self.transitions.get(&current) {
                for t in ts {
                    if unreached.binary_search(&t.1).is_ok() {
                        next.push_back(t.1);
                    }
                }
            }

            unreached.remove_item(&current);
        }

        unreached
    }

    pub fn get_dead_states(&self) -> Vec<usize> {
        let mut dead: Vec<usize> = (0..self.states.len()).collect();
        let mut current: usize;
        let mut stack = vec![self.initial().to_owned()];
        let mut path = Vec::new();

        // "DFS"
        while dead.len() > 0 && stack.len() > 0 {
            current = stack.pop().unwrap();
            path.push(current);

            if let Some(trans) = self.transitions.get(&current) {
                if trans.len() == 0 {
                    if self.state_accept(current) {
                        for s in &path {
                            dead.remove_item(&s);
                        }
                    } // else &current is dead
                }

                for t in trans {
                    if dead.binary_search(&t.1).is_ok() {
                        stack.push(t.1);
                    } else {
                        dead.remove_item(&t.1);
                    }
                }
            }
        }

        dead
    }

    pub fn remove_unreachable_states(&mut self) {
        let unreached = self.get_unreachable_states();

        for state in unreached {
            self.remove_state(state);
        }
    }

    pub fn remove_dead_states(&mut self) {
        let dead = self.get_dead_states();

        for state in dead {
            self.remove_state(state);
        }
    }

    pub fn minimize(&mut self) {
        self.remove_unreachable_states();
        self.remove_dead_states();
    }
}

impl<T: Display + Debug + Eq + Hash + Ord> Dfa<T> {
    pub fn to_csv(&self) -> String {
        let mut csv = String::from("State");
        let mut alphabet: Vec<&T> = self.alphabet.iter().collect();
        let mut states: Vec<&usize> = self.states.keys().collect();

        alphabet.sort();
        states.sort();

        // Header
        for a in &alphabet {
            csv += format!(",{}", a).as_str();
        }

        csv.push('\n');

        for k in &states {
            let accept = self.states.get(&k).unwrap();

            if k.to_owned() == self.initial() { csv.push_str("->"); }
            if accept.to_owned() { csv.push('*'); }

            csv += format!("<{}>", k).as_str();

            for a in &alphabet {
                match self.transitions.get(&k) {
                    Some(trans) => {
                        let mut has_states = false;

                        for t in trans {
                            if &t.0 == a.to_owned() {
                                // Controls the first comma
                                if ! has_states { csv.push(','); has_states = true; }
                                csv += format!("<{}>", t.1).as_str();
                            }
                        }

                        if ! has_states {
                            csv.push_str(",-");
                        }
                    },
                    None => csv.push_str(",-")
                }
            }

            csv.push('\n');
        }

        csv
    }
}
