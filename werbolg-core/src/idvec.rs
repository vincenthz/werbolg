use super::id::IdF;
use alloc::vec::Vec;
use core::marker::PhantomData;

pub struct IdVec<ID, T> {
    vec: Vec<T>,
    phantom: PhantomData<ID>,
}

impl<ID: IdF, T> core::ops::Index<ID> for IdVec<ID, T> {
    type Output = T;

    fn index(&self, index: ID) -> &Self::Output {
        &self.vec[index.as_index()]
    }
}

impl<ID: IdF, T> core::ops::IndexMut<ID> for IdVec<ID, T> {
    fn index_mut(&mut self, index: ID) -> &mut T {
        &mut self.vec[index.as_index()]
    }
}

impl<ID: IdF, T> IdVec<ID, T> {
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            phantom: PhantomData,
        }
    }

    pub fn get(&self, id: ID) -> Option<&T> {
        let idx = id.as_index();
        if self.vec.len() > idx {
            Some(&self.vec[idx])
        } else {
            None
        }
    }

    pub fn next_id(&self) -> ID {
        ID::from_slice_len(&self.vec)
    }

    pub fn push(&mut self, v: T) -> ID {
        let id = ID::from_slice_len(&self.vec);
        self.vec.push(v);
        id
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.vec.iter_mut()
    }

    pub fn iter(&self) -> impl Iterator<Item = (ID, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(i, t)| (ID::from_collection_len(i), t))
    }

    pub fn into_iter(self) -> impl Iterator<Item = (ID, T)> {
        self.vec
            .into_iter()
            .enumerate()
            .map(|(i, t)| (ID::from_collection_len(i), t))
    }

    pub fn concat(&mut self, after: &mut IdVecAfter<ID, T>) {
        assert!(self.vec.len() == after.ofs.as_index());
        self.vec.append(&mut after.id_vec.vec)
    }

    pub fn remap<F, U>(self, f: F) -> IdVec<ID, U>
    where
        F: Fn(T) -> U,
    {
        let mut new = IdVec::<ID, U>::new();
        for (id, t) in self.into_iter() {
            let new_id = new.push(f(t));
            assert_eq!(new_id, id);
        }
        new
    }
}

pub struct IdVecAfter<ID, T> {
    id_vec: IdVec<ID, T>,
    ofs: ID,
}

impl<ID: IdF, T> IdVecAfter<ID, T> {
    pub fn new(first_id: ID) -> Self {
        Self {
            id_vec: IdVec::new(),
            ofs: first_id,
        }
    }

    pub fn from_idvec(id_vec: IdVec<ID, T>, first_id: ID) -> Self {
        Self {
            id_vec,
            ofs: first_id,
        }
    }

    pub fn push(&mut self, v: T) -> ID {
        let id = self.id_vec.push(v);
        let new_id = ID::remap(id, self.ofs);
        new_id
    }

    pub fn remap<F>(&mut self, f: F)
    where
        F: Fn(&mut T) -> (),
    {
        for elem in self.id_vec.iter_mut() {
            f(elem)
        }
    }
}