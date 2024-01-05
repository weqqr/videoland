use std::any::{Any, TypeId};
use std::cell::{Ref, RefMut, UnsafeCell};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use ahash::{AHashMap, AHashSet};

use crate::exec::SystemParam;
use crate::Registry;

pub struct Res<'a, T: 'static> {
    value: Ref<'a, T>,
}

impl<'a, T> SystemParam for Res<'a, T> {
    type Item<'w> = Res<'w, T>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        Res {
            value: reg.res::<T>(),
        }
    }
}

impl<'a, T> Deref for Res<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        Ref::deref(&self.value)
    }
}

pub struct ResMut<'a, T: 'static> {
    value: RefMut<'a, T>,
}

impl<'a, T> SystemParam for ResMut<'a, T> {
    type Item<'w> = ResMut<'w, T>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        ResMut {
            value: reg.res_mut::<T>(),
        }
    }
}

impl<'a, T> Deref for ResMut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        RefMut::deref(&self.value)
    }
}

impl<'a, T> DerefMut for ResMut<'a, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        RefMut::deref_mut(&mut self.value)
    }
}

//
//
//

pub type UntypedColumn = Box<dyn Any>;

pub struct Archetype {
    component_types: AHashSet<TypeId>,
    component_columns: AHashMap<TypeId, UntypedColumn>,
}

impl Archetype {
    pub fn new(types: AHashSet<TypeId>) -> Self {
        Self {
            component_types: types,
            component_columns: AHashMap::new(),
        }
    }

    pub fn has(&self, ty: TypeId) -> bool {
        self.component_types.contains(&ty)
    }

    pub fn column<T: 'static>(&self) -> &UnsafeCell<Vec<T>> {
        self.component_columns
            .get(&TypeId::of::<T>())
            .unwrap()
            .downcast_ref()
            .unwrap()
    }
}

pub struct Col<'a, T> {
    inner: &'a UnsafeCell<Vec<T>>,
}

impl<'a, T> Column for Col<'a, T> {
    type Item = &'a T;

    fn component_by_id(&self, id: usize) -> Option<Self::Item> {
        unsafe { (*self.inner.get()).get(id) }
    }
}

pub struct ColMut<'a, T> {
    inner: &'a UnsafeCell<Vec<T>>,
}

impl<'a, T> Column for ColMut<'a, T> {
    type Item = &'a mut T;

    fn component_by_id(&self, id: usize) -> Option<Self::Item> {
        unsafe { (*self.inner.get()).get_mut(id) }
    }
}

pub struct Query<'a, M: Matcher> {
    archetypes: &'a Vec<Archetype>,
    _pd: PhantomData<fn() -> M>,
}

impl<'a, M: Matcher> SystemParam for Query<'a, M> {
    type Item<'w> = Query<'w, M>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        Query {
            archetypes: &reg.archetypes,
            _pd: PhantomData,
        }
    }
}

impl<'a, M: Matcher> Query<'a, M> {
    pub fn iter(&self) -> impl Iterator<Item = M::Row<'_>> + '_ {
        self.archetypes
            .iter()
            .filter(|archetype| M::matches(archetype))
            .flat_map(|archetype| M::iter(archetype))
    }
}

pub trait Matcher {
    type Row<'a>;

    fn matches(archetype: &Archetype) -> bool;
    fn iter(archetype: &Archetype) -> impl Iterator<Item = Self::Row<'_>>;
}

impl<A: Fetch, B: Fetch> Matcher for (A, B) {
    type Row<'a> = (A::ItemRef<'a>, B::ItemRef<'a>);

    fn matches(archetype: &Archetype) -> bool {
        archetype.has(TypeId::of::<A::Item>()) && archetype.has(TypeId::of::<B::Item>())
    }

    fn iter(archetype: &Archetype) -> impl Iterator<Item = Self::Row<'_>> {
        MatchedRows {
            tuple: (A::fetch_column(archetype), B::fetch_column(archetype)),
            index: 0,
        }
    }
}

pub struct MatchedRows<T> {
    tuple: T,
    index: usize,
}

impl<A: Column, B: Column> Iterator for MatchedRows<(A, B)> {
    type Item = (A::Item, B::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let tuple = (
            self.tuple.0.component_by_id(self.index)?,
            self.tuple.1.component_by_id(self.index)?,
        );

        self.index += 1;

        Some(tuple)
    }
}

pub trait Fetch {
    type Item: 'static;
    type ItemRef<'a>;
    type Column<'a>: Column<Item = Self::ItemRef<'a>>;

    fn fetch_column(archetype: &Archetype) -> Self::Column<'_>;
}

impl<T: 'static> Fetch for &T {
    type Item = T;
    type ItemRef<'a> = &'a T;
    type Column<'a> = Col<'a, T>;

    fn fetch_column(archetype: &Archetype) -> Self::Column<'_> {
        Col {
            inner: archetype.column::<T>(),
        }
    }
}

impl<T: 'static> Fetch for &mut T {
    type Item = T;
    type ItemRef<'a> = &'a mut T;
    type Column<'a> = ColMut<'a, T>;

    fn fetch_column(archetype: &Archetype) -> Self::Column<'_> {
        ColMut {
            inner: archetype.column::<T>(),
        }
    }
}

pub trait Column {
    type Item;

    fn component_by_id(&self, id: usize) -> Option<Self::Item>;
}
