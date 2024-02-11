#[cfg(feature = "threadsafe")]
/// threadsafe refcount alias to Arc
pub type WerRefCount<A> = std::sync::Arc<A>;

#[cfg(not(feature = "threadsafe"))]
/// Not threadsafe refcount alias to Rc
pub type WerRefCount<A> = alloc::rc::Rc<A>;
