extern crate dfa;
#[macro_use]
extern crate log;
extern crate env_logger;

use dfa::lexer::Dfa;
use std::fs::File;
use std::io::{ BufRead, BufReader };
use std::env;
use std::collections::HashMap;

const INITIAL_STATE_CHAR: char = 'S';

#[derive(PartialEq, Clone, Copy)]
// enum Input: State Control for Token and Grammar recognizance
// someword <- std token
//
// <S> ::= a<A> | b<B> | <>
//  ^      ^       ^^^   ^^
//  |      |       |||   ||
//  |      |       |||   Epsilon
//  |      |       Nonterminal Symbol (State)
//  |      Terminal Symbol (Transition)
//  State
enum Input {
    // Reading tokens as-is
    // E.g.: if
    // E.g.: else
    Normal,
    // Reading State definitions, like the left part of <S> ::= ...
    StateDef,
    // Reading the transitions, like the terminals of the right part of state definition
    // E.g.: In `<S> ::= a<B> | b<E>`, the terminals are 'a' and 'b'
    StateTransitions,
    // Reading the transitions, like the nonterminals of the right part of state definition
    // E.g.: In `<S> ::= e<C> | q<B> | <>`, the nonterminals are '<C>' '<B>' and '<>'.
    // <> is aknowleged as Epsilon (Epsilon is a terminal symbol! But in this state it is aknowledged!)
    // The bool member is to identify if any char exists inside "<>", eg: <B> = bool true and
    // <> = false
    StateTransitionTarget(bool)
}

fn main() {
    let files: Vec<String> = env::args().skip(1).collect();
    let mut reading = Input::Normal;
    let mut dfa = Dfa::new();
    let mut reader: BufReader<File>;

    env_logger::init().expect("Logger out!");

    for f in &files {
        // TODO: Translate to English (or maybe Esperanto!)
        let file = File::open(f).expect("Não consegui ler os arquivos");
        let mut temp_transition: Option<char> = None;
        let mut grammar_mapper: HashMap<char, usize> = HashMap::new();

        debug!("Reading `{}`...", f);
        reader = BufReader::new(file);

        for l in reader.lines() {
            // TODO: Fix non-helpful error message
            // TODO: Translate to English (or maybe Esperanto!)
            let line = l.expect("Houve um erro ao ler um arquivo");
            debug!("Line: `{}`", line);

            for c in line.chars() {
                match reading {
                    Input::Normal if c != ' ' => {
                        if c == '<' {
                            reading = Input::StateDef;
                        } else {
                            let state_index = dfa.add_state(false);
                            dfa.create_transition_and_walk(c, state_index);
                        }
                    },
                    Input::StateDef if c != ' ' => {
                        match c {
                            '<' => continue,
                            '>' => reading = Input::StateTransitions,
                            _   => {
                                // Add to mapper which index solves to current State, e.g. <A> maps to
                                // index 3, <E> to index 8...
                                let index = if c == INITIAL_STATE_CHAR {
                                    *dfa.initial()
                                } else {
                                    if ! grammar_mapper.contains_key(&c) {
                                        let state = dfa.add_state(false);
                                        grammar_mapper.insert(c, state);

                                        debug!("[DEF] Indexing {} to {}", c, state);
                                    }

                                    *grammar_mapper.get(&c).unwrap()
                                };

                                // If current char is == INITIAL_STATE_CHAR, rewind to initial
                                // else, go to new state
                                if c == INITIAL_STATE_CHAR { dfa.rewind(); }
                                else { dfa.set_current(index).expect("This should not happen!"); }
                            }
                        }
                    },
                    Input::StateTransitions => {
                        match c {
                            '<'       => reading = Input::StateTransitionTarget(false),
                            // Epsilon Transitions, `b` in <A> ::= a<A> | b | c<C> or in
                            // <B> ::= a<B> | b
                            '|' | ' ' => {
                                if let Some(t) = temp_transition.take() {
                                    let empty_state = dfa.add_state(true);
                                    warn!("Creating new empty-state to {}: {}", t, empty_state);
                                    dfa.create_transition(t, empty_state);
                                }
                            },
                            ':' | '=' => continue,
                            ch if ch != ' ' => {
                                if temp_transition.is_none() {
                                    temp_transition = Some(ch);
                                } else {
                                    // If there is two transitions, the grammar is not regular
                                    warn!(
                                        "Nonregular grammar detected (a.k.a. reassignment to temp_transition! '{}' -> '{:?}')",
                                        c, temp_transition
                                    );
                                }
                            },
                            _ => ()
                        }
                    },
                    Input::StateTransitionTarget(had_state) if c != ' ' => {
                        if c == '>' {
                            reading = Input::StateTransitions;

                            // Check if is Epsilon (aka <>)
                            if temp_transition.is_none() && ! had_state {
                                dfa.set_current_state_accept(true)
                            }
                        } else {
                            // In recognization, get the entry value if state exists.
                            // If state doesn't exists yet, we need to map it [`or_insert`] and hope that
                            // it will be defined in the future :P
                            let target = if c == INITIAL_STATE_CHAR {
                                *dfa.initial()
                            } else {
                                if ! grammar_mapper.contains_key(&c) {
                                    let state = dfa.add_state(false);
                                    grammar_mapper.insert(c, state);

                                    debug!("[TRANS] Indexing {} to {}", c, state);
                                }

                                *grammar_mapper.get(&c).unwrap()
                            };

                            if let Some(t) = temp_transition.take() {
                                dfa.create_transition(t, target)
                            } else {
                                warn!("Epsilon-transition to <{}>", c);
                            }

                            reading = Input::StateTransitionTarget(true);
                        }
                    }
                    _ => ()
                }
            }

            // Line ends like: <A> ::= a<A> | b<B> | c
            // and so 'c' is not parsed
            if let Some(t) = temp_transition.take() {
                let empty_state = dfa.add_state(true);
                warn!("Creating new empty-state to {}: {}", t, empty_state);
                dfa.create_transition(t, empty_state);
            }

            if reading == Input::Normal {
                // We had finished the current line, so the last state accept the current token
                dfa.set_current_state_accept(true);
                dfa.rewind();
            } else {
                // Finished reading a line of grammar, must reset the state to keep reading
                reading = Input::StateDef;
            }
        }
    }

    info!("All files were parsed");

    dfa.determinize();
    dfa.minimize();
    println!("{}", dfa.to_csv());
}
