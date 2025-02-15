use std::{
    cell::{Cell, RefCell},
    marker::PhantomData,
    pin::Pin,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use futures_signals::signal::Signal;
use waker_fn::waker_fn;

use crate::{Listenable, Observable, ObservableAs, Version};

pub struct ToSignal<W, I, O, M>
where
    W: ObservableAs<I>,
    M: Fn(&I) -> O,
    Self: Unpin,
{
    wrapped: W,
    mapper: M,
    last_version: Version,
    _phantom: PhantomData<I>,
}

impl<W, I, O, M> ToSignal<W, I, O, M>
where
    W: ObservableAs<I>,
    M: Fn(&I) -> O,
    Self: Unpin,
{
    pub fn new(wrapped: W, mapper: M) -> Self {
        Self {
            wrapped,
            mapper,
            last_version: Version::new_null(),
            _phantom: PhantomData,
        }
    }
}

impl<W, I, O, M> Signal for ToSignal<W, I, O, M>
where
    W: ObservableAs<I>,
    M: Fn(&I) -> O,
    Self: Unpin,
{
    type Item = O;

    fn poll_change(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        let current_version = this.wrapped.get_version();
        if current_version > this.last_version {
            this.last_version = current_version;
            let val = this.wrapped.borrow_observable_as();
            let out = (this.mapper)(&*val);
            this.wrapped.add_waker(cx.waker().to_owned());
            Poll::Ready(Some(out))
        } else {
            Poll::Pending
        }
    }
}

pub struct FromSignal<S>
where
    S: Signal + Unpin,
    S::Item: Default,
{
    signal: RefCell<S>,
    value: RefCell<S::Item>,
    wakers: Arc<Mutex<Vec<Waker>>>,
    combined_waker: Waker,
    version: Cell<Version>,
}

impl<S> FromSignal<S>
where
    S: Signal + Unpin,
    S::Item: Default,
{
    pub fn new(signal: S) -> Self {
        let wakers: Arc<Mutex<Vec<Waker>>> = Default::default();
        let wakers_cloned = wakers.clone();
        Self {
            signal: RefCell::new(signal),
            value: RefCell::new(Default::default()),
            wakers,
            combined_waker: waker_fn(move || {
                wakers_cloned
                    .lock()
                    .unwrap()
                    .drain(..)
                    .for_each(Waker::wake);
            }),
            version: Cell::new(Version::new()),
        }
    }
}

pub trait ObservableFromSignal
where
    Self: Signal + Unpin + Sized,
    Self::Item: Default,
{
    fn as_observable(self) -> FromSignal<Self> {
        FromSignal::new(self)
    }
}

impl<S> ObservableFromSignal for S
where
    S: Signal + Unpin,
    S::Item: Default,
{
}

impl<S> Observable for FromSignal<S>
where
    S: Signal + Unpin,
    S::Item: Default,
{
    type Data = S::Item;
    fn borrow_observable<'b>(&'b self) -> crate::ObservableBorrow<'b, S::Item> {
        crate::ObservableBorrow::RefCell(self.value.borrow())
    }
}

impl<S> Listenable for FromSignal<S>
where
    S: Signal + Unpin,
    S::Item: Default,
{
    fn add_waker(&self, waker: Waker) {
        self.wakers.lock().unwrap().push(waker)
    }
    fn get_version(&self) -> Version {
        let this = self;
        let mut cx = Context::from_waker(&this.combined_waker);
        let mut sig = this.signal.borrow_mut();
        let sig = Pin::new(&mut *sig);
        match sig.poll_change(&mut cx) {
            Poll::Ready(Some(item)) => {
                *self.value.borrow_mut() = item;
                let new_ver = self.version.get().incremented();
                self.version.set(new_ver);
                new_ver
            }
            _ => self.version.get(),
        }
    }
}
