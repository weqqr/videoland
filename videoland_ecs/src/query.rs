use std::cell::{Ref, RefMut};
use std::ops::{Deref, DerefMut};

use crate::exec::SystemParam;
use crate::{Archetype, Column, ColumnMut, Registry};

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

pub trait Fetch {
    type Output<'a>;

    fn fetch(archetype: &Archetype) -> Self::Output<'_>;
}

impl<T: 'static> Fetch for &T {
    type Output<'a> = Column<'a, T>;

    fn fetch(archetype: &Archetype) -> Self::Output<'_> {
        archetype.get_component_column_by_type::<T>()
    }
}

impl<T: 'static> Fetch for &mut T {
    type Output<'a> = ColumnMut<'a, T>;

    fn fetch(archetype: &Archetype) -> Self::Output<'_> {
        archetype.get_component_column_mut_by_type::<T>()
    }
}

pub struct Query<'a, M: Match> {
    columns: M::Output<'a>,
}

impl<'a, M: Match> SystemParam for Query<'a, M> {
    type Item<'w> = Query<'w, M>;

    fn get(reg: &Registry) -> Self::Item<'_> {
        M::match_columns(&reg.archetypes[0])
    }
}

pub trait Match: Sized {
    type Output<'a>;

    fn match_columns(arch: &Archetype) -> Query<Self>;
}

macro_rules! impl_match_for_fetch_tuple {
    ($($ty:ident),*) => {
        impl<$($ty: Fetch),*> Match for ($($ty,)*) {
            type Output<'a> = ($($ty::Output<'a>,)*);

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn match_columns(arch: &Archetype) -> Query<Self> {
                $(
                    let $ty = $ty::fetch(arch);
                )*

                Query {
                    columns: ($($ty,)*),
                }
            }
        }
    };
}

impl_match_for_fetch_tuple!();
impl_match_for_fetch_tuple!(A);
impl_match_for_fetch_tuple!(A, B);
impl_match_for_fetch_tuple!(A, B, C);
impl_match_for_fetch_tuple!(A, B, C, D);
impl_match_for_fetch_tuple!(A, B, C, D, E);
impl_match_for_fetch_tuple!(A, B, C, D, E, F);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G, H);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G, H, I);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_match_for_fetch_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
