macro_rules! hashset {
    (@unit $e:expr) => (());
    (@count $($e:expr),*) => ( [$( hashset!(@unit $e) ),*].len() );
    ( $( $e:expr ),* ) => ({
        let _cap = hashset!(@count $($e),*);
        let mut h = ::std::collections::HashSet::with_capacity(_cap);
        $( h.insert($e); )*
        h
    });
    ( $( $e:expr, )+ ) => ( hashset!( $($e),+ ) );
}

macro_rules! wm_debug {
    ( $($a:tt)* ) => (
        if cfg!(debug_assertions) {
            let (f, l) = (file!(), line!());
            print!("{}:{}: ", f, l);
            println!{$($a)*}; 
        })
}

#[cfg(feature = "core_intrinsics")]
pub fn print_type_of<T>(_: &T) {
    wm_debug!("{}", unsafe { std::intrinsics::type_name::<T>() });
}

#[cfg(not(feature = "core_intrinsics"))]
pub fn print_type_of<T>(_: &T) {
}

#[cfg(test)]
mod tests {
    use std::str;

    #[test]
    fn test_hashset1() {
        let h = hashset!( "a", "b",);
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn test_hashset2() {
        let h = hashset!( "a", "b");
        assert_eq!(h.len(), 2);
    }

    #[test]
    fn test_hashset3() {
        let s = "first".to_string();
        let h = hashset!( s.as_ref(), str::from_utf8(b"hello").unwrap(), "third");
        assert_eq!(h.len(), 3);
    }

    #[test]
    fn test_hashset4() {
        let l = hashset!(@count "", "", "", "");
        assert_eq!(l, 4);
    }
}
