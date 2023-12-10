#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u32);

pub struct IdAllocator(u32);

impl IdAllocator {
    pub fn new() -> Self {
        Self(0)
    }

    pub fn allocate(&mut self) -> Id {
        let v = self.0;
        self.0 += 1;
        Id(v)
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

#[derive(Debug, Copy, Clone)]
pub struct FunId(Id);

#[derive(Debug, Copy, Clone)]
pub struct LitId(Id);

define_id_remapper!(FunId);
define_id_remapper!(LitId);
