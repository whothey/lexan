Lexan
===============

An university task whose goal is to create a Deterministic Finite Automaton.

## Input Files

Input files may have tokens as-is or Regular Grammars, but each must follow the rules:

### Tokens

Tokens should be separated by lines. To create a DFA to tokens `if`, `else` and `end`
you must have a file:

```
if
else
end
```

None of the following will generate the desired DFA:

```
if else
e
n
d
```

### Regular Grammars

Grammars are defined by state-definition pairs, separated by `::=` as it follows:

```
<S> ::= a<A> | b<B>
<A> ::= b<S> | a<A>
<B> ::= b<B> | <>
```

Where:

- `<_>` is a nonterminal symbol and `_` is any single-byte character
- `<>` is aknowledged as "Epsilon"
- Any single-byte character is a terminal symbol, except: `:`, `=`, ` `, `<`, `>` and `|`
- The left side of `::=` is the state and the right side are its transitions
- A state may have multiple transitions and them are separated by `|`
- Each transition must be defined as `a<A>` or `<>`, where `a` is any terminal symbol
  and `<A>` is any nonterminal symbol.

You may have multiple grammars defined as:

```
<S> ::= a<A> | b<B>                  <-- S State: Here begins a grammar
<A> ::= b<S> | a<A>
<S> ::= a<A> | e<A> | i<A>           <-- S State: Here begins another grammar
<A> ::= a<A> | e<A> | i<A> | <>
```
