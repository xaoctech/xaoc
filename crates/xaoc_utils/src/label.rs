use std::any::TypeId;
use std::borrow::Cow;
use std::fmt::{Debug, Formatter};
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;
#[cfg(feature = "check_label_hashes")]
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Label<Domain> {
    id: u64,
    name: &'static str,
    _marker: PhantomData<Domain>,
    #[cfg(feature = "check_label_hashes")]
    validated: AtomicBool,
}

impl<Domain> Debug for Label<Domain> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "Label<{}>({:?}, 0x{:08x})", std::any::type_name::<Domain>(), self.name, self.id)
    }
}

impl<Domain: 'static> Label<Domain> {
    const fn static_new(name: &'static str) -> Self {
        Self {
            id: const_fnv1a_hash::fnv1a_hash_str_64(name),
            name,
            #[cfg(feature = "check_label_hashes")]
            validated: AtomicBool::new(false),
            _marker: PhantomData
        }
    }

    fn new<S: Into<Cow<'static, str>>>(name: S) -> Self {
        let name = name.into();
        let id = const_fnv1a_hash::fnv1a_hash_str_64(name.as_ref());
        let name = hashes_table::find(TypeId::of::<Domain>(), id, name);
        Self {
            id,
            name,
            #[cfg(feature = "check_label_hashes")]
            validated: AtomicBool::new(true),
            _marker: PhantomData
        }
    }

    fn id(&self) -> u64 {
        #[cfg(feature = "check_label_hashes")]
        if !self.validated.load(Ordering::Relaxed) {
            hashes_table::find(TypeId::of::<Domain>(), self.id, Cow::from(self.name));
            self.validated.store(true, Ordering::Relaxed)
        }
        self.id
    }

    fn name(&self) -> &str {
        self.name
    }

}

impl<Domain> Clone for Label<Domain> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name,
            #[cfg(feature = "check_label_hashes")]
            validated: AtomicBool::new(self.validated.load(Ordering::Relaxed)),
            _marker: PhantomData,
        }
    }
}
// impl<Domain> Copy for Label<Domain> {}

impl<Domain> PartialEq for Label<Domain> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}
impl<Domain> Eq for Label<Domain> {}
impl<Domain> Hash for Label<Domain> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

mod hashes_table {
    use std::any::TypeId;
    use std::borrow::Cow;
    use hashbrown::hash_map::Entry;
    use parking_lot::Mutex;
    use crate::hash::{HashMap};

    const HASHES: once_cell::sync::Lazy<Mutex<HashMap<(TypeId, u64), &'static str>>> = once_cell::sync::Lazy::new(|| {
        Mutex::new(HashMap::new())
    });

    pub fn find(type_id: TypeId, id: u64, name: Cow<'static, str>) -> &'static str {
        let hashes = &*HASHES;
        let mut _lock = hashes.lock();
        match _lock.entry((type_id, id)) {
            Entry::Occupied(o) => if *o.get() != name { panic!("Duplicate hash value {:08x} for strings {:?} and {:?}", id, name, o.get()) } else { *o.get() },
            Entry::Vacant(v) => v.insert(match name {
                Cow::Borrowed(b) => b,
                Cow::Owned(o) => Box::leak(o.into_boxed_str())
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Tag;

    #[test]
    fn static_label() {
        const L1: Label::<Tag> = Label::static_new("1");
        const L2: Label::<Tag> = Label::static_new("2");
        const L3: Label::<Tag> = Label::static_new("1");
        assert_ne!(L1, L2);
        assert_ne!(L1.id(), L2.id());
        assert_ne!(L1.name(), L2.name());
        assert_eq!(L1, L3);
        assert_eq!(L1.id(), L3.id());
        assert_eq!(L1.name(), L3.name());
    }

    #[test]
    fn dynamic_label() {
        const L1: Label::<Tag> = Label::static_new("1");

        let l1 = Label::new(String::from("1"));
        let l2 = Label::new("2");
        let l3 = Label::new("1");
        assert_eq!(L1, l1);
        assert_eq!(L1.id(), l1.id());
        assert_eq!(L1.name(), l1.name());
        assert_ne!(l1, l2);
        assert_ne!(l1.id(), l2.id());
        assert_ne!(l1.name(), l2.name());
        assert_eq!(l1, l3);
        assert_eq!(l1.id(), l3.id());
        assert_eq!(l1.name(), l3.name());
    }
}
