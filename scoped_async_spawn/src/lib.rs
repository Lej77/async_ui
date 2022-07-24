#![deny(unsafe_op_in_unsafe_fn)]
mod pointer;
pub mod boxed;

use std::{
    future::Future,
    marker::PhantomPinned,
    pin::Pin,
    rc::Rc,
    task::{Poll, Waker},
};

use pin_cell::{PinCell, PinMut};
use pin_project_lite::pin_project;
use pointer::Pointer;
use scoped_tls::scoped_thread_local;

scoped_thread_local!(static UNFORGETTABLE_SCOPE: Pointer);
pin_project! {
    pub struct GiveUnforgettableScope<F: Future> {
        #[pin] fut: F,
    }
}
impl<F: Future> Future for GiveUnforgettableScope<F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let fut_ref: &F = &self.as_ref().fut;
        let fut_ptr = Pointer::new(fut_ref);
        UNFORGETTABLE_SCOPE.set(&fut_ptr, || self.project().fut.poll(cx))
    }
}
impl<F: Future + 'static> GiveUnforgettableScope<F> {
    pub fn new_static(fut: F) -> Self {
        unsafe { Self::new(fut) }
    }
}
impl<F: Future> GiveUnforgettableScope<F> {
    unsafe fn new(fut: F) -> Self {
        Self { fut }
    }
}

impl<F: Future> Future for Inner<F> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let this = self.as_mut().project();
        match this {
            InnerProject::Created { fut, state } => match state {
                CreatedState::NotYetPinned => {
                    unreachable!("Cannot be spawned before pinned.");
                }
                CreatedState::Spawned { local_waker } => match fut.poll(cx) {
                    Poll::Ready(res) => {
                        let waker = local_waker.to_owned();
                        self.set(Inner::Finished { out: Some(res) });
                        waker.wake();
                        Poll::Ready(())
                    }
                    Poll::Pending => Poll::Pending,
                },
            },
            InnerProject::Finished { .. } => {
                panic!("Polled after Ready returned.");
            }
            InnerProject::Aborted => Poll::Ready(()),
        }
    }
}
pub struct RemoteStaticFuture {
    remote: Pin<Rc<PinCell<dyn Future<Output = ()> + 'static>>>,
}
impl RemoteStaticFuture {
    unsafe fn new<'x, F: Future + 'x>(remote: Pin<Rc<PinCell<Inner<F>>>>) -> Self {
        let remote = remote as Pin<Rc<PinCell<dyn Future<Output = ()> + 'x>>>;
        let remote = unsafe {
            std::mem::transmute::<
                Pin<Rc<PinCell<dyn Future<Output = ()> + 'x>>>,
                Pin<Rc<PinCell<dyn Future<Output = ()> + 'static>>>,
            >(remote)
        };
        Self { remote }
    }
}
impl Future for RemoteStaticFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let cell = self.remote.as_ref();
        let mut bm = cell.borrow_mut();
        let bm = PinMut::as_mut(&mut bm);
        bm.poll(cx)
    }
}
pin_project! {
    pub struct SpawnedFuture<F: Future, S: Fn(RemoteStaticFuture) -> O, O> {
        remote: DropWrappedRemote<F>,
        spawn_fn: S,
        task: Option<O>,
        _phantom: PhantomPinned
    }
}
struct DropWrappedRemote<F: Future> {
    remote: Pin<Rc<PinCell<Inner<F>>>>,
}
impl<F: Future> Drop for DropWrappedRemote<F> {
    fn drop(&mut self) {
        let mut bm = self.remote.as_ref().borrow_mut();
        let mut bm = PinMut::as_mut(&mut bm);
        bm.set(Inner::Aborted);
    }
}

impl<F: Future, S: Fn(RemoteStaticFuture) -> O, O> SpawnedFuture<F, S, O> {
    pub fn new(fut: F, spawn_fn: S) -> Self {
        let remote = Rc::pin(PinCell::new(Inner::Created {
            fut: unsafe { GiveUnforgettableScope::new(fut) },
            state: CreatedState::NotYetPinned,
        }));
        let remote = DropWrappedRemote { remote };
        let task = None;
        Self {
            remote,
            spawn_fn,
            task,
            _phantom: PhantomPinned,
        }
    }
}
impl<F: Future, S: Fn(RemoteStaticFuture) -> O, O> Future for SpawnedFuture<F, S, O> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let here = Pointer::new(&*self);
        let this = self.project();
        let remote_to_spawn = {
            let cell = this.remote.remote.as_ref();
            let mut bm = cell.borrow_mut();
            let bm = PinMut::as_mut(&mut bm);
            match bm.project() {
                InnerProject::Created { state, .. } => match state {
                    CreatedState::NotYetPinned => {
                        let in_scope = UNFORGETTABLE_SCOPE.with(|sc| sc.contains(here));
                        if !in_scope {
                            panic!("Not in scope.");
                        }
                        *state = CreatedState::Spawned {
                            local_waker: cx.waker().to_owned(),
                        };
                        let remote = unsafe { RemoteStaticFuture::new(this.remote.remote.clone()) };
                        remote
                    }
                    CreatedState::Spawned { .. } => return Poll::Pending,
                },
                InnerProject::Finished { out } => {
                    if let Some(res) = out.take() {
                        return Poll::Ready(res);
                    } else {
                        panic!("Polled after Ready returned.");
                    }
                }
                InnerProject::Aborted => {
                    unreachable!("Cannot be polled after aborted.");
                }
            }
        };
        let task = (this.spawn_fn)(remote_to_spawn);
        *this.task = Some(task);
        Poll::Pending
    }
}

pin_project! {
    #[project = InnerProject]
    enum Inner<F: Future> {
        Created {
            #[pin] fut: GiveUnforgettableScope<F>,
            state: CreatedState
        },
        Finished {
            out: Option<F::Output>
        },
        Aborted
    }
}
enum CreatedState {
    NotYetPinned,
    Spawned { local_waker: Waker },
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
