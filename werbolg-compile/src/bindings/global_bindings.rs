use super::bindings::Bindings;
use super::types::BindingType;
use crate::hier::Hier;
use werbolg_core::{AbsPath, Namespace};

pub struct GlobalBindings(pub(crate) Hier<Bindings<BindingType>>);

impl GlobalBindings {
    pub fn new() -> Self {
        Self(Hier::default())
    }

    pub fn add(&mut self, name: AbsPath, value: BindingType) -> Result<(), ()> {
        let (namespace, ident) = name.split();

        if !self.0.namespace_exist(namespace.clone()) {
            self.0.add_ns_hier(namespace.clone()).unwrap()
        }

        self.0
            .on_mut(&namespace, |bindings| {
                bindings.add(ident.clone(), value.clone())
            })
            .map_err(|_| ())
    }

    #[allow(unused)]
    pub fn get(&self, name: &AbsPath) -> Option<&BindingType> {
        let (namespace, ident) = name.split();
        let bindings = self.0.get(&namespace).unwrap();
        bindings.get(&ident)
    }

    #[allow(unused)]
    pub fn iter<'a>(&'a self) -> impl Iterator<Item = (AbsPath, &'a BindingType)> {
        self.0.iterator(
            Namespace::root(),
            alloc::rc::Rc::new(|x: &'a Bindings<BindingType>| alloc::boxed::Box::new(x.iter())),
        )
    }
}
