use hashbrown::{hash_map, HashMap};
use werbolg_core::Ident;

#[derive(Clone)]
pub struct Bindings<T>(HashMap<Ident, T>);

impl<T> Default for Bindings<T> {
    fn default() -> Self {
        Bindings::new()
    }
}

pub struct BindingsIterator<'a, T>(hash_map::Iter<'a, Ident, T>);

impl<'a, T> Iterator for BindingsIterator<'a, T> {
    type Item = (&'a Ident, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
}

pub struct BindingInsertError {
    pub name: Ident,
}

impl<T> Bindings<T> {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn add(&mut self, name: Ident, value: T) -> Result<(), BindingInsertError> {
        if self.0.get(&name).is_some() {
            Err(BindingInsertError { name })
        } else {
            self.0.insert(name, value);
            Ok(())
        }
    }

    pub fn add_replace(&mut self, name: Ident, value: T) {
        self.0.insert(name, value);
    }

    #[allow(unused)]
    pub fn remove(&mut self, name: Ident) {
        self.0.remove(&name);
    }

    pub fn get(&self, name: &Ident) -> Option<&T> {
        self.0.get(name)
    }

    pub fn dump<W: core::fmt::Write>(&self, writer: &mut W) -> Result<(), core::fmt::Error> {
        for (ident, _) in self.iter() {
            writeln!(writer, "{:?}", ident)?
        }
        Ok(())
    }

    pub fn iter<'a>(&'a self) -> BindingsIterator<'a, T> {
        BindingsIterator(self.0.iter())
    }
}
