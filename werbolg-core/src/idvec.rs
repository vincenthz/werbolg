//! A Vector indexed by a specific ID

use super::id::IdF;
use alloc::vec::Vec;
use core::marker::PhantomData;

/// A Vector Indexed by a specific ID
///
/// Note that it can be dereferenced using the array syntax `idec[id]`
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
    /// Create a new empty IdVec
    pub fn new() -> Self {
        Self {
            vec: Vec::new(),
            phantom: PhantomData,
        }
    }

    /// Get the index
    pub fn get(&self, id: ID) -> Option<&T> {
        let idx = id.as_index();
        if self.vec.len() > idx {
            Some(&self.vec[idx])
        } else {
            None
        }
    }

    /// Return the next Id that will be created on push
    pub fn next_id(&self) -> ID {
        ID::from_slice_len(&self.vec)
    }

    /// Append a new element to this IdVec, and returns the Id associated
    pub fn push(&mut self, v: T) -> ID {
        let id = ID::from_slice_len(&self.vec);
        self.vec.push(v);
        id
    }

    /// Create a mutable iterator for this IdVec
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.vec.iter_mut()
    }

    /// Create an reference Iterator for this IdVec, note that the Id is also given
    pub fn iter(&self) -> impl Iterator<Item = (ID, &T)> {
        self.vec
            .iter()
            .enumerate()
            .map(|(i, t)| (ID::from_collection_len(i), t))
    }

    /// Create a value Iterator for this IdVec
    pub fn into_iter(self) -> impl Iterator<Item = (ID, T)> {
        self.vec
            .into_iter()
            .enumerate()
            .map(|(i, t)| (ID::from_collection_len(i), t))
    }

    /// Append a IdVecAfter to this IdVec
    pub fn concat(&mut self, after: &mut IdVecAfter<ID, T>) {
        assert!(self.vec.len() == after.ofs.as_index());
        self.vec.append(&mut after.id_vec.vec)
    }

    /// Consume the IdVec and create a new IdVec using the function 'f' to map
    /// all the elements T of this IdVec.
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

/// An IdVec that doesn't starts at Id=0 with the explicit goal to
/// append the underlying IdVec to another IdVec
pub struct IdVecAfter<ID, T> {
    id_vec: IdVec<ID, T>,
    ofs: ID,
}

impl<ID: IdF, T> IdVecAfter<ID, T> {
    /// Create a new IdVec starting at ID=first_id
    pub fn new(first_id: ID) -> Self {
        Self {
            id_vec: IdVec::new(),
            ofs: first_id,
        }
    }

    /// Create an offset IdVec from another IdVec
    pub fn from_idvec(id_vec: IdVec<ID, T>, first_id: ID) -> Self {
        Self {
            id_vec,
            ofs: first_id,
        }
    }

    /// Append to this IdVecAfter a value T and returns its Id
    pub fn push(&mut self, v: T) -> ID {
        let id = self.id_vec.push(v);
        let new_id = ID::remap(id, self.ofs);
        new_id
    }

    /// Remap all element of this IdVec in place
    pub fn remap<F>(&mut self, f: F)
    where
        F: Fn(&mut T) -> (),
    {
        for elem in self.id_vec.iter_mut() {
            f(elem)
        }
    }
}
