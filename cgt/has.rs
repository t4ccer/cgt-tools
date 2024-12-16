//! `Has` trait and utils

/// Types that implement `Has<T>` claim to contain `T` in their structure and can give access to it
///
/// It is useful when different parts need to operate of a concrete part of abstract datatype.
/// e.g. a graph vertex may have color and position, game only cares about color but layout
/// algorithm cares only about position
pub trait Has<T> {
    #[allow(missing_docs)]
    fn get_inner(&self) -> &T;

    #[allow(missing_docs)]
    fn get_inner_mut(&mut self) -> &mut T;
}

impl<T> Has<T> for T {
    #[inline(always)]
    fn get_inner(&self) -> &T {
        self
    }

    #[inline(always)]
    fn get_inner_mut(&mut self) -> &mut T {
        self
    }
}
