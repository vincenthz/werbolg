#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(u32);

impl Id {
    pub fn as_index(self) -> usize {
        self.0 as usize
    }

    pub fn from_slice_len<T>(slice: &[T]) -> Self {
        Id(slice.len() as u32)
    }

    pub fn from_collection_len(len: usize) -> Self {
        Id(len as u32)
    }

    pub fn remap(left: Self, right: Self) -> Self {
        Self(left.0 + right.0)
    }

    pub fn add(left: Self, right: u32) -> Self {
        Self(left.0.checked_add(right).expect("ID valid add"))
    }

    pub fn diff(left: Self, right: Self) -> u32 {
        left.0.checked_sub(right.0).expect("ID valid diff")
    }
}

impl core::fmt::Debug for Id {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub trait IdRemapper: Copy {
    fn uncat(self) -> Id;
    fn cat(id: Id) -> Self;
}

macro_rules! define_id_remapper {
    ($constr:ident) => {
        impl IdRemapper for $constr {
            fn uncat(self) -> Id {
                self.0
            }

            fn cat(id: Id) -> Self {
                Self(id)
            }
        }
    };
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct FunId(Id);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LitId(Id);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct ConstrId(Id);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NifId(Id);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GlobalId(Id);

define_id_remapper!(FunId);
define_id_remapper!(LitId);
define_id_remapper!(ConstrId);
define_id_remapper!(NifId);
define_id_remapper!(GlobalId);

#[derive(Clone, Copy, Debug)]
pub enum ValueFun {
    Native(NifId),
    Fun(FunId),
}
