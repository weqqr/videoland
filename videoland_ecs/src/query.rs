use std::cell::{Ref, RefMut};
use std::ops::{Deref, DerefMut};

use crate::exec::SystemParam;
use crate::{Archetype, Registry, Column, ColumnMut};

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

pub trait Fetch<'a> {
    type Output;

    fn fetch(archetype: &'a Archetype) -> Self::Output;
}

impl<'a, T: 'static> Fetch<'a> for &T {
    type Output = Column<'a, T>;

    fn fetch(archetype: &'a Archetype) -> Self::Output {
        archetype.get_component_column_by_type::<T>()
    }
}

impl<'a, T: 'static> Fetch<'a> for &'a mut T {
    type Output = ColumnMut<'a, T>;

    fn fetch(archetype: &'a Archetype) -> Self::Output {
        archetype.get_component_column_mut_by_type::<T>()
    }
}
