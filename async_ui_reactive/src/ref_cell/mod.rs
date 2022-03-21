pub mod channel;
pub mod reactive;
use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

#[derive(Debug)]
struct IMCell<T>(RefCell<T>);
type Shared<T> = Rc<T>;
type LockReadGuard<'l, T> = Ref<'l, T>;
type LockWriteGuard<'l, T> = RefMut<'l, T>;
impl<T> IMCell<T> {
    fn lock_read(&self) -> LockReadGuard<'_, T> {
        self.0.borrow()
    }
    fn lock_write(&self) -> LockWriteGuard<'_, T> {
        self.0.borrow_mut()
    }
    fn new(value: T) -> Self {
        Self(RefCell::new(value))
    }
}
