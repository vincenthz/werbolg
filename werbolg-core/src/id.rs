#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u32);

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
