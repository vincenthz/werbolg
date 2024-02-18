use alloc::vec::Vec;
use hashbrown::HashMap;
use werbolg_core::{Ident, Namespace};

/// A hierarchical T with recursives namespaces as Ident
pub struct Hier<T> {
    current: T,
    ns: HashMap<Ident, Hier<T>>,
}

impl<T: Default> Default for Hier<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

pub struct HierError<E> {
    pub namespace: Namespace,
    pub err: Option<E>,
}

impl<T: Default> Hier<T> {
    pub fn new(current: T) -> Self {
        Self {
            current,
            ns: HashMap::new(),
        }
    }

    pub fn namespace_exist(&self, namespace: Namespace) -> bool {
        if namespace.is_root() {
            true
        } else {
            let (id, next) = namespace.clone().drop_first();
            if let Some(ns) = self.ns.get(&id) {
                ns.namespace_exist(next)
            } else {
                false
            }
        }
    }

    #[allow(unused)]
    pub fn add_ns(&mut self, ident: Ident, t: T) -> Result<(), ()> {
        let already_exist = self.ns.insert(ident, Hier::new(t));
        if already_exist.is_some() {
            Err(())
        } else {
            Ok(())
        }
    }

    pub fn add_ns_hier(&mut self, namespace: Namespace) -> Result<(), ()> {
        if namespace.is_root() {
            Ok(())
        } else {
            let (id, next) = namespace.clone().drop_first();
            if let Some(r) = self.ns.get_mut(&id) {
                r.add_ns_hier(next)
            } else {
                let mut h = Hier::new(T::default());
                h.add_ns_hier(next)?;
                self.ns.insert(id, h);
                Ok(())
            }
        }
    }

    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn on_mut<E, F>(&mut self, namespace: &Namespace, mut f: F) -> Result<(), HierError<E>>
    where
        F: FnMut(&mut T) -> Result<(), E>,
    {
        if namespace.is_root() {
            f(&mut self.current).map_err(|e| HierError {
                namespace: namespace.clone(),
                err: Some(e),
            })
        } else {
            let (id, next) = namespace.clone().drop_first();
            if let Some(r) = self.ns.get_mut(&id) {
                r.on_mut(&next, f)
            } else {
                Err(HierError {
                    namespace: namespace.clone(),
                    err: None,
                })
            }
        }
    }

    pub fn get(&self, namespace: &Namespace) -> Result<&T, ()> {
        if namespace.is_root() {
            Ok(&self.current)
        } else {
            let mut namespace = namespace.clone();
            let mut hier_pointer = self;
            loop {
                let (id, next) = namespace.drop_first();
                if let Some(r) = hier_pointer.ns.get(&id) {
                    hier_pointer = &r;
                    if next.is_root() {
                        return Ok(&r.current);
                    } else {
                        namespace = next;
                    }
                } else {
                    return Err(());
                }
            }
        }
    }

    pub fn get_sub(&self, id: &Ident) -> Result<&Hier<T>, ()> {
        if let Some(hier) = self.ns.get(id) {
            Ok(hier)
        } else {
            Err(())
        }
    }

    pub fn dump<'a>(&'a self, current: Namespace, out: &mut Vec<(Namespace, &'a T)>) {
        out.push((current.clone(), &self.current));
        for (ns_name, st) in self.ns.iter() {
            let child_namespace = current.clone().append(ns_name.clone());
            st.dump(child_namespace, out)
        }
    }
}
