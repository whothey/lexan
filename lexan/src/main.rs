#![feature(vec_remove_item)]
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate clap;

mod dfa;

use clap::{ App, Arg };
use env_logger::LogBuilder;
use dfa::Dfa;
use std::path::PathBuf;
use std::fs::{ File, OpenOptions };
use std::io::{ BufRead, BufReader, BufWriter, Write };
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

fn parse_grammar(files: Vec<&str>) -> Dfa<char> {
    let mut reading = Input::Normal;
    let mut dfa = Dfa::new();
    let mut reader: BufReader<File>;

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

    dfa
}

fn dump_automata(aut: &Dfa<char>, p: &PathBuf) {
    let mut fp: File;
    let mut writer: BufWriter<File>;

    {
        let mut path = p.clone();
        path.set_extension("dot");
        let dotfile = path.as_path();

        fp = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(dotfile)
            .unwrap();

        writer = BufWriter::new(fp);
        writer.write_all(aut.to_dot().as_bytes()).unwrap();
    }

    {
        let mut path = p.clone();
        path.set_extension("csv");
        let csvfile = path.as_path();

        fp = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(csvfile)
            .unwrap();

        writer = BufWriter::new(fp);
        writer.write_all(aut.to_csv().as_bytes()).unwrap();
    }
}

fn main() {
    let app = App::new("DFA Generator")
        .version("0.1.0")
        .author("Gabriel Henrique Rudey <gabriel.rudey@gmail.com>")
        .about("Create DFAs by Formal Grammars")
        .arg(Arg::with_name("files")
             .help("The files to be parsed")
             .takes_value(true)
             .value_name("FILE")
             .multiple(true)
             .required(true))
        .arg(Arg::with_name("dump")
             .short("d")
             .long("dump")
             .takes_value(true)
             .value_name("DIRECTORY")
             .help("The directory to dump debug files"))
        .arg(Arg::with_name("verbosity")
             .short("v")
             .help("Set the log level")
             .multiple(true));

    let matches = app.get_matches();
    let mut logger = LogBuilder::new();
    let log_level  = env::var("LOG").unwrap_or_else(|_| {
        match matches.occurrences_of("verbosity") {
            1 => "ERROR".to_string(),
            2 => "WARN".to_string(),
            3 => "INFO".to_string(),
            4 => "DEBUG".to_string(),
            _ => "NONE".to_string()
        }
    });

    logger.parse(&log_level);
    logger.init().expect("Could not start logger");

    let files: Vec<&str>   = matches.values_of("files").unwrap().collect();
    let dump: Option<&str> = matches.value_of("dump");

    let mut dfa = parse_grammar(files);

    info!("All files were parsed");

    // Debug or simply calculate the result
    if let Some(dir) = dump {
        let mut file = PathBuf::from(dir.to_owned());

        file.push("1fa");
        dump_automata(&dfa, &file);

        dfa.determinize();
        file.set_file_name("2dfa");
        dump_automata(&dfa, &file);

        file.set_file_name("3dfa_nounreached");
        dfa.remove_unreachable_states();
        dump_automata(&dfa, &file);

        dfa.remove_dead_states();
        file.set_file_name("4dfa_final");
        dump_automata(&dfa, &file);

        dfa.insert_error_state();
        file.set_file_name("5dfa_error");
        dump_automata(&dfa, &file);
    } else {
        dfa.determinize();
        dfa.minimize();
        dfa.insert_error_state();
    }

    println!("{}", dfa.to_csv());
}
