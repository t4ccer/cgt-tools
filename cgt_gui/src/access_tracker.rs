#![allow(dead_code)]

use std::ops::{Deref, DerefMut};

#[derive(Debug, Clone)]
pub(crate) struct AccessTracker<T> {
    value: T,
    modified: bool,
}

impl<T> AccessTracker<T> {
    #[inline(always)]
    pub(crate) fn new(value: T) -> Self {
        Self {
            value,
            modified: true,
        }
    }

    #[inline(always)]
    pub(crate) fn clear_flag(&mut self) -> bool {
        let was_modified = self.modified;
        self.modified = false;
        was_modified
    }

    #[inline(always)]
    pub(crate) fn get_mut_untracked(&mut self) -> &mut T {
        &mut self.value
    }

    #[inline(always)]
    pub(crate) fn map<'a, F, R>(&'a mut self, map_fn: F) -> BorrowedAccessTracker<'a, R>
    where
        F: FnOnce(&'a mut T) -> &'a mut R,
        R: 'a,
    {
        BorrowedAccessTracker {
            value: map_fn(&mut self.value),
            modified: &mut self.modified,
        }
    }
}

impl<T> From<T> for AccessTracker<T> {
    #[inline(always)]
    fn from(value: T) -> Self {
        AccessTracker::new(value)
    }
}

impl<T> Deref for AccessTracker<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for AccessTracker<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.modified = true;
        &mut self.value
    }
}

#[derive(Debug)]
pub(crate) struct BorrowedAccessTracker<'a, R> {
    modified: &'a mut bool,
    value: &'a mut R,
}

impl<'a, R> BorrowedAccessTracker<'a, R> {
    #[inline(always)]
    pub(crate) fn map<F, U>(&'a mut self, map_fn: F) -> BorrowedAccessTracker<'a, U>
    where
        F: FnOnce(&'a mut R) -> &'a mut U,
        U: 'a,
    {
        BorrowedAccessTracker {
            value: map_fn(&mut self.value),
            modified: &mut self.modified,
        }
    }
}

impl<T> Deref for BorrowedAccessTracker<'_, T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for BorrowedAccessTracker<'_, T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut Self::Target {
        *self.modified = true;
        self.value
    }
}
