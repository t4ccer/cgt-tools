macro_rules! atomic_enum {
    ($(#[$enum_attr:meta])*
     $enum_vis:vis enum $enum_name:ident {$(
         $(#[$variant_attr:meta])*
         $enum_visariant:ident,
     )*}

     $(#[$atomic_attr:meta])* $atomic_vis:vis struct $atomic_name:ident;) => {
        $(#[$enum_attr])*
        $enum_vis enum $enum_name {
            $($(#[$variant_attr])* $enum_visariant,)*
        }

        $(#[$atomic_attr])*
        $atomic_vis struct $atomic_name(::std::sync::atomic::AtomicU32);

        #[allow(dead_code)]
        impl $atomic_name {
            fn new(val: $enum_name) -> $atomic_name {
                $atomic_name(::std::sync::atomic::AtomicU32::new(val as u32))
            }

            fn load(&self, order: ::std::sync::atomic::Ordering) -> $enum_name {
                let raw = self.0.load(order);
                match raw {
                    $(__n if __n == $enum_name::$enum_visariant as u32 => $enum_name::$enum_visariant,)*
                    _ => unreachable!()
                }
            }

            fn store(&self, val: $enum_name, order: ::std::sync::atomic::Ordering) {
                self.0.store(val as u32, order);
            }
        }
    }
}

pub(crate) use atomic_enum;
