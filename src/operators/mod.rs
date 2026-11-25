//! The operators, one type per file, in the categories of
//! <https://reactivex.io/documentation/operators.html>.
//!
//! Sources are the types of [`creating`]; every other operator wraps a source and is normally
//! reached through the method of the same name on
//! [`ObservableExt`](crate::observable::ObservableExt). Each type's documentation has an example.
//!
//! # Finding an operator
//!
//! By what it is called elsewhere: the search box of this documentation knows the names of other
//! ReactiveX libraries and of Rust's iterators and streams (`tap` finds `do_before_next`, `race`
//! finds `amb`). By what it does: the tables below, after the [decision tree] of ReactiveX. By
//! category: the [last table](#by-category).
//!
//! [decision tree]: https://reactivex.io/documentation/operators.html#tree
//!
//! ## Create an observable
//!
//! | I want | Use |
//! |---|---|
//! | one value | [`Just`] |
//! | the items of a collection or an iterator | [`FromIter`] |
//! | a range of integers | [`Range`] |
//! | a state stepped by a function | [`FromIter`] over [`successors`](std::iter::successors) |
//! | the same value several times | [`Repeat`] |
//! | the result of a function called at the subscription | [`Start`] |
//! | a `Result`: its value or its error | [`FromResult`] |
//! | a future | [`FromFuture`], [`FromTryFuture`] |
//! | an async stream | `FromStream`, `FromTryStream`, or `FromStreamAcked`, `FromTryStreamAcked` with backpressure (feature `futures`) |
//! | a value after a delay, or a counter at every period | [`Timer`], [`Interval`] |
//! | nothing: complete at once, fail at once, or stay silent | [`Empty`], [`Throw`], [`Never`] |
//! | a callback API, or custom logic | [`Create`] |
//! | an observable built anew for each subscription | [`Defer`], or [`Using`] when it needs a resource |
//! | events pushed by hand, to several observers | a [`subject`](crate::subject) |
//!
//! ## Combine observables
//!
//! | I want | Use |
//! |---|---|
//! | interleave their items | [`merge`] |
//! | one after the other | [`concat`]; [`concat_eager`] subscribes them all at once |
//! | values before the first item, or after the last | [`start_with`], [`concat`] |
//! | the first one to emit, dropping the others | [`amb`] |
//! | pair their items by position | [`zip`] |
//! | the latest of each, whenever any of them emits | [`combine_latest`] |
//! | the latest of the others, whenever the source emits | [`with_latest_from`] |
//! | the last of each, once all complete | [`fork_join`] |
//! | pair the items whose time windows overlap | [`join`], [`group_join`] |
//! | several patterns of sources | [`When`] |
//!
//! ## Transform each item
//!
//! | I want | Use |
//! |---|---|
//! | with a function | [`map`] |
//! | with a function that may drop the item | [`filter_map`] |
//! | with a function and a running state | [`scan`] |
//! | pair it with the one before | [`pairwise`] |
//! | pair it with the time it arrived, or the time since the last | [`timestamp`], [`time_interval`] |
//!
//! ## Turn each item into an observable, such as a request
//!
//! | I want | Use |
//! |---|---|
//! | run them all at once, items in any order | [`flat_map`]; [`max_concurrency`](transforming::flat_map::FlatMap::max_concurrency) bounds how many |
//! | run one at a time, in order | [`concat_map`], whose order is that of `Iterator::flat_map` |
//! | run them all at once, items in order | [`concat_eager_map`] |
//! | drop the running one when a new item arrives | [`switch_map`] |
//! | ignore new items while one is running | [`exhaust_map`] |
//! | flatten observables that are already items | [`merge_all`], [`concat_all`], [`concat_eager_all`], [`switch`], [`exhaust_all`] |
//!
//! ## Keep only some items
//!
//! | I want | Use |
//! |---|---|
//! | those that satisfy a predicate | [`filter`] |
//! | the first, the last, or the only one | [`first`], [`last`], [`single`] |
//! | the first that satisfies a predicate | [`filter`], then [`first`] |
//! | the one at an index | [`element_at`] |
//! | the first or the last n | [`take`], [`take_last`] |
//! | all but the first or the last n | [`skip`], [`skip_last`] |
//! | until, or from, another observable emits (a [`Timer`], for a time) | [`take_until`], [`skip_until`] |
//! | while, or from the first that does not satisfy, a predicate | [`take_while`], [`skip_while`] |
//! | without duplicates | [`distinct`], [`distinct_with_key_selector`] |
//! | without consecutive duplicates | [`distinct_until_changed`], [`distinct_until_changed_with_key_selector`] |
//! | none, only the termination | [`ignore_elements`] |
//!
//! ## Limit the rate
//!
//! | I want | Use |
//! |---|---|
//! | once the source has paused, as for a search box | [`debounce`] |
//! | the first item of each span | [`throttle`] |
//! | the last item of each span, once it ends | [`audit`] |
//! | the first item at once, then the last of each span | [`throttle_latest`] |
//! | the latest item at every period, or at another observable's items | [`sample`] (with an [`Interval`]) |
//! | in batches | [`buffer_with_time`], [`buffer_with_count`], [`buffer_with_time_or_count`] |
//!
//! ## Group items
//!
//! | I want | Use |
//! |---|---|
//! | in `Vec`s, delimited by another observable, a count or a time | [`buffer`], [`buffer_with_count`], [`buffer_with_time`], [`buffer_with_time_or_count`] |
//! | in overlapping `Vec`s, or with gaps | [`buffer_with_count_and_skip`], [`buffer_with_time_and_shift`] |
//! | as above, in observables | [`window`] and the `window_with_*` methods |
//! | by key, one observable per key | [`group_by`] |
//!
//! ## Reduce the whole sequence to one value
//!
//! | I want | Use |
//! |---|---|
//! | a `Vec`, or another collection | [`to_vec`], [`collect`] |
//! | the last n items, as a `Vec` | [`take_last`], then [`to_vec`] |
//! | with an accumulator | [`reduce`] |
//! | count, sum, average, minimum, maximum | [`count`], [`sum`], [`average`], [`min`], [`max`] |
//! | whether every item satisfies a predicate | [`all`] |
//! | whether any item does | [`map`] to a `bool`, then `contains(true)` |
//! | whether it contains an item, or is empty | [`contains`], `all(\|_\| false)` |
//! | whether two sequences are equal | [`sequence_equal`] |
//!
//! ## Handle an error, or an empty sequence
//!
//! | I want | Use |
//! |---|---|
//! | continue with another observable, or a value | [`catch`] (with a [`Just`]) |
//! | try again, at once or after a delay | [`retry`] (with [`delay_subscription`]) |
//! | change the error | [`map_err`] |
//! | fail when items are late | [`timeout`], then [`catch`] to fall back |
//! | a value, or an observable, in place of an empty sequence | [`default_if_empty`], [`switch_if_empty`] |
//! | start over after a completion | [`repeat_source`] |
//!
//! ## Choose when and where things run
//!
//! | I want | Use |
//! |---|---|
//! | emit items later | [`delay`] |
//! | subscribe later | [`delay_subscription`] |
//! | subscribe on a scheduler | [`subscribe_on`] |
//! | receive events on a scheduler | [`observe_on`] |
//!
//! ## Share one subscription among several observers
//!
//! | I want | Use |
//! |---|---|
//! | while there are any | [`share`] |
//! | and give late observers the earlier items | [`share_replay`] |
//! | and give late observers the current value, or only the last | [`share_behavior`], [`share_last`] |
//! | and keep it when nobody is left, as a cache does | [`replay`], then [`ref_count`](connectable::connectable_controller::ConnectableController::ref_count) with the `reset_on_*` options |
//! | started by hand | [`publish`], [`publish_behavior`], [`publish_last`], [`replay`] |
//! | through a subject of your own | [`multicast`] |
//!
//! ## Observe a stream, or act on its events
//!
//! | I want | Use |
//! |---|---|
//! | look at each item | [`do_before_next`], [`do_after_next`] |
//! | act when it ends, whatever ended it | [`do_finally`] |
//! | act on a completion or an error | [`do_before_termination`], [`do_after_termination`] |
//! | act on a subscription or a disposal | [`do_before_subscription`], [`do_after_subscription`], [`do_before_disposal`], [`do_after_disposal`] |
//! | print every event | [`debug_to_stdout`], [`debug`] |
//! | keep a resource for as long as the subscription | [`hold`], [`Using`] |
//! | turn events into items, and back | [`materialize`], [`dematerialize`] |
//! | intercept an event and decide what to forward | [`hook_on_next`], [`hook_on_termination`], [`hook_on_subscription`] |
//!
//! ## Leave the world of observables, or change the type
//!
//! | I want | Use |
//! |---|---|
//! | call a closure for each item | [`subscribe_with_callback`] |
//! | await the first item, or the last | [`into_future`], [`into_try_future`]; [`last`], then one of them |
//! | consume as an async stream | `into_stream`, `into_try_stream` (feature `futures`) |
//! | store observables of different types together | [`into_boxed`], [`into_cloneable_boxed`] |
//! | join a source that never fails, or never emits, to others | [`with_error_type`], [`with_item_type`] |
//! | make a `Local` source `Shared` | [`into_shared`] |
//!
//! # By category
//!
//! | Module | Operators |
//! |---|---|
//! | [`creating`] | [`Create`], [`Defer`], [`Empty`], [`FromFuture`], [`FromTryFuture`], [`FromIter`], [`FromResult`], `FromStream`, `FromTryStream`, `FromStreamAcked`, `FromTryStreamAcked`, [`Interval`], [`Just`], [`Never`], [`Range`], [`Repeat`], [`Start`], [`Throw`], [`Timer`] |
//! | [`transforming`] | [`map`], [`scan`], [`pairwise`], [`flat_map`], [`concat_map`], [`concat_eager_map`], [`switch_map`], [`exhaust_map`], [`group_by`], [`buffer`], [`buffer_with_count`], [`buffer_with_count_and_skip`], [`buffer_with_time`], [`buffer_with_time_and_shift`], [`buffer_with_time_or_count`], [`window`], [`window_with_count`], [`window_with_count_and_skip`], [`window_with_time`], [`window_with_time_and_shift`], [`window_with_time_or_count`] |
//! | [`filtering`] | [`filter`], [`filter_map`], [`first`], [`last`], [`single`], [`element_at`], [`take`], [`take_last`], [`skip`], [`skip_last`], [`distinct`], [`distinct_with_key_selector`], [`distinct_until_changed`], [`distinct_until_changed_with_key_selector`], [`debounce`], [`audit`], [`throttle`], [`throttle_latest`], [`sample`], [`ignore_elements`] |
//! | [`combining`] | [`merge`], [`merge_all`], [`concat`], [`concat_all`], [`concat_eager`], [`concat_eager_all`], [`switch`], [`exhaust_all`], [`zip`], [`combine_latest`], [`with_latest_from`], [`fork_join`], [`join`], [`group_join`], [`When`], [`start_with`] |
//! | [`conditional_boolean`] | [`all`], [`contains`], [`sequence_equal`], [`default_if_empty`], [`switch_if_empty`], [`amb`], [`take_while`], [`take_until`], [`skip_while`], [`skip_until`] |
//! | [`mathematical_aggregate`] | [`count`], [`sum`], [`average`], [`min`], [`max`], [`reduce`], [`collect`], [`to_vec`] |
//! | [`error_handling`] | [`catch`], [`map_err`], [`retry`] |
//! | [`utility`] | [`delay`], [`delay_subscription`], [`timeout`], [`timestamp`], [`time_interval`], [`hold`], [`do_finally`], [`repeat_source`], [`stop_on_disposal`], [`materialize`], [`dematerialize`], [`subscribe_on`], [`observe_on`], [`Using`], `do_before_*`, `do_after_*` |
//! | [`connectable`] | [`multicast`], [`publish`], [`publish_behavior`], [`publish_last`], [`replay`], `ref_count`, [`share`], [`share_behavior`], [`share_last`], [`share_replay`] |
//! | [`others`] | [`into_shared`], [`with_item_type`], [`with_error_type`], [`debug`], [`debug_to_stdout`], `hook_on_*` |
//!
//! [`Create`]: creating::create::Create
//! [`Defer`]: creating::defer::Defer
//! [`Empty`]: creating::empty::Empty
//! [`FromFuture`]: creating::from_future::FromFuture
//! [`FromIter`]: creating::from_iter::FromIter
//! [`FromResult`]: creating::from_result::FromResult
//! [`FromTryFuture`]: creating::from_try_future::FromTryFuture
//! [`Interval`]: creating::interval::Interval
//! [`Just`]: creating::just::Just
//! [`Never`]: creating::never::Never
//! [`Range`]: creating::range::Range
//! [`Repeat`]: creating::repeat::Repeat
//! [`Start`]: creating::start::Start
//! [`Throw`]: creating::throw::Throw
//! [`Timer`]: creating::timer::Timer
//! [`Using`]: utility::using::Using
//! [`When`]: combining::when::When
//! [`all`]: crate::observable::ObservableExt::all
//! [`amb`]: crate::observable::ObservableExt::amb
//! [`audit`]: crate::observable::ObservableExt::audit
//! [`average`]: crate::observable::ObservableExt::average
//! [`buffer`]: crate::observable::ObservableExt::buffer
//! [`buffer_with_count`]: crate::observable::ObservableExt::buffer_with_count
//! [`buffer_with_count_and_skip`]: crate::observable::ObservableExt::buffer_with_count_and_skip
//! [`buffer_with_time`]: crate::observable::ObservableExt::buffer_with_time
//! [`buffer_with_time_and_shift`]: crate::observable::ObservableExt::buffer_with_time_and_shift
//! [`buffer_with_time_or_count`]: crate::observable::ObservableExt::buffer_with_time_or_count
//! [`catch`]: crate::observable::ObservableExt::catch
//! [`collect`]: crate::observable::ObservableExt::collect
//! [`combine_latest`]: crate::observable::ObservableExt::combine_latest
//! [`concat`]: crate::observable::ObservableExt::concat
//! [`concat_all`]: crate::observable::ObservableExt::concat_all
//! [`concat_eager`]: crate::observable::ObservableExt::concat_eager
//! [`concat_eager_all`]: crate::observable::ObservableExt::concat_eager_all
//! [`concat_eager_map`]: crate::observable::ObservableExt::concat_eager_map
//! [`concat_map`]: crate::observable::ObservableExt::concat_map
//! [`contains`]: crate::observable::ObservableExt::contains
//! [`count`]: crate::observable::ObservableExt::count
//! [`debounce`]: crate::observable::ObservableExt::debounce
//! [`debug`]: crate::observable::ObservableExt::debug
//! [`debug_to_stdout`]: crate::observable::ObservableExt::debug_to_stdout
//! [`default_if_empty`]: crate::observable::ObservableExt::default_if_empty
//! [`delay`]: crate::observable::ObservableExt::delay
//! [`delay_subscription`]: crate::observable::ObservableExt::delay_subscription
//! [`dematerialize`]: crate::observable::ObservableExt::dematerialize
//! [`distinct`]: crate::observable::ObservableExt::distinct
//! [`distinct_until_changed`]: crate::observable::ObservableExt::distinct_until_changed
//! [`distinct_until_changed_with_key_selector`]: crate::observable::ObservableExt::distinct_until_changed_with_key_selector
//! [`distinct_with_key_selector`]: crate::observable::ObservableExt::distinct_with_key_selector
//! [`do_after_disposal`]: crate::observable::ObservableExt::do_after_disposal
//! [`do_after_next`]: crate::observable::ObservableExt::do_after_next
//! [`do_after_subscription`]: crate::observable::ObservableExt::do_after_subscription
//! [`do_after_termination`]: crate::observable::ObservableExt::do_after_termination
//! [`do_before_disposal`]: crate::observable::ObservableExt::do_before_disposal
//! [`do_before_next`]: crate::observable::ObservableExt::do_before_next
//! [`do_before_subscription`]: crate::observable::ObservableExt::do_before_subscription
//! [`do_before_termination`]: crate::observable::ObservableExt::do_before_termination
//! [`do_finally`]: crate::observable::ObservableExt::do_finally
//! [`element_at`]: crate::observable::ObservableExt::element_at
//! [`exhaust_all`]: crate::observable::ObservableExt::exhaust_all
//! [`exhaust_map`]: crate::observable::ObservableExt::exhaust_map
//! [`filter`]: crate::observable::ObservableExt::filter
//! [`filter_map`]: crate::observable::ObservableExt::filter_map
//! [`first`]: crate::observable::ObservableExt::first
//! [`flat_map`]: crate::observable::ObservableExt::flat_map
//! [`fork_join`]: crate::observable::ObservableExt::fork_join
//! [`group_by`]: crate::observable::ObservableExt::group_by
//! [`group_join`]: crate::observable::ObservableExt::group_join
//! [`hold`]: crate::observable::ObservableExt::hold
//! [`hook_on_next`]: crate::observable::ObservableExt::hook_on_next
//! [`hook_on_subscription`]: crate::observable::ObservableExt::hook_on_subscription
//! [`hook_on_termination`]: crate::observable::ObservableExt::hook_on_termination
//! [`ignore_elements`]: crate::observable::ObservableExt::ignore_elements
//! [`into_boxed`]: crate::observable::ObservableExt::into_boxed
//! [`into_cloneable_boxed`]: crate::observable::ObservableExt::into_cloneable_boxed
//! [`into_future`]: crate::observable::ObservableExt::into_future
//! [`into_shared`]: crate::observable::ObservableExt::into_shared
//! [`into_try_future`]: crate::observable::ObservableExt::into_try_future
//! [`join`]: crate::observable::ObservableExt::join
//! [`last`]: crate::observable::ObservableExt::last
//! [`map`]: crate::observable::ObservableExt::map
//! [`map_err`]: crate::observable::ObservableExt::map_err
//! [`materialize`]: crate::observable::ObservableExt::materialize
//! [`max`]: crate::observable::ObservableExt::max
//! [`merge`]: crate::observable::ObservableExt::merge
//! [`merge_all`]: crate::observable::ObservableExt::merge_all
//! [`min`]: crate::observable::ObservableExt::min
//! [`multicast`]: crate::observable::ObservableExt::multicast
//! [`observe_on`]: crate::observable::ObservableExt::observe_on
//! [`pairwise`]: crate::observable::ObservableExt::pairwise
//! [`publish`]: crate::observable::ObservableExt::publish
//! [`publish_behavior`]: crate::observable::ObservableExt::publish_behavior
//! [`publish_last`]: crate::observable::ObservableExt::publish_last
//! [`reduce`]: crate::observable::ObservableExt::reduce
//! [`repeat_source`]: crate::observable::ObservableExt::repeat_source
//! [`replay`]: crate::observable::ObservableExt::replay
//! [`retry`]: crate::observable::ObservableExt::retry
//! [`sample`]: crate::observable::ObservableExt::sample
//! [`scan`]: crate::observable::ObservableExt::scan
//! [`sequence_equal`]: crate::observable::ObservableExt::sequence_equal
//! [`share`]: crate::observable::ObservableExt::share
//! [`share_behavior`]: crate::observable::ObservableExt::share_behavior
//! [`share_last`]: crate::observable::ObservableExt::share_last
//! [`share_replay`]: crate::observable::ObservableExt::share_replay
//! [`single`]: crate::observable::ObservableExt::single
//! [`skip`]: crate::observable::ObservableExt::skip
//! [`skip_last`]: crate::observable::ObservableExt::skip_last
//! [`skip_until`]: crate::observable::ObservableExt::skip_until
//! [`skip_while`]: crate::observable::ObservableExt::skip_while
//! [`start_with`]: crate::observable::ObservableExt::start_with
//! [`stop_on_disposal`]: crate::observable::ObservableExt::stop_on_disposal
//! [`subscribe_on`]: crate::observable::ObservableExt::subscribe_on
//! [`subscribe_with_callback`]: crate::observable::ObservableExt::subscribe_with_callback
//! [`sum`]: crate::observable::ObservableExt::sum
//! [`switch`]: crate::observable::ObservableExt::switch
//! [`switch_if_empty`]: crate::observable::ObservableExt::switch_if_empty
//! [`switch_map`]: crate::observable::ObservableExt::switch_map
//! [`take`]: crate::observable::ObservableExt::take
//! [`take_last`]: crate::observable::ObservableExt::take_last
//! [`take_until`]: crate::observable::ObservableExt::take_until
//! [`take_while`]: crate::observable::ObservableExt::take_while
//! [`throttle`]: crate::observable::ObservableExt::throttle
//! [`throttle_latest`]: crate::observable::ObservableExt::throttle_latest
//! [`time_interval`]: crate::observable::ObservableExt::time_interval
//! [`timeout`]: crate::observable::ObservableExt::timeout
//! [`timestamp`]: crate::observable::ObservableExt::timestamp
//! [`to_vec`]: crate::observable::ObservableExt::to_vec
//! [`window`]: crate::observable::ObservableExt::window
//! [`window_with_count`]: crate::observable::ObservableExt::window_with_count
//! [`window_with_count_and_skip`]: crate::observable::ObservableExt::window_with_count_and_skip
//! [`window_with_time`]: crate::observable::ObservableExt::window_with_time
//! [`window_with_time_and_shift`]: crate::observable::ObservableExt::window_with_time_and_shift
//! [`window_with_time_or_count`]: crate::observable::ObservableExt::window_with_time_or_count
//! [`with_error_type`]: crate::observable::ObservableExt::with_error_type
//! [`with_item_type`]: crate::observable::ObservableExt::with_item_type
//! [`with_latest_from`]: crate::observable::ObservableExt::with_latest_from
//! [`zip`]: crate::observable::ObservableExt::zip

pub mod combining;
pub mod conditional_boolean;
pub mod connectable;
pub mod creating;
pub mod error_handling;
pub mod filtering;
pub mod mathematical_aggregate;
pub mod others;
pub mod transforming;
pub mod utility;
