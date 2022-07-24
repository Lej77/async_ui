use std::{
    cell::{Ref, RefMut},
    ops::{Deref, DerefMut},
};

use crate::{ObserbableInner, Observable};

pub struct ObservableBorrowRef<'a, T> {
    inner: Ref<'a, ObserbableInner<T>>,
}
pub struct ObservableBorrowRefMut<'a, T> {
    inner: RefMut<'a, ObserbableInner<T>>,
}
impl<T> Observable<T> {
    pub fn borrow(&self) -> ObservableBorrowRef<'_, T> {
        ObservableBorrowRef {
            inner: self.inner.borrow(),
        }
    }
    pub fn borrow_mut(&self) -> ObservableBorrowRefMut<'_, T> {
        ObservableBorrowRefMut {
            inner: self.inner.borrow_mut(),
        }
    }
}
impl<'a, T> Deref for ObservableBorrowRef<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner.data
    }
}
impl<'a, T> Deref for ObservableBorrowRefMut<'a, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        &self.inner.data
    }
}
impl<'a, T> DerefMut for ObservableBorrowRefMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner.data
    }
}
impl<'a, T> Drop for ObservableBorrowRefMut<'a, T> {
    fn drop(&mut self) {
        for (_key, waker) in self.inner.listeners.iter() {
            waker.wake_by_ref();
        }
    }
}
