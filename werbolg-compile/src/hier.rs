use alloc::boxed::Box;
use alloc::vec::Vec;
use hashbrown::{hash_map, HashMap};
use werbolg_core::{AbsPath, Ident, Namespace};

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

impl<T> Hier<T> {
    pub fn iterator<'a, I, J, F>(
        &'a self,
        namespace: Namespace,
        f: alloc::rc::Rc<F>,
    ) -> HierIterator<'a, I, J, T, F>
    where
        F: Fn(&'a T) -> J,
        J: Iterator<Item = (&'a Ident, &'a I)>,
    {
        HierIterator::<'a> {
            namespace,
            current_consumed: false,
            current: CurrentOrSub::Current(f(&self.current)),
            ns: self.ns.iter(),
            f: f,
        }
    }
}

/// Hierarchy iterator for Hier<T>
///
/// 'a is the lifetime of the initial Hier<T>
/// I is the item inside the second element of the Iterator of the T
/// J is the iterator object created from T which return a Tuple of elements, (Ident, I)
/// T is the object embedded inside Hier
/// F is the closure to create the iterator associated with T
pub struct HierIterator<'a, I: 'a, J, T, F: Fn(&'a T) -> J>
where
    J: Iterator<Item = (&'a Ident, &'a I)>,
{
    namespace: Namespace,
    current_consumed: bool,
    current: CurrentOrSub<J, Box<HierIterator<'a, I, J, T, F>>>,
    ns: hash_map::Iter<'a, Ident, Hier<T>>,
    f: alloc::rc::Rc<F>,
}

pub enum CurrentOrSub<A, B> {
    Current(A),
    Sub(B),
}

impl<'a, I, J, T, F: Fn(&'a T) -> J> Iterator for HierIterator<'a, I, J, T, F>
where
    J: Iterator<Item = (&'a Ident, &'a I)>,
{
    type Item = (AbsPath, &'a I);

    fn next(&mut self) -> Option<Self::Item> {
        let next = match &mut self.current {
            CurrentOrSub::Current(c_iter) => {
                if let Some(x) = c_iter.next() {
                    let path = AbsPath::new(&self.namespace, x.0);
                    Some((path, x.1))
                } else {
                    None
                }
            }
            CurrentOrSub::Sub(e) => e.next(),
        };
        if let Some(x) = next {
            Some(x)
        } else {
            if self.current_consumed {
                self.namespace = self.namespace.parent();
            } else {
                self.current_consumed = true;
            }
            match self.ns.next() {
                None => None,
                Some((ns, hier)) => {
                    self.namespace = self.namespace.clone().append(ns.clone());
                    let x = hier.iterator(self.namespace.clone(), self.f.clone());
                    self.current = CurrentOrSub::Sub(Box::new(x));
                    self.next()
                }
            }
        }
    }
}
