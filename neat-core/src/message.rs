use std::marker::PhantomData;
use std::net::SocketAddr;

use crate::app::FunctionalState;

pub use crate::app::Message as App;

/// Lifting to allow some part of input "bypass" the functional state and still
/// get preserved in the output without the state being aware of it.
///
/// One common kind of lifting is functor instance i.e. `fmap` in Haskell, which
/// lift `M -> N` into some `F<M> -> F<N>`. But in general lifting does not
/// require the input `M` and the output `Self::Out` to have the same
/// constructor.
pub trait Lift<S, M> {
    type Out<'output>
    where
        Self: 'output,
        S: 'output;

    fn update<'a>(&'a mut self, state: &'a mut S, message: M) -> Self::Out<'a>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct OptionLift;

impl<S, M> Lift<S, Option<M>> for OptionLift
where
    S: FunctionalState<M>,
{
    type Out<'o> = Option<S::Output<'o>> where Self: 'o, S: 'o;

    fn update<'a>(&'a mut self, state: &'a mut S, message: Option<M>) -> Self::Out<'a> {
        message.map(|message| state.update(message))
    }
}

pub use crate::barrier::Message as Barrier;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct BarrierLift;

// TODO
// impl<S, M> Lift<S, Barrier<M>> for BarrierLift
// where
//     S: FunctionalState<M>,
// {
//     type Out<'o> = Barrier<S::Output<'o>> where Self: 'o, S: 'o;

//     fn update<'a>(&'a mut self, state: &'a mut S, message: Barrier<M>) -> Self::Out<'a> {
//         let mut output = Vec::new();
//         for (message, host) in message {
//             output.push((state.update(message), host))
//         }
//         output
//     }
// }

pub use crate::dispatch::Message as Dispatch;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct DispatchLift;
// TODO

/// Rich-featured timeout protocol.
///
/// The message consumer should guarantee that after a timeout is unset, it will
/// not get received. It should also try it best to guarantee that after a
/// timeout is reset, it should be delievered at the delayed deadline instead of
/// the previous one.
///
/// The message producer should guarantee that for each timeout value, i.e.
/// instance of type `T`, the message order must be one `Set(T)`, followed by
/// zero or more `Reset(T)`, followed by zero or one `Unset(T)`. There must not
/// be any other ordering other invalid message sequence e.g. multiple `Unset`.
///
/// The contract is relatively complicated (makes it hard to be encoded into
/// type checking) and error-prone, so consider use tick-based timeout when it
/// is capatible.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Timeout<T> {
    Set(T),
    Reset(T),
    Unset(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TimeoutLift;

impl<S, M> Lift<S, Timeout<M>> for TimeoutLift
where
    S: FunctionalState<M>,
{
    type Out<'o> = Timeout<S::Output<'o>> where Self: 'o, S: 'o;

    fn update<'a>(&'a mut self, state: &'a mut S, message: Timeout<M>) -> Self::Out<'a> {
        match message {
            Timeout::Set(message) => Timeout::Set(state.update(message)),
            Timeout::Reset(message) => Timeout::Reset(state.update(message)),
            Timeout::Unset(message) => Timeout::Unset(state.update(message)),
        }
    }
}

pub type Transport<T> = (SocketAddr, T);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct TransportLift;

impl<S, M> Lift<S, Transport<M>> for TransportLift
where
    S: FunctionalState<M>,
{
    type Out<'o> = Transport<S::Output<'o>> where Self: 'o, S: 'o;

    fn update<'a>(&'a mut self, state: &'a mut S, message: Transport<M>) -> Self::Out<'a> {
        let (addr, message) = message;
        (addr, state.update(message))
    }
}

pub use crate::route::Message as Route;

impl<K, M> Route<K, M> {
    pub fn to(k: K) -> impl FnOnce(M) -> Self {
        move |m| Self::To(k, m)
    }
}

// type inference works better with `K`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct RouteLift<K>(PhantomData<K>);

impl<S, K, M> Lift<S, Route<K, M>> for RouteLift<K>
where
    S: FunctionalState<M>,
{
    type Out<'o> = Route<K, S::Output<'o>> where Self: 'o, S: 'o;

    fn update<'a>(&'a mut self, state: &'a mut S, message: Route<K, M>) -> Self::Out<'a> {
        match message {
            Route::To(dest, message) => Route::To(dest, state.update(message)),
            Route::ToAll(message) => Route::ToAll(state.update(message)),
        }
    }
}
