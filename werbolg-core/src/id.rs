pub trait IdF:
    core::fmt::Debug + core::hash::Hash + PartialEq + Eq + PartialOrd + Ord + Copy
{
    fn as_index(self) -> usize;
    fn from_slice_len<T>(slice: &[T]) -> Self;
    fn from_collection_len(len: usize) -> Self;
    fn remap(left: Self, right: Self) -> Self;
    fn add(left: Self, right: u32) -> Self;
    fn diff(left: Self, right: Self) -> u32;
}

macro_rules! define_id_remapper {
    ($constr:ident, $bt:ident, $n:literal, $c:expr) => {
        #[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $constr($bt);

        impl IdF for $constr {
            fn as_index(self) -> usize {
                self.0 as usize
            }

            fn from_slice_len<T>(slice: &[T]) -> Self {
                Self(slice.len() as $bt)
            }

            fn from_collection_len(len: usize) -> Self {
                Self(len as $bt)
            }

            fn remap(left: Self, right: Self) -> Self {
                Self(left.0 + right.0)
            }

            fn add(left: Self, right: u32) -> Self {
                Self(left.0.checked_add(right).expect("ID valid add"))
            }

            fn diff(left: Self, right: Self) -> u32 {
                left.0.checked_sub(right.0).expect("ID valid diff")
            }
        }

        impl core::fmt::Debug for $constr {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{}{:?}", $c, self.0)
            }
        }
    };
}

define_id_remapper!(FunId, u32, 32, 'F');
define_id_remapper!(LitId, u32, 32, 'L');
define_id_remapper!(ConstrId, u32, 32, 'C');
define_id_remapper!(NifId, u32, 32, 'N');
define_id_remapper!(GlobalId, u32, 32, 'G');
define_id_remapper!(InstructionAddress, u32, 32, '%');

#[derive(Clone, Copy, Debug)]
pub enum ValueFun {
    Native(NifId),
    Fun(FunId),
}
