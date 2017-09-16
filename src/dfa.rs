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

#[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn states(&self) -> &HashMap<usize, State> {
        &self.states
    }

    /// Add a new state and return its index
    pub fn add_state(&mut self, state: State) -> usize {
        let index = self.states
            .keys()
            .max()
            .unwrap_or(&0)
            .to_owned() + 1;

        self.states.insert(index, state);

        index
    }

    #[allow(dead_code)]
    pub fn set_initial(&mut self, i: usize) {
        self.initial = i;
    }

    pub fn initial(&self) -> &usize {
        &self.initial
    }

    pub fn rewind(&mut self) {
        self.current = self.initial;
    }

    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn alphabet(&self) -> &HashSet<T> {
        &self.alphabet
    }

    #[allow(dead_code)]
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

        if self.transitions.contains_key(state) {
            self.transitions.get_mut(state).unwrap().insert(trans);
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
        for ts in self.transitions.values_mut() {
            ts.retain(|x| x.1 != index);
        }

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

            for t in &self.transitions[index] {
                if &t.0 == c {
                    multiple.insert(t.1);
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

            if !ndt.is_empty() {
                ndet.insert(*s, ndt);
            }
        }

        if !ndet.is_empty() {
            Some(ndet)
        } else {
            None
        }
    }

    /// Remove non-deterministic states from the DFA
    pub fn determinize(&mut self) {
        let mut state_map: HashMap<usize, HashSet<usize>> = HashMap::new();

        while let Some(non_deterministic) = self.non_determinist_states() {
            // Map the new created states and their new transitions
            let mut new_states: HashMap<usize, Vec<_>> = HashMap::new();

            // {usize => {T => usize [dest]}}
            for (s, by) in non_deterministic {
                // {T => usize}
                // First, for each non-deterministic transition, map a new state
                for (c, to) in &by {
                    let mut trans_to: HashSet<_> = HashSet::new();
                    let mut has_equivalent: Option<usize> = None;
                    let mut ndtrans = Vec::new(); // Vec of non-det transitions

                    // Parse all transitions of the future new determinized state
                    for t in to {
                        // If target states are created by minimization, then get its
                        // original (the ones whose created the first state) transitions,
                        // else simply insert the state
                        if state_map.contains_key(t) {
                            trans_to = trans_to.union(&state_map[t]).cloned().collect();
                        } else {
                            trans_to.insert(t.to_owned());
                        }
                    }

                    // Check if there is any equivalent determinized transition created
                    for (ns, mapped) in &state_map {
                        if mapped == &trans_to {
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

                        state_map.insert(index, trans_to);

                        index
                    };

                    // Cleanup the non-deterministic states removing the non-deterministic
                    // transitions
                    if let Some(ts) = self.transitions.get_mut(&s) {
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
                    self.create_transition_between(&s, &newstate, c.clone());
                    // Map this state its transitions
                    new_states.insert(newstate, ndtrans);
                }
            }

            // After all states are mapped then we could create their transitions, else
            // inconsistent transitions may be mapped making determinization worthless
            for (ns, ts) in new_states {
                // Check if any of the states is 
                let superstate = {
                    let mut state = None;
                    let mut ss = HashSet::new();

                    for ndt in &ts {
                        if state_map.contains_key(&ndt.1) {
                            ss = ss.union(&state_map[&ndt.1]).cloned().collect();
                        }
                    }

                    for ndt in &ts {
                        if state_map.contains_key(&ndt.1) && ss == state_map[&ndt.1] {
                            state = Some(ndt.1);
                            break;
                        }
                    }

                    state
                };

                let new_state_transitions = {
                    let mut trans = Vec::new();

                    if let Some(ss) = superstate {
                        for t in &self.transitions[&ss] {
                            trans.push(t.clone());
                        }
                    } else {
                        for ndt in ts {
                            // Add relationed states transitions
                            if let Some(ts) = self.transitions.get(&ndt.1) {
                                for t in ts {
                                    trans.push(t.clone());
                                }
                            }
                        }
                    }

                    trans
                };

                for dt in new_state_transitions {
                    self.add_transition_to(&ns, dt);
                }
            }
        }
    }

    // Would be great to use an "Iterator" to BFS
    pub fn get_unreachable_states(&self) -> Vec<usize> {
        let mut unreached: Vec<usize> = self.states.keys().cloned().collect();
        let mut current: usize;
        let mut next = VecDeque::new();

        // Using binary seach requires a sorted vec
        unreached.sort();
        
        next.push_back(self.initial().to_owned());

        // "BFS"
        while !unreached.is_empty() && !next.is_empty() {
            current = next.pop_front().unwrap();

            if let Some(ts) = self.transitions.get(&current) {
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
        let mut unvisited: Vec<usize> = self.states.keys().cloned().collect();
        let mut dead: Vec<usize>;
        // The current path of DFS
        let mut path: Vec<usize> = Vec::new();
        // (path, stacked_by)
        let mut stack: Vec<(usize, usize)> = vec![
            (self.initial().to_owned(), self.initial().to_owned())
        ];

        // Using binary seach requires a sorted vec
        unvisited.sort();
        dead = unvisited.clone();

        // "DFS"
        while !dead.is_empty() && !stack.is_empty() {
            let (current, stacked_by) = stack.pop().unwrap();

            // Check and correct path
            while let Some(last_in_path) = path.iter().last().cloned() {
                if stacked_by != last_in_path { path.pop(); }
                else { break; }
            }

            path.push(current);

            if let Some(trans) = self.transitions.get(&current) {
                for t in trans {
                    // Check if any neighbour accept or is not dead, if so, remove it from dead
                    // states and set the whole path as non-dead
                    if self.state_accept(t.1) || dead.binary_search(&t.1).is_err() {
                        dead.remove_item(&t.1);
                        for s in &path { dead.remove_item(s); }
                    }

                    // Stack neighbours that were not visited
                    if unvisited.iter().any(|x| x == &t.1) {
                        unvisited.remove_item(&t.1);
                        stack.push((t.1, current));
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

    pub fn insert_error_state(&mut self) {
        let error_state    = self.add_state(true);
        let states: Vec<_> = self.states.keys().cloned().collect();
        let alphabet: HashSet<_> = self.alphabet.iter().cloned().collect();

        info!("Error State: {}", error_state);

        for state in states {
            let transitions_by = { 
                let transitions = self.transitions.entry(state).or_insert_with(HashSet::new);
                transitions.iter().map(|x| x.0.clone()).collect()
            };

            let missing = alphabet.difference(&transitions_by);

            debug!("Missing on {}: {:?}", state, missing);

            for ch in missing {
                self.create_transition_between(&state, &error_state, ch.to_owned());
            }
        }
    }
}

impl<T: Display + Debug + Eq + Hash + Ord> Dfa<T> {
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph FA {\nrankdir=\"LR\";\n");
        let alphabet: Vec<&T>   = { let mut a = self.alphabet.iter().collect::<Vec<_>>(); a.sort(); a };
        let states: Vec<&usize> = { let mut s = self.states.keys().collect::<Vec<_>>(); s.sort(); s };

        for state in states {
            if self.state_accept(state.to_owned()) {
                dot += format!("{} [shape=doublecircle];\n", state).as_str();
            }

            for s in &alphabet {
                if let Some(transitions) = self.transitions.get(state) {
                    let mut ts = "{".to_string();

                    for t in transitions.iter() {
                        if &&t.0 == s {
                            if ts.len() > 1 { ts.push(','); }
                            ts += format!("{}", t.1).as_str();
                        }
                    }

                    ts.push('}');

                    if ts.len() > 2 {
                        dot += format!("{} -> {} [label={}];\n", state, ts, s).as_str();
                    }
                }
            }
        }

        dot.push_str("}\n");

        dot
    }

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
            let accept = &self.states[k];

            if *k == self.initial() { csv.push_str("->"); }
            if accept.to_owned() { csv.push('*'); }

            csv += format!("<{}>", k).as_str();

            for a in &alphabet {
                match self.transitions.get(k) {
                    Some(trans) => {
                        let mut has_states = false;

                        for t in trans {
                            if t.0 == **a {
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
