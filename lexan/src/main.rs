extern crate dfa;

use dfa::lexer::Dfa;
use std::fs::File;
use std::io::{ stderr, Write, BufRead, BufReader };
use std::env;
use std::collections::HashMap;

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
    Normal,
    // Reading State definitions, like the left part of <S> ::= ...
    StateDef,
    // Reading the transitions, like the terminals of the right part of state definition
    // E.g.: In `<S> ::= a<B> | b<E>`, the terminals are 'a' and 'b'
    StateTransitions,
    // Reading the transitions, like the nonterminals of the right part of state definition
    // E.g.: In `<S> ::= e<C> | q<B> | <>`, the nonterminals are '<C>' '<B>' and '<>'.
    // <> is aknowleged as Epsilon
    StateTransitionTarget
}

fn main() {
    let files: Vec<String> = env::args().skip(1).collect();
    let mut reading = Input::Normal;
    let mut dfa = Dfa::new();
    let mut reader: BufReader<File>;

    for f in &files {
        // TODO: Translate to English (or maybe Esperanto!)
        let file = File::open(f).expect("Não consegui ler os arquivos");
        let mut temp_transition: Option<char> = None;
        let mut grammar_mapper: HashMap<char, usize> = HashMap::new();

        writeln!(stderr(), "Reading `{}`...", f).unwrap();
        reader = BufReader::new(file);

        for l in reader.lines() {
            // TODO: Fix non-helpful error message
            // TODO: Translate to English (or maybe Esperanto!)
            let line = l.expect("Houve um erro ao ler um arquivo");

            for c in line.chars() {
                // Skipping separators
                if c == ' ' { continue; }

                match reading {
                    Input::Normal => {
                        if c == '<' {
                            reading = Input::StateDef;
                        } else {
                            let state_index = dfa.add_state(false);
                            dfa.create_transition_and_walk(c, state_index);
                        }
                    },
                    Input::StateDef => {
                        if c == '>' {
                            reading = Input::StateTransitions;
                        } else {
                            // Add the new state
                            let index = dfa.add_state(false);

                            // Add to mapper which index solves to current State, e.g. <S> maps to
                            // index 3, <E> to index 8...
                            grammar_mapper.insert(c, index);

                            // Walk to new state
                            dfa.set_current(index).expect("This should not happen!");
                        }
                    },
                    Input::StateTransitions => {
                        match c {
                            '<'             => reading = Input::StateTransitionTarget,
                            '|' | ':' | '=' => continue,
                            ch              => {
                                if temp_transition.is_none() {
                                    temp_transition = Some(ch);
                                } else {
                                    writeln!(stderr(), "Warning: Reassignment to temp_transition! '{}' -> '{:?}'", c, temp_transition).unwrap();
                                }
                            }
                        }
                    },
                    Input::StateTransitionTarget => {
                        if c == '>' {
                            // TODO: Handle e-transitions here
                            reading = Input::StateTransitions;
                        } else {
                            // In recognization, state exists in mappings
                            if grammar_mapper.contains_key(&c) {
                                let target = grammar_mapper.get(&c).unwrap();

                                if let Some(t) = temp_transition.take() {
                                    dfa.create_transition(t, *target)
                                } else {
                                    writeln!(stderr(), "Epsilon-transition to <{}>", c).unwrap();
                                }
                            } else {
                                let index = dfa.add_state(false);
                                // State don't exists yet, we need to map it and hope that it will be
                                // defined in the future :P
                                writeln!(stderr(), "Transição para estado inexistente: {}", c).unwrap();
                                grammar_mapper.insert(c, index);
                            }
                        }
                    }
                }
            }
        }

        // We had finished the current line, so the last state accept the current token
        dfa.set_current_state_accept(true);
        dfa.rewind();
    }

    println!("{}", dfa.to_csv());
}
