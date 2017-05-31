use lexer::*;

#[test]
fn it_creates_a_dfa() {
    let dfa: Dfa<char> = Dfa::new();

    assert!(dfa.states().len() == 1);
}

#[test]
fn it_can_add_new_states_to_dfa() {
    let mut dfa: Dfa<char> = Dfa::new();

    dfa.add_state(false);

    assert!(dfa.states().len() == 2);
}

#[test]
fn it_add_new_transitions_and_walk() {
    let mut dfa = Dfa::new();
    let b = dfa.add_state(false);
    let c = dfa.add_state(false);
    let d = dfa.add_state(true);

    dfa.create_transition_and_walk('b', b);
    dfa.create_transition_and_walk('c', c);
    dfa.create_transition_and_walk('d', d);

    assert!(dfa.states().len() == 4);
    assert!(dfa.current() == 3);

    dfa.rewind();

    assert!(dfa.current() == dfa.initial());
}

#[test]
fn it_handle_multiple_states_on_same_transition() {
    let mut dfa = Dfa::new();
    let a1 = dfa.add_state(false);
    let a2 = dfa.add_state(false);
    let b = dfa.add_state(true);
    let c = dfa.add_state(true);

    dfa.create_transition_and_walk('a', a1);
    dfa.create_transition_and_walk('b', b);
    dfa.rewind();
    dfa.create_transition_and_walk('a', a2);
    dfa.create_transition_and_walk('c', c);

    // 'a', 'b' and 'c'
    assert!(dfa.alphabet().len() == 3);
    // Two of transitions by <S> -> a
    assert!(dfa.transitions().len() == 3);
    // S, a1, a2, b and c
    assert!(dfa.states().len() == 5);
}

#[test]
fn it_creates_the_csv() {
    let mut dfa = Dfa::new();
    let b = dfa.add_state(false);
    let c = dfa.add_state(false);
    let d = dfa.add_state(true);

    dfa.create_transition_and_walk('b', b);
    dfa.create_transition_and_walk('c', c);
    dfa.create_transition_and_walk('d', d);

    println!("{}", dfa.to_csv());
}

#[test]
fn it_solves_project1_example() {
    let mut dfa: Dfa<char> = Dfa::new();
    let spec = "se
entao
senao
<S> ::= a<A> | e<A> | i<A> | i<A> | u<A>
<A> ::= a<A> | e<A> | i<A> | o<A> | u<A> | <>";

    for line in spec.lines() {
        for c in line.chars() {
            let state_index = dfa.add_state(false);
            dfa.create_transition_and_walk(c, state_index);
        }

        dfa.rewind();
    }

    println!("{}", dfa.to_csv());
}
