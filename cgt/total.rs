//! Total ordering wrappers
//!
//! Useful when there exists a total ordering but it does not obey domain-specific rules but you
//! can use the wrapper type if you need to store that type in some collection.

use std::{cmp::Ordering, hash::Hash, ops::Deref};

/// Types that have non-canonical total ordering
pub trait TotalWrappable {
    /// Should obey the same laws as [`std::cmp::Ord::cmp`]
    fn total_cmp(&self, other: &Self) -> Ordering;

    /// Should obey the same laws as [`std::hash::Hash::hash`]
    fn total_hash<H: std::hash::Hasher>(&self, state: &mut H);

    /// Should obey the same laws as [`std::cmp::PartialEq::eq`]
    #[inline(always)]
    fn total_eq(&self, other: &Self) -> bool {
        matches!(self.total_cmp(other), Ordering::Equal)
    }
}

impl<T> TotalWrappable for &T
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn total_cmp(&self, other: &Self) -> Ordering {
        (**self).total_cmp(&**other)
    }

    #[inline(always)]
    fn total_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).total_hash(state);
    }
}

impl<T> TotalWrappable for &mut T
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn total_cmp(&self, other: &Self) -> Ordering {
        (**self).total_cmp(&**other)
    }

    #[inline(always)]
    fn total_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).total_hash(state);
    }
}

impl<T> TotalWrappable for Vec<T>
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn total_cmp(&self, other: &Self) -> Ordering {
        TotalWrapper::from_inner_slice(self).cmp(TotalWrapper::from_inner_slice(other))
    }

    #[inline(always)]
    fn total_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        TotalWrapper::from_inner_slice(self).hash(state);
    }
}

impl<T> TotalWrappable for [T]
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn total_cmp(&self, other: &Self) -> Ordering {
        TotalWrapper::from_inner_slice(self).cmp(TotalWrapper::from_inner_slice(other))
    }

    #[inline(always)]
    fn total_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        TotalWrapper::from_inner_slice(self).hash(state);
    }
}

/// `TotalWrapper` can be used when inner type implements non-canonical total ordering
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct TotalWrapper<T> {
    inner: T,
}

impl<T> TotalWrapper<T> {
    /// Create new wrapper
    #[inline(always)]
    pub const fn new(inner: T) -> TotalWrapper<T> {
        TotalWrapper { inner }
    }

    /// Extract the inner value
    #[inline(always)]
    pub fn get(self) -> T {
        self.inner
    }

    // SAFETY: TotalWrapper is repr(transparent)
    unsafe_impl_inner_collections!(TotalWrapper<T>, T, pub);
}

impl<T> PartialEq for TotalWrapper<T>
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        matches!(self.inner.total_cmp(&other.inner), Ordering::Equal)
    }
}

impl<T> Eq for TotalWrapper<T> where T: TotalWrappable {}

#[allow(clippy::non_canonical_partial_ord_impl)]
impl<T> PartialOrd for TotalWrapper<T>
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.inner.total_cmp(&other.inner))
    }
}

impl<T> Ord for TotalWrapper<T>
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn cmp(&self, other: &Self) -> Ordering {
        self.inner.total_cmp(&other.inner)
    }
}

impl<T> std::hash::Hash for TotalWrapper<T>
where
    T: TotalWrappable,
{
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.inner.total_hash(state);
    }
}

impl<T> Deref for TotalWrapper<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T> AsRef<T> for TotalWrapper<T> {
    fn as_ref(&self) -> &T {
        &self.inner
    }
}

macro_rules! unsafe_impl_inner_collections {
    ($wrapper:ty, $inner:ty, $vis:vis) => {
        #[allow(dead_code, missing_docs)]
        #[inline(always)]
        $vis fn into_inner_vec(moves: Vec<$wrapper>) -> Vec<$inner> {
            let mut md = ::core::mem::ManuallyDrop::new(moves);
            let ptr: *mut $wrapper = md.as_mut_ptr();
            let len = md.len();
            let capacity = md.capacity();
            unsafe { Vec::from_raw_parts(ptr.cast::<$inner>(), len, capacity) }
        }

        #[allow(dead_code, missing_docs)]
        #[inline(always)]
        $vis fn from_inner_vec(moves: Vec<$inner>) -> Vec<$wrapper> {
            let mut md = ::core::mem::ManuallyDrop::new(moves);
            let ptr: *mut $inner = md.as_mut_ptr();
            let len = md.len();
            let capacity = md.capacity();
            unsafe { Vec::from_raw_parts(ptr.cast::<$wrapper>(), len, capacity) }
        }

        #[allow(dead_code, missing_docs)]
        #[inline(always)]
        $vis fn into_inner_slice(moves: &[$wrapper]) -> &[$inner] {
            let md = ::core::mem::ManuallyDrop::new(moves);
            let ptr: *const $wrapper = md.as_ptr();
            let len = md.len();
            unsafe { core::slice::from_raw_parts(ptr.cast::<$inner>(), len) }
        }

        #[allow(dead_code, missing_docs)]
        #[inline(always)]
        $vis fn from_inner_slice(moves: &[$inner]) -> &[$wrapper] {
            let md = ::core::mem::ManuallyDrop::new(moves);
            let ptr: *const $inner = md.as_ptr();
            let len = md.len();
            unsafe { core::slice::from_raw_parts(ptr.cast::<$wrapper>(), len) }
        }

        /// Create new reference to wrapper
        #[allow(dead_code, missing_docs)]
        #[inline(always)]
        $vis fn from_ref(inner: &$inner) -> &$wrapper {
            unsafe { &*core::ptr::from_ref::<$inner>(inner).cast::<$wrapper>() }
        }
    };
}
pub(crate) use unsafe_impl_inner_collections;

macro_rules! impl_total_wrapper {
    ( $(#[$attr:meta])*
      $struct_vis:vis struct $wrapper:ident {
          $field_vis:vis $field:ident: $inner:ty $(,)?
      }
    ) => {
        $(#[$attr])*
        #[repr(transparent)]
        $struct_vis struct $wrapper {
            $field: $inner,
        }

        impl $wrapper {
            // SAFETY: $wrapper is #[repr(transparent)]
            $crate::total::unsafe_impl_inner_collections!($wrapper, $inner, $field_vis);
        }

        impl $crate::total::TotalWrappable for $wrapper {
            #[inline(always)]
            fn total_cmp(&self, other: &Self) -> ::core::cmp::Ordering {
                self.$field.cmp(&other.$field)
            }

            #[inline(always)]
            fn total_eq(&self, other: &Self) -> ::core::primitive::bool {
                self.$field.eq(&other.$field)
            }

            #[inline(always)]
            fn total_hash<H: ::std::hash::Hasher>(&self, state: &mut H) {
                use ::std::hash::Hash;

                self.$field.hash(state);
            }
        }
    };
}
pub(crate) use impl_total_wrapper;
