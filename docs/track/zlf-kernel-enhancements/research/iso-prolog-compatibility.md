# ISO Prolog Compatibility and General Prolog Programming Capabilities

## 1. Purpose

zlf currently focuses on WAM-backed graph facts, storage providers, compiled rules, and graph/query integration. This note evaluates how far zlf is from ISO-style general Prolog and defines an implementation path for common Prolog programming capabilities:

- ISO core syntax and terms;
- arithmetic and comparison;
- strings, atoms, codes, chars;
- proper list representation and pattern matching;
- control predicates;
- dynamic database predicates;
- commonly expected standard libraries.

The goal is not to clone SWI-Prolog completely in one step. The goal is to become compatible with the ISO core plus a practical standard-library subset sufficient for ordinary Prolog programs.

## 2. Current zlf gap summary

### 2.1 Already present

- WAM-style heap, registers, choice points, trail, environments.
- Basic unification and backtracking.
- Facts and rules compiled to WAM instructions.
- Multi-goal queries via generated wrapper rules.
- Persistent rule storage.
- Storage/index-backed predicates.
- Cut family, execute, permanent variables, list instruction support, switch instructions.
- Parser support for atoms, variables, numbers, strings, compounds, lists, objects, simple inequality.

### 2.2 Major missing ISO/general Prolog capabilities

| Area | Current state | Required state |
|---|---|---|
| List representation | `Term::List` lowered to internal `list/N` structure | canonical `[]` and `'.'/2` cons cells; support `[H|T]` matching |
| Operators | almost no operator parser | ISO operators: `:-`, `,`, `;`, `->`, `\+`, `=`, `\=`, `is`, arithmetic comparisons, etc. |
| Arithmetic | no `is/2` or arithmetic evaluator | integer/float arithmetic, `=:=`, `=\=`, `<`, `=<`, `>`, `>=` |
| Type tests | missing | `var/1`, `nonvar/1`, `atom/1`, `integer/1`, `float/1`, `number/1`, `atomic/1`, `compound/1` |
| Term inspection | missing | `functor/3`, `arg/3`, `=../2`, `copy_term/2`, `term_variables/2` |
| Control | partial cut only | `true/0`, `fail/0`, `!/0`, `;/2`, `->/2`, `\+/1`, `once/1`, `call/N` subset |
| Dynamic DB | custom fact writer only | `asserta/1`, `assertz/1`, `retract/1`, `retractall/1`, `abolish/1`, `clause/2` subset |
| Error model | ad-hoc errors | ISO-style instantiation/type/domain/existence/permission errors |
| Standard libraries | none | practical libraries: lists, apply, ordsets, aggregate subset, dcg basics |
| IO/streams | not implemented | defer or minimal `write/1`, `nl/0`, `read_term/2` for REPL/debug |
| Modules | not implemented | defer; not needed for graph database MVP |

## 3. Compatibility target

### 3.1 Target level A: ISO Core Practical Subset

This is the first meaningful milestone.

Required:

- proper terms and canonical lists;
- unification and term comparison;
- arithmetic evaluation and comparison;
- type tests;
- control predicates;
- dynamic assert/retract integration with zlf storage/rule store;
- lists library subset.

Not required:

- full ISO stream system;
- full modules;
- full term expansion;
- full implementation-defined flags;
- exact SWI-Prolog compatibility.

### 3.2 Target level B: Common Prolog Programming Subset

Adds:

- strings/chars/codes predicates;
- meta-call subset;
- DCG basics;
- aggregate/findall/bagof/setof subset;
- ordsets library;
- better errors and predicate indicators.

### 3.3 Target level C: Broad Prolog Compatibility

Adds:

- richer IO and streams;
- source loading/consult;
- modules or namespaces;
- full operator declarations;
- more SWI-compatible libraries.

## 4. Term model changes

### 4.1 Numbers

Current `Term::Number(f64)` is insufficient for ISO-style arithmetic. Split numbers:

```rust
enum Term {
    Variable(String),
    Atom(String),
    Integer(i64),
    Float(f64),
    String(String),
    Compound { name: String, args: Vec<Term> },
    List(Vec<Term>),        // parser convenience only
    Object(Vec<(String, Term)>),
}
```

Parser can keep `Term::List` as AST sugar, but WAM lowering should convert lists to canonical cons cells.

### 4.2 Canonical list representation

ISO lists are syntactic sugar:

```prolog
[]           == []
[a]          == '.'(a, [])
[a, b]       == '.'(a, '.'(b, []))
[H|T]        == '.'(H, T)
```

Current `list/N` representation should be treated as a temporary internal shortcut and replaced in WAM codegen.

Required parser AST:

```rust
enum Term {
    List { items: Vec<Term>, tail: Option<Box<Term>> },
}
```

or separate:

```rust
Term::List(Vec<Term>)
Term::ListCons { head: Box<Term>, tail: Box<Term> }
```

Recommended internal AST:

```rust
Term::List { items: Vec<Term>, tail: Option<Box<Term>> }
```

Lowering algorithm:

```text
lower_list([a,b,c])      -> '.'(a, '.'(b, '.'(c, [])))
lower_list([a,b|Tail])   -> '.'(a, '.'(b, Tail))
lower_list([])           -> []
```

### 4.3 Strings, chars, codes

Keep `Term::String(String)` for strings. Add conversion predicates later:

```prolog
atom_string(Atom, String).
string_chars(String, Chars).
string_codes(String, Codes).
atom_chars(Atom, Chars).
atom_codes(Atom, Codes).
number_string(Number, String).
```

Character list representation:

```prolog
string_chars("ab", [a,b]).
string_codes("ab", [97,98]).
```

## 5. Parser and operator support

### 5.1 Required operators

Minimum operators to parse ordinary Prolog:

| Operator | Type | Meaning |
|---|---|---|
| `:-` | xfx | rule/directive |
| `,` | xfy | conjunction |
| `;` | xfy | disjunction |
| `->` | xfy | if-then |
| `\+` | fy | negation as failure |
| `=` | xfx | unification |
| `\=` | xfx | not unifiable |
| `==` | xfx | term identical |
| `\==` | xfx | term not identical |
| `@<`, `@=<`, `@>`, `@>=` | xfx | standard term order |
| `is` | xfx | arithmetic evaluation |
| `=:=`, `=\=`, `<`, `=<`, `>`, `>=` | xfx | arithmetic comparisons |
| `+`, `-` | yfx/fy | arithmetic plus/minus/sign |
| `*`, `/`, `//`, `mod`, `rem` | yfx | arithmetic ops |

Implementation choices:

1. Implement a Pratt parser / precedence climbing parser for operators.
2. Keep pest for tokens and high-level clauses.
3. Convert operators into canonical compounds:

```prolog
X is Y + 1      -> is(X, +(Y, 1))
A, B            -> ','(A, B)
A ; B           -> ';'(A, B)
\+ Goal         -> '\+'(Goal)
```

### 5.2 Directives

Support at least:

```prolog
:- dynamic p/2.
:- table reachable/2.
:- op(Precedence, Type, Name).   % later
```

Parser representation:

```rust
enum PrologItem {
    Fact(Term),
    Rule(PrologRule),
    Directive(Term),
    Query(Vec<Term>),
}
```

## 6. Builtin architecture

### 6.1 Builtin provider trait

Add an internal builtin layer before storage/index providers.

```rust
trait BuiltinPredicate {
    fn key(&self) -> PredicateKey;
    fn eval(&self, args: &[Term], ctx: &mut BuiltinContext) -> BuiltinResult;
}

enum BuiltinResult {
    Fail,
    Succeed,
    Deterministic(Vec<Binding>),
    Nondeterministic(Vec<Vec<Term>>),
    Error(WamError),
}
```

For WAM integration, builtins can be resolved during `Call(PredicateKey)`:

```text
call p/n:
  if builtin exists:
      execute builtin over argument registers
      return success/failure/choice answers
  else if program entry exists:
      jump to compiled code
  else if provider facts exist:
      materialized provider clauses
```

### 6.2 Builtin mode discipline

Some builtins are relational, some are directional.

| Builtin | Mode behavior |
|---|---|
| `=/2` | relational |
| `\=/2` | relational test |
| `is/2` | left output or bound, right arithmetic expression must be evaluable |
| `=:=/2` | both arithmetic expressions evaluable |
| `arg/3` | partially relational but MVP can require index + compound bound |
| `functor/3` | bidirectional in ISO; MVP can implement common modes first |
| `length/2` | relational but MVP can support list-bound or length-bound finite generation later |

When unsupported modes are called, return structured instantiation/type/domain errors.

## 7. Core builtin list

### 7.1 Control

| Predicate | Semantics |
|---|---|
| `true/0` | always succeeds |
| `fail/0`, `false/0` | always fails |
| `!/0` | compile to cut instruction |
| `once/1` | call goal, cut after first answer |
| `call/1` | call a callable term |
| `\+/1` | negation as failure; stratification check later |
| `;/2` | disjunction |
| `->/2` | if-then; later `*->/2` optional |

### 7.2 Unification and term tests

| Predicate | Semantics |
|---|---|
| `=/2` | unify |
| `\=/2` | succeeds if not unifiable |
| `==/2` | identical after deref, no binding |
| `\==/2` | not identical |
| `@</2`, `@=</2`, `@>/2`, `@>=/2` | standard term ordering |
| `compare/3` | returns `<`, `=`, `>` atom |

### 7.3 Type tests

```prolog
var/1
nonvar/1
atom/1
integer/1
float/1
number/1
atomic/1
compound/1
ground/1
```

### 7.4 Term decomposition

```prolog
functor(Term, Name, Arity).
arg(Index, Term, Arg).
Term =.. List.
copy_term(Term, Copy).
term_variables(Term, Vars).
```

MVP modes:

- `functor(+Term, -Name, -Arity)`
- `functor(-Term, +Name, +Arity)` creates term with fresh variables
- `arg(+Index, +Term, -Arg)`
- `=..(+Term, -List)` and `=..(-Term, +List)`

### 7.5 Arithmetic evaluation

```prolog
X is Expr.
A =:= B.
A =\= B.
A < B.
A =< B.
A > B.
A >= B.
```

Arithmetic expression evaluator:

```rust
enum NumberValue { Integer(i64), Float(f64) }

fn eval_arith(term: &Term, env: &Bindings) -> Result<NumberValue> {
    match term {
        Integer(i) => Integer(i),
        Float(f) => Float(f),
        Compound { name: "+", args: [a,b] } => eval(a) + eval(b),
        Compound { name: "-", args: [a,b] } => eval(a) - eval(b),
        Compound { name: "-", args: [a] } => -eval(a),
        Compound { name: "*", args: [a,b] } => eval(a) * eval(b),
        Compound { name: "/", args: [a,b] } => Float(eval(a) / eval(b)),
        Compound { name: "//", args: [a,b] } => integer_div(eval_int(a), eval_int(b)),
        Compound { name: "mod", args: [a,b] } => eval_int(a) mod eval_int(b),
        Compound { name: "abs", args: [a] } => abs(eval(a)),
        Variable(_) => instantiation_error,
        _ => type_error(evaluable, term),
    }
}
```

### 7.6 Arithmetic functions

Minimum:

```prolog
+Expr
-Expr
A + B
A - B
A * B
A / B
A // B
A mod B
A rem B
abs(X)
min(A,B)
max(A,B)
```

Later:

```prolog
sin/cos/tan/log/exp/sqrt/floor/ceiling/round/truncate
```

## 8. Meta-call and `call/N`

Meta-call is required for ordinary Prolog programming and for libraries such as `apply`, `maplist`, `include`, `exclude`, and DCG helper predicates.

### 8.1 Supported predicates

Minimum set:

```prolog
call(Goal).
call(Closure, A1).
call(Closure, A1, A2).
call(Closure, A1, A2, A3).
call(Closure, A1, A2, A3, A4).
```

Practical upper bound:

```prolog
call/1 .. call/8
```

SWI supports more via generated definitions, but `call/1..8` is enough for common libraries.

### 8.2 Callable terms

A term is callable if it is:

- an atom, e.g. `true`;
- a compound term, e.g. `member(X, Xs)`;
- a closure term to which `call/N` appends extra arguments.

Examples:

```prolog
call(true).
call(member(X), [a,b,c]).        % expands to member(X, [a,b,c])
call(edge(alice, knows), X).     % expands to edge(alice, knows, X)
```

Closure expansion algorithm:

```text
expand_call(Closure, ExtraArgs):
  if Closure is atom A:
      return compound(A, ExtraArgs)
  if Closure is compound F(Args):
      return compound(F, Args ++ ExtraArgs)
  if Closure is variable:
      instantiation_error
  otherwise:
      type_error(callable, Closure)
```

### 8.3 WAM integration strategy

There are two possible implementation paths.

#### Option A: Native meta-call builtin

`call/N` is a builtin implemented in Rust:

1. Read callable term from WAM registers.
2. Expand closure with extra arguments.
3. Convert expanded term to a `PredicateKey` and argument registers.
4. Dispatch through the same predicate resolution path used by ordinary `Call`:
   - compiled program entry;
   - builtin registry;
   - storage/index providers;
   - rule store/provider materialization.
5. Preserve caller continuation and choice point semantics.

This is the recommended MVP.

#### Option B: Compile-time expansion where statically known

If the compiler sees:

```prolog
call(foo(X)).
```

it can emit a direct `Call(foo/1)`.

This is an optimization only. The runtime builtin is still required for dynamic higher-order code.

### 8.4 Continuations and choice points

`call/N` must behave like a normal predicate call:

- if the called goal has multiple answers, `call/N` should expose them on backtracking;
- cut inside the called goal cuts only within that called predicate according to normal cut scope;
- caller choice points must remain intact unless cut semantics require otherwise.

Implementation detail:

```text
call/N builtin should not simply run a nested query to completion and return a Vec.
```

That would lose Prolog backtracking semantics. The MVP can initially materialize answer choice points, but long-term it should dispatch into the WAM call path.

### 8.5 Error behavior

| Case | Error |
|---|---|
| `call(X)` with X unbound | instantiation_error |
| `call(3)` | type_error(callable, 3) |
| expanded predicate missing | existence_error(procedure, Name/Arity) or fail depending unknown flag |

## 9. Standard library implementation model

There is no single universal ISO standard library equivalent to SWI's full library set. zlf should ship a practical compatibility library set and load it through a `library(...)` mechanism.

### 9.1 `library(Name)` is a source spec, not a normal predicate

In Prolog systems, `library(lists)` is normally used inside loading directives:

```prolog
:- use_module(library(lists)).
:- ensure_loaded(library(lists)).
```

zlf should support these directive forms:

```prolog
:- use_module(library(lists)).
:- ensure_loaded(library(lists)).
```

For a no-module MVP, `use_module/1` and `ensure_loaded/1` can both mean:

```text
load the named standard library into the current runtime if not already loaded
```

`library/1` itself should be parsed as a compound source spec:

```prolog
library(lists)
```

not as a callable runtime predicate.

### 9.2 Standard library registry

Add a registry:

```rust
struct StdLibRegistry {
    libs: HashMap<String, StdLibDescriptor>,
}

struct StdLibDescriptor {
    name: String,
    source_units: Vec<EmbeddedPrologSource>,
    native_predicates: Vec<PredicateKey>,
    dependencies: Vec<String>,
}
```

Runtime state:

```rust
struct LoadedLibraries {
    loaded: HashSet<String>,
}
```

Loading algorithm:

```text
load_library(Name):
  if Name already loaded: return ok
  desc = registry.get(Name) else existence_error(source_sink, library(Name))
  for dep in desc.dependencies:
      load_library(dep)
  register desc.native_predicates with builtin registry
  parse desc.source_units
  compile source rules into system rule store / in-memory rule set
  mark Name loaded
```

### 9.3 Rust builtin vs Prolog source decision

Use three implementation classes.

#### Class A: Rust native builtins

Use Rust for predicates that are:

- arithmetic or type-sensitive;
- impure or runtime-inspective;
- require WAM heap/control integration;
- performance-critical with complex modes.

Examples:

```prolog
is/2
=:=/2
var/1
nonvar/1
functor/3
arg/3
=../2
call/N
findall/3
length/2  % recommended native or hybrid
sort/2
keysort/2
assertz/1
retract/1
```

#### Class B: Prolog source libraries

Use embedded Prolog source for pure relational predicates that naturally express recursion.

Examples:

```prolog
member/2
append/3
select/3
prefix/2
suffix/2
sublist/2
maplist/2..N  % after call/N
include/3
exclude/3
foldl/4
```

Example embedded source:

```prolog
member(X, [X|_]).
member(X, [_|Xs]) :- member(X, Xs).

append([], Ys, Ys).
append([X|Xs], Ys, [X|Zs]) :- append(Xs, Ys, Zs).
```

#### Class C: Hybrid predicates

Use a Rust fast path plus Prolog fallback when modes are relational.

Examples:

```prolog
length/2
reverse/2
permutation/2
```

For `length/2`:

- `length(+List, -N)` native count;
- `length(-List, +N)` native finite generation;
- `length(-List, -N)` should throw instantiation_error or be deferred to avoid infinite generation.

### 9.4 Source layout

Recommended crate layout:

```text
crates/zlf-prolog/src/stdlib/
  mod.rs
  registry.rs
  builtins.rs
  sources/
    lists.pl
    apply.pl
    ordsets.pl
    dcg.pl
```

Embed Prolog sources:

```rust
const LISTS_PL: &str = include_str!("sources/lists.pl");
```

Compile these into a system rule layer:

```text
builtin registry -> system stdlib rules -> user persisted rules -> storage/index providers
```

Precedence recommendation:

1. core builtins;
2. loaded stdlib rules;
3. user rules;
4. provider facts.

Do not persist stdlib rules into RocksDB by default. They are versioned with the binary and loaded into runtime memory. Persist only the user's `use_module/1` directive if source files later become user-loadable.

### 9.5 `library(lists)`

Required predicates:

```prolog
member/2
append/3
select/3
nth0/3
nth1/3
length/2
reverse/2
last/2
prefix/2
suffix/2
sublist/2
permutation/2
sort/2
msort/2
keysort/2
```

MVP order:

1. `member/2`
2. `append/3`
3. `length/2`
4. `reverse/2`
5. `select/3`
6. `nth0/3`, `nth1/3`

Some should be native for performance and mode safety, especially `length/2`, `sort/2`, `keysort/2`.

### 9.6 `library(apply)` subset

```prolog
maplist/2
maplist/3
maplist/4
include/3
exclude/3
foldl/4
```

Requires `call/N`.

Example source:

```prolog
maplist(_, []).
maplist(P, [X|Xs]) :- call(P, X), maplist(P, Xs).

include(_, [], []).
include(P, [X|Xs], [X|Ys]) :- call(P, X), !, include(P, Xs, Ys).
include(P, [_|Xs], Ys) :- include(P, Xs, Ys).
```

### 9.7 `library(ordsets)` subset

```prolog
list_to_ord_set/2
ord_memberchk/2
ord_union/3
ord_intersection/3
ord_subtract/3
```

Use Rust native `sort/2` to implement `list_to_ord_set/2`, then Prolog source for simple set operations.

### 9.8 Aggregation subset

```prolog
findall/3
bagof/3
setof/3
aggregate_all/3  % later
```

`findall/3` should come before `bagof/3` and `setof/3`.

Implementation idea:

```text
findall(Template, Goal, Bag):
  run Goal collecting all Template instances
  copy each Template instance out of the WAM heap
  bind Bag to canonical list(instances)
```

`findall/3` is best implemented as Rust native because it needs controlled nested execution and term copying.

## 10. DCG implementation

DCG is essential for practical Prolog parsing code. It should be implemented as a source-to-source expansion before WAM rule compilation.

### 10.1 Surface syntax

Examples:

```prolog
sentence --> noun_phrase, verb_phrase.
noun_phrase --> determiner, noun.
determiner --> [the].
noun --> [cat].
```

Query via `phrase/2`:

```prolog
?- phrase(sentence, [the, cat]).
```

### 10.2 Core expansion model

A DCG nonterminal `p//N` expands to predicate `p/(N+2)` with difference-list arguments.

```prolog
sentence --> noun_phrase, verb_phrase.
```

expands to:

```prolog
sentence(S0, S) :-
    noun_phrase(S0, S1),
    verb_phrase(S1, S).
```

A nonterminal with arguments:

```prolog
integer(N) --> digits(Ds), { number_codes(N, Ds) }.
```

expands to:

```prolog
integer(N, S0, S) :-
    digits(Ds, S0, S1),
    number_codes(N, Ds),
    S1 = S.
```

### 10.3 Terminal expansion

Terminals are list matches.

```prolog
[a]
```

expands to:

```prolog
S0 = [a|S]
```

Multiple terminals:

```prolog
[a,b,c]
```

expands to:

```prolog
S0 = [a,b,c|S]
```

or a sequence of cons unifications.

### 10.4 Embedded Prolog goals

Curly goals are copied into the body without consuming input:

```prolog
number(N) --> [N], { integer(N) }.
```

expands to:

```prolog
number(N, S0, S) :-
    S0 = [N|S1],
    integer(N),
    S1 = S.
```

### 10.5 DCG body expansion algorithm

```text
expand_dcg_rule(Head --> Body):
  create fresh variables S0, S
  expanded_head = append_args(Head, [S0, S])
  expanded_body = expand_dcg_sequence(Body, S0, S)
  return Rule(expanded_head :- expanded_body)
```

For body sequence:

```text
expand_dcg_sequence((A, B), S0, S):
  fresh S1
  expand A from S0 to S1
  expand B from S1 to S
```

For alternatives:

```text
expand_dcg_sequence((A ; B), S0, S):
  (expand A S0 S ; expand B S0 S)
```

For nonterminal call:

```text
expand_dcg_goal(p(Args), S0, S):
  p(Args..., S0, S)

expand_dcg_goal(atom_nonterminal, S0, S):
  atom_nonterminal(S0, S)
```

For terminal list:

```text
expand_dcg_goal([T1,T2,...], S0, S):
  S0 = [T1,T2,...|S]
```

For embedded goal:

```text
expand_dcg_goal({Goal}, S0, S):
  Goal, S0 = S
```

### 10.6 `phrase/2` and `phrase/3`

```prolog
phrase(GrammarBody, List).
phrase(GrammarBody, List, Rest).
```

Semantics:

```prolog
phrase(Body, List) :- phrase(Body, List, []).
phrase(Body, List, Rest) :- call(expanded_body(Body, List, Rest)).
```

Implementation options:

1. Native Rust builtin expands grammar body at runtime and calls it.
2. Prolog source wrapper plus native helper.

Recommended MVP:

- compile DCG rules at load time into ordinary rules;
- implement `phrase/2` and `phrase/3` as Rust native builtins that append difference-list args and meta-call the expanded nonterminal.

### 10.7 Required parser support for DCG

Add parsing for:

```prolog
Head --> Body.
{ Goal }
terminal lists inside DCG body
```

Represent DCG rules before expansion:

```rust
struct DcgRule {
    head: Term,
    body: Term,
}
```

Then expand to `PrologRule` before WAM codegen.

### 10.8 DCG verification cases

```prolog
det --> [the].
noun --> [cat].
np --> det, noun.
```

Tests:

```prolog
?- phrase(np, [the, cat]).
true.

?- phrase(np, [the, dog]).
false.

?- phrase(np, [the, cat, sleeps], Rest).
Rest = [sleeps].
```

## 11. String/atom library subset

```prolog
atom_string/2
atom_chars/2
atom_codes/2
string_chars/2
string_codes/2
number_string/2
sub_atom/5
atom_concat/3
string_concat/3
```

## 12. Dynamic database predicates

zlf already has storage-backed fact writing. ISO-style dynamic DB should map to it carefully.

### 12.1 Declarations

```prolog
:- dynamic p/2.
```

For zlf graph facts, builtins such as `node/1`, `edge/3`, `label/2`, `property/3` are already dynamic provider predicates.

### 12.2 Assert

```prolog
assertz(FactOrRule).
asserta(FactOrRule).
```

MVP:

- graph facts go through `StorageFactWriter`;
- rules go through `StorageRuleStore`;
- `asserta` and `assertz` can initially behave the same if rule ordering is not implemented;
- later preserve clause order.

### 12.3 Retract

```prolog
retract(Head).
retract((Head :- Body)).
retractall(Head).
```

MVP:

- support graph fact retraction first;
- support user rule retraction by exact source/hash later;
- `retractall/1` removes all matching graph facts/rules.

### 12.4 Clause inspection

```prolog
clause(Head, Body).
current_predicate(Name/Arity).
```

Map to rule store + predicate registry.

## 13. Error model

Use ISO-style error categories internally even if the Rust error enum remains compact.

```prolog
instantiation_error
uninstantiation_error
TypeError(Expected, Actual)
domain_error(Domain, Culprit)
existence_error(ObjectType, Culprit)
permission_error(Operation, PermissionType, Culprit)
evaluation_error(Error)
representation_error(Flag)
syntax_error(Message)
```

Examples:

| Query | Error |
|---|---|
| `X is Y + 1` where `Y` unbound | instantiation_error |
| `X is alice + 1` | type_error(evaluable, alice) |
| `arg(0, f(a), X)` | domain_error(not_less_than_one, 0) |
| `unknown_predicate(X)` depending flag | existence_error(procedure, unknown_predicate/1) |

## 14. Implementation phases

### Phase ISO-0: parser and term model foundation

- Split integer/float terms.
- Add quoted atoms.
- Add canonical list AST with tail support.
- Lower lists to `[]` and `'.'/2`.
- Add operator parser for core operators.
- Add directive/query item parser.

### Phase ISO-1: builtin execution layer

- Add builtin registry.
- Add call path for deterministic builtins.
- Add unification/test/type builtins.
- Add arithmetic evaluator and arithmetic comparisons.
- Add structured ISO-style errors.

### Phase ISO-2: list/string standard predicates

- Implement `library(lists)` MVP.
- Implement string/atom/code conversion predicates.
- Add list pattern matching tests: `[H|T]`, `[a,b|T]`, nested lists.

### Phase ISO-3: control and meta-call

- Implement disjunction and if-then parser/codegen.
- Implement `\+/1` with current NAF semantics.
- Add `once/1`.
- Add `call/1` and later `call/N`.

### Phase ISO-4: dynamic database compatibility

- Implement `assertz/1`, `asserta/1`, `retract/1`, `retractall/1`.
- Map graph facts to storage writer/deleter.
- Map rules to rule store.
- Implement `clause/2`, `current_predicate/1` over registry.

### Phase ISO-5: common libraries

- `findall/3`.
- `bagof/3`, `setof/3` later.
- `library(apply)` subset.
- `library(ordsets)` subset.
- DCG basics.

## 15. Verification strategy

### 15.1 ISO-style unit tests

Create tests by category:

```text
crates/zlf-prolog/tests/iso_terms.rs
crates/zlf-prolog/tests/iso_arithmetic.rs
crates/zlf-prolog/tests/iso_lists.rs
crates/zlf-prolog/tests/iso_control.rs
crates/zlf-prolog/tests/iso_dynamic.rs
crates/zlf-prolog/tests/iso_libraries.rs
```

### 15.2 Example test cases

Lists:

```prolog
?- [H|T] = [a,b,c].
H = a, T = [b,c].
```

Arithmetic:

```prolog
?- X is 1 + 2 * 3.
X = 7.

?- 2 + 3 =:= 5.
true.
```

Type tests:

```prolog
?- atom(alice).
true.

?- number(3.14).
true.
```

Term decomposition:

```prolog
?- functor(parent(alice,bob), Name, Arity).
Name = parent, Arity = 2.

?- parent(alice,bob) =.. L.
L = [parent, alice, bob].
```

Dynamic DB:

```prolog
?- assertz(likes(alice, tea)).
?- likes(alice, X).
X = tea.
?- retract(likes(alice, tea)).
?- likes(alice, tea).
false.
```

Findall:

```prolog
?- findall(X, member(X, [a,b,c]), Xs).
Xs = [a,b,c].
```

## 16. Recommendation

Add ISO/general Prolog support as a parallel track after Stage 0 fact mutation begins, but before full tabling. The best next order is:

1. canonical list representation and `[H|T]` matching;
2. arithmetic evaluator and arithmetic comparisons;
3. type/term builtins;
4. list library MVP;
5. dynamic database predicates mapped to zlf storage/rules;
6. control/meta-call;
7. findall/aggregation;
8. DCG and larger libraries.

Reason: proper lists, arithmetic, and control are prerequisites for many standard-library predicates and for writing useful recursive Prolog programs.
