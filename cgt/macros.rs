//! Macros for internal use

macro_rules! after_then_block {
    (then $b:block; $($rest:tt)*) => {
        if_chain!($($rest)*)
    };

    (if let $p:pat = $e:expr; $($rest:tt)*) => {
        $crate::macros::after_then_block!($($rest)*)
    };

    (if $e:expr; $($rest:tt)*) => {
        $crate::macros::after_then_block!($($rest)*)
    };
}
pub(crate) use after_then_block;

macro_rules! if_chain {
    (if let $p:pat = $e:expr; $($rest:tt)*) => {{
        let mut flag = false;
        if_chain!(flag, if let $p = $e; $($rest)*);
        if !flag {
            $crate::macros::after_then_block!($($rest)*);
        }
    }};

    (if $e:expr; $($rest:tt)*) => {{
        let mut flag = false;
        if_chain!(flag, if $e; $($rest)*);
        if !flag {
            $crate::macros::after_then_block!($($rest)*);
        }
    }};

    ($flag:ident, if let $p:pat = $e:expr; $($rest:tt)*) => {
        if let $p = $e {
            $crate::macros::if_chain!($flag, $($rest)*)
        }
    };

    ($flag:ident, if $e:expr; $($rest:tt)*) => {
        if $e {
            $crate::macros::if_chain!($flag, $($rest)*)
        }
    };

    ($flag:ident, then $b:block; $($rest:tt)*) => {{
        $flag = true;
        $b
    }};

    (else $b:block;) => {
        $b
    };
}

pub(crate) use if_chain;

#[test]
#[allow(clippy::missing_const_for_fn)]
fn test_if_chain() {
    let mut _bar;

    if_chain! {
        if let Some(x) = None::<i32>;
        if let Some(y) = None::<i32>;
        if x == y;
        if let Some(z) = None::<i32>;
        if x == z + 1;
        then {
            let _ = x + y;
            _bar = "foo";
        };

        if let Some(_) = None::<i32>;
        then {
            _bar = "bar";
        };

        else {
            _bar = "baz";
        };
    };
}

macro_rules! _impl_bin_op {
    ($type:ident, $body:expr, $trait:ident, $fn:ident) => {
        impl ::std::ops::$trait<$type> for $type {
            type Output = $type;
            fn $fn(self, rhs: $type) -> $type {
                (|lhs, rhs| $body(lhs, rhs))(&self, &rhs)
            }
        }

        impl ::std::ops::$trait<&$type> for $type {
            type Output = $type;
            fn $fn(self, rhs: &$type) -> $type {
                (|lhs, rhs| $body(lhs, rhs))(&self, rhs)
            }
        }

        impl ::std::ops::$trait<$type> for &$type {
            type Output = $type;
            fn $fn(self, rhs: $type) -> $type {
                (|lhs, rhs| $body(lhs, rhs))(self, &rhs)
            }
        }

        impl ::std::ops::$trait<&$type> for &$type {
            type Output = $type;
            fn $fn(self, rhs: &$type) -> $type {
                (|lhs, rhs| $body(lhs, rhs))(self, &rhs)
            }
        }
    };
}
pub(crate) use _impl_bin_op;

macro_rules! _impl_un_op {
    ($type:ident, $body:expr, $trait:ident, $fn:ident) => {
        impl ::std::ops::$trait for $type {
            type Output = $type;
            fn $fn(self) -> $type {
                (|lhs| $body(lhs))(&self)
            }
        }

        impl ::std::ops::$trait for &$type {
            type Output = $type;
            fn $fn(self) -> $type {
                (|lhs| $body(lhs))(self)
            }
        }
    };
}
pub(crate) use _impl_un_op;

macro_rules! _impl_assign_op {
    ($type:ident, $body:expr, $trait:ident, $fn:ident) => {
        impl ::std::ops::$trait for $type {
            fn $fn(&mut self, rhs: $type) {
                (|lhs, rhs| $body(lhs, rhs))(self, &rhs)
            }
        }
    };
}
pub(crate) use _impl_assign_op;

macro_rules! impl_boilerplate_trait {
    (Add, $type:ident, $body:expr) => {
        $crate::macros::_impl_bin_op!($type, $body, Add, add);
    };
    (AddAssign, $type:ident, $body:expr) => {
        $crate::macros::_impl_assign_op!($type, $body, AddAssign, add_assign);
    };
    (Sub, $type:ident, $body:expr) => {
        $crate::macros::_impl_bin_op!($type, $body, Sub, sub);
    };
    (SubAssign, $type:ident, $body:expr) => {
        $crate::macros::_impl_assign_op!($type, $body, SubAssign, sub_assign);
    };
    (Neg, $type:ident, $body:expr) => {
        $crate::macros::_impl_un_op!($type, $body, Neg, neg);
    };
}

pub(crate) use impl_boilerplate_trait;
