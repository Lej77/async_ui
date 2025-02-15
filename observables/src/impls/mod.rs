use crate::{Listenable, Observable, ObservableBorrow, Version};
mod stdlib;

pub struct NoChange<T>(pub T);
impl<T> Listenable for NoChange<T> {
    fn add_waker(&self, _waker: std::task::Waker) {
        // NO-OP
    }
    fn get_version(&self) -> crate::Version {
        Version::new()
    }
}
impl<T> Observable for NoChange<T> {
    type Data = T;
    fn borrow_observable<'b>(&'b self) -> ObservableBorrow<'b, T> {
        ObservableBorrow::Borrow(&self.0)
    }
}

// impl<'t, T: ?Sized> ObservableBase for &'t T
// where
//     T: ObservableBase,
// {
//     fn add_waker(&self, waker: std::task::Waker) {
//         <T as ObservableBase>::add_waker(self, waker)
//     }
//     fn get_version(&self) -> Version {
//         <T as ObservableBase>::get_version(self)
//     }
// }
// impl<'t, T: ?Sized> Observable for &'t T
// where
//     T: Observable,
// {
//     type Data = T::Data;

//     fn get_borrow<'b>(&'b self) -> ObservableBorrow<'b, Self::Data> {
//         <T as Observable>::get_borrow(self)
//     }
// }
