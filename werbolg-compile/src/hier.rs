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

#[derive(Debug)]
pub struct HierError<E> {
    pub namespace: Namespace,
    pub err: Option<E>,
}

pub enum HierSearchError {
    ParentNotExisting(Namespace),
    HierAlreadyExist { parent: Namespace, ident: Ident },
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

    pub fn add_ns_hier(&mut self, namespace: Namespace, sub_hier: Hier<T>) -> Result<(), ()> {
        if namespace.is_root() {
            return Err(());
        }
        let mut nss = &mut self.ns;
        let mut namespace = namespace;
        loop {
            let (ns, sub_ns) = namespace.drop_first();
            if sub_ns.is_root() {
                let dup = nss.insert(ns, sub_hier).is_some();
                break if dup { Err(()) } else { Ok(()) };
            } else {
                let sub = nss.get_mut(&ns);
                if let Some(sub) = sub {
                    nss = &mut sub.ns;
                    namespace = sub_ns;
                } else {
                    break Err(());
                }
            }
        }
    }

    pub fn add_ns_empty_hier(&mut self, namespace: Namespace) -> Result<(), ()> {
        if namespace.is_root() {
            Ok(())
        } else {
            let (id, next) = namespace.clone().drop_first();
            if let Some(r) = self.ns.get_mut(&id) {
                r.add_ns_empty_hier(next)
            } else {
                let mut h = Hier::new(T::default());
                h.add_ns_empty_hier(next)?;
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
    pub fn flat_iterator<'a, I, J, F>(
        &'a self,
        namespace: Namespace,
        f: alloc::rc::Rc<F>,
    ) -> HierFlatIterator<'a, I, J, T, F>
    where
        F: Fn(&'a T) -> J,
        J: Iterator<Item = (&'a Ident, &'a I)>,
    {
        HierFlatIterator::<'a> {
            namespace,
            current: CurrentOrSub::Current(f(&self.current)),
            ns: self.ns.iter(),
            f: f,
        }
    }

    pub fn hier_iterator<'a, I, F>(
        &'a self,
        namespace: Namespace,
        f: alloc::rc::Rc<F>,
    ) -> HierIterator<'a, I, T, F>
    where
        F: Fn(&'a T) -> I,
    {
        HierIterator {
            namespace,
            current: Some(f(&self.current)),
            hier_iter: None,
            ns: self.ns.iter(),
            f: f,
        }
    }
}

pub struct HierIterator<'a, I: 'a, T, F> {
    namespace: Namespace,
    current: Option<I>,
    hier_iter: Option<Box<HierIterator<'a, I, T, F>>>,
    ns: hash_map::Iter<'a, Ident, Hier<T>>,
    f: alloc::rc::Rc<F>,
}

impl<'a, I: 'a, T, F> Iterator for HierIterator<'a, I, T, F>
where
    F: Fn(&'a T) -> I,
{
    type Item = (Namespace, I);

    fn next(&mut self) -> Option<Self::Item> {
        let mut current = None;
        core::mem::swap(&mut self.current, &mut current);

        if let Some(current) = current {
            Some((self.namespace.clone(), current))
        } else {
            loop {
                let mut hier_iter_var = None;
                core::mem::swap(&mut self.hier_iter, &mut hier_iter_var);

                if let Some(mut hier_iter) = hier_iter_var {
                    let x = hier_iter.next();
                    if x.is_some() {
                        self.hier_iter = Some(hier_iter);
                        break x;
                    }
                    // if none, then we iterate
                } else {
                    if let Some((next_ident, next_hier)) = self.ns.next() {
                        let next_namespace = self.namespace.clone().append(next_ident.clone());
                        let x = next_hier.hier_iterator(next_namespace, self.f.clone());
                        self.hier_iter = Some(Box::new(x));
                    } else {
                        break None;
                    }
                }
            }
        }
    }
}

/// Flat hierarchy iterator for Hier<T>
///
/// 'a is the lifetime of the initial Hier<T>
/// I is the item inside the second element of the Iterator of the T
/// J is the iterator object created from T which return a Tuple of elements, (Ident, I)
/// T is the object embedded inside Hier
/// F is the closure to create the iterator associated with T
pub struct HierFlatIterator<'a, I: 'a, J, T, F: Fn(&'a T) -> J>
where
    J: Iterator<Item = (&'a Ident, &'a I)>,
{
    namespace: Namespace,
    current: CurrentOrSub<J, Box<HierFlatIterator<'a, I, J, T, F>>>,
    ns: hash_map::Iter<'a, Ident, Hier<T>>,
    f: alloc::rc::Rc<F>,
}

pub enum CurrentOrSub<A, B> {
    Current(A),
    Sub(B),
}

impl<'a, I, J, T, F: Fn(&'a T) -> J> Iterator for HierFlatIterator<'a, I, J, T, F>
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
            match self.ns.next() {
                None => None,
                Some((ns, hier)) => {
                    let sub_namespace = self.namespace.clone().append(ns.clone());
                    let x = hier.flat_iterator(sub_namespace, self.f.clone());
                    self.current = CurrentOrSub::Sub(Box::new(x));
                    self.next()
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::rc::Rc;
    use alloc::vec;

    type FakeMap = HashMap<Ident, u32>;

    fn get_iter<'a>(map: &'a FakeMap) -> hashbrown::hash_map::Iter<'a, Ident, u32> {
        map.iter()
    }

    fn fake_hier() -> Hier<FakeMap> {
        // create the following hier
        // [ ("a", 10), ("sub::b"), 11), ("sub2::subsub::c", 12) ]

        let mut root_h: FakeMap = HashMap::new();
        root_h.insert(Ident::from("a"), 10u32);

        let mut sub_h: FakeMap = HashMap::new();
        sub_h.insert(Ident::from("b"), 11u32);

        let mut sub_sub_h: FakeMap = HashMap::new();
        sub_sub_h.insert(Ident::from("c"), 12u32);

        let mut hier = Hier::new(root_h);
        let mut sub_hier = Hier::new(FakeMap::new());
        sub_hier
            .add_ns_hier(
                Namespace::root().append(Ident::from("subsub")),
                Hier::new(sub_sub_h),
            )
            .unwrap();

        hier.add_ns(Ident::from("sub"), sub_h).unwrap();
        hier.add_ns_hier(Namespace::root().append(Ident::from("sub2")), sub_hier)
            .unwrap();

        hier
    }

    #[test]
    fn works_hier() {
        let hier = fake_hier();
        let vals = hier
            .hier_iterator(Namespace::root(), Rc::new(get_iter))
            .collect::<Vec<_>>();

        let expected_namespaces = [
            Namespace::root(),
            Namespace::root().append(Ident::from("sub")),
            Namespace::root().append(Ident::from("sub2")),
            Namespace::root()
                .append(Ident::from("sub2"))
                .append(Ident::from("subsub")),
        ];
        let expected_values = [
            vec![(Ident::from("a"), &10u32)],
            vec![(Ident::from("b"), &11u32)],
            vec![],
            vec![(Ident::from("c"), &12u32)],
        ];

        let mut bits = 0b1111;
        for (namespace, i) in vals.into_iter() {
            let x = i.map(|(i, x)| (i.clone(), x)).collect::<Vec<_>>();

            let mut found = false;
            for (i, (n, vs)) in expected_namespaces
                .iter()
                .zip(expected_values.iter())
                .enumerate()
            {
                if &namespace == n {
                    if bits & (1 << i) == 0 {
                        panic!("duplicated call to namespace {:?}", n);
                    }
                    assert_eq!(vs, &x, "not equal in namespace {:?}", n);
                    bits = bits & !(1 << i);
                    found = true;
                    break;
                }
            }
            if !found {
                panic!("unexpected namespace {:?}", namespace)
            }
        }
        assert_eq!(bits, 0)
    }

    #[test]
    fn works_flat() {
        let hier = fake_hier();

        let mut vals = hier
            .flat_iterator(Namespace::root(), Rc::new(get_iter))
            .collect::<Vec<_>>();
        vals.sort_by(|(a, _), (b, _)| a.cmp(b));
        let sub = Namespace::root().append(Ident::from("sub"));
        let sub2 = Namespace::root()
            .append(Ident::from("sub2"))
            .append(Ident::from("subsub"));
        let a_path = AbsPath::new(&Namespace::root(), &Ident::from("a"));
        let b_path = AbsPath::new(&sub, &Ident::from("b"));
        let c_path = AbsPath::new(&sub2, &Ident::from("c"));

        assert_eq!(vals, vec![(a_path, &10), (b_path, &11), (c_path, &12)]);
    }
}
