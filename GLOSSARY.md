# rx-rust

A ReactiveX implementation for Rust: observables, the operators that transform and combine
them, and the observers they deliver to.

## Combining sources

**Source set**:
The fixed collection of observables a combining operator subscribes, given in one of two forms.
_Avoid_: sources list, source collection

**Tuple form**:
A source set whose number of observables is known when the program is written, each with its
own item type.
_Avoid_: static form, heterogeneous sources

**List form**:
A source set whose number of observables is decided as the program runs, all of one type.
_Avoid_: dynamic form, iterator form, `*_all` form

**Position**:
The place of one observable in its source set, and of everything the operator keeps for it.
_Avoid_: index, key, slot

**Position count**:
How many positions a source set has, known before any of its observables is subscribed.
_Avoid_: len, membership, source count

**Subscribed count**:
How many observables of a source set were actually subscribed; smaller than the position count
when the operator stops while subscribing.
_Avoid_: len, active sources

**Combination**:
One output made of an item from every position of a source set.
_Avoid_: snapshot, tuple of latest

**Ready**:
The state of a source set in which every position has at least one value, so a combination can
be made.
_Avoid_: complete, full
