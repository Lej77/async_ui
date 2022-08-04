use std::{
    cell::{Cell, RefCell},
    task::Waker,
};

pub struct Listeners {
    inner: RefCell<ListenersInner>,
    version: Cell<u64>,
}
struct ListenersInner {
    outside_wakers: Vec<Waker>,
    inside_wakers: Vec<Waker>,
}

impl Listeners {
    pub const fn new() -> Self {
        let inner = RefCell::new(ListenersInner {
            outside_wakers: Vec::new(),
            inside_wakers: Vec::new(),
        });
        Self {
            inner,
            version: Cell::new(0),
        }
    }
    fn invalidate_version(&self) {
        self.version.set(self.version.get() + 1);
    }
    pub(crate) fn invalidate_inside(&self) {
        self.invalidate_version();
        let mut bm = self.inner.borrow_mut();
        bm.inside_wakers.drain(..).for_each(Waker::wake);
    }
    pub(crate) fn invalidate_outside(&self) {
        self.invalidate_version();
        let mut bm = self.inner.borrow_mut();
        bm.outside_wakers.drain(..).for_each(Waker::wake);
    }
}
