#![allow(dead_code)]

pub mod exec;
pub mod query;

pub use exec::*;
pub use query::*;

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};

use ahash::HashMap;

pub struct Registry {
    resources: HashMap<TypeId, Box<RefCell<dyn Any>>>,
    archetypes: Vec<Archetype>,
    last_id: u64,
}

impl Registry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::default(),
            archetypes: Vec::new(),
            last_id: 1,
        }
    }

    pub fn insert<R: 'static>(&mut self, r: R) {
        let id = TypeId::of::<R>();
        self.resources.insert(id, Box::new(RefCell::new(r)));
    }

    pub fn res<R: 'static>(&self) -> Ref<R> {
        let id = TypeId::of::<R>();
        let resource = self
            .resources
            .get(&id)
            .unwrap_or_else(|| {
                panic!("unknown resource: {}", std::any::type_name::<R>());
            })
            .borrow();
        Ref::map(resource, |x| x.downcast_ref().unwrap())
    }

    pub fn res_mut<R: 'static>(&self) -> RefMut<R> {
        let id = TypeId::of::<R>();
        let r = self
            .resources
            .get(&id)
            .unwrap_or_else(|| {
                panic!("unknown resource: {}", std::any::type_name::<R>());
            })
            .borrow_mut();
        RefMut::map(r, |x| x.downcast_mut().unwrap())
    }

    pub fn archetypes(&self) -> &Vec<Archetype> {
        &self.archetypes
    }

    fn create_archetype(&mut self, types: Vec<TypeId>, columns: Vec<Box<RefCell<dyn Any>>>) -> &mut Archetype {
        let archetype = Archetype::new(types, columns);
        self.archetypes.push(archetype);
        self.archetypes.last_mut().unwrap()
    }

    pub fn spawn<B: Bundle>(&mut self, bundle: B) -> Entity {
        let types = B::component_types();

        let id = self.last_id;
        self.last_id += 1;

        let entity = Entity {
            id,
        };

        let archetype_index = self
            .archetypes
            .iter()
            .position(|a| a.matches_component_types(&types));

        let archetype = match archetype_index {
            Some(p) => &mut self.archetypes[p],
            None => self.create_archetype(types, B::empty_columns()),
        };

        let row_index = archetype.insert_entity(entity);

        bundle.insert_into_archetype(row_index, archetype);

        entity
    }
}

#[derive(Clone, Copy)]
pub struct Entity {
    id: u64,
}

pub struct Column<'a, T> {
    inner: Ref<'a, Vec<T>>,
}

pub struct ColumnMut<'a, T> {
    inner: RefMut<'a, Vec<T>>,
}

pub struct Archetype {
    // column types
    component_types: Vec<TypeId>,

    // len(entities) == len(component_table.column())
    entities: Vec<Entity>,
    component_columns: Vec<Box<RefCell<dyn Any>>>,
}

impl Archetype {
    fn new(component_types: Vec<TypeId>, columns: Vec<Box<RefCell<dyn Any>>>) -> Self {
        Self {
            component_types,
            entities: Vec::new(),
            component_columns: columns,
        }
    }

    fn insert_entity(&mut self, entity: Entity) -> usize {
        let index = self.entities.len();

        self.entities.push(entity);

        index
    }

    fn insert_component_at_index<T: 'static>(&mut self, row_index: usize, component: T) {
        let id = TypeId::of::<T>();
        let id = self.component_types.iter().position(|x| *x == id).unwrap();

        let mut column = self.component_columns[id].borrow_mut();
        let column_data = column.downcast_mut::<Vec<T>>().unwrap();

        if row_index < column_data.len() {
            column_data.insert(row_index, component);
        } else {
            column_data.push(component);
        }
    }

    fn has_component<T: 'static>(&self) -> bool {
        let id = TypeId::of::<T>();

        self.component_types.contains(&id)
    }

    fn matches_component_types(&self, types: &[TypeId]) -> bool {
        let same_len = types.len() == self.component_types.len();

        same_len && types.iter().all(|ty| self.component_types.contains(ty))
    }

    fn get_component_column_by_type<T: 'static>(&self) -> Option<Column<T>> {
        let id = TypeId::of::<T>();
        let id = self.component_types.iter().position(|x| *x == id)?;
        let column = self.component_columns[id].borrow();
        let column = Ref::map(column, |c| c.downcast_ref::<Vec<T>>().unwrap());

        Some(Column { inner: column })
    }

    fn get_component_column_mut_by_type<T: 'static>(&self) -> Option<ColumnMut<T>> {
        let id = TypeId::of::<T>();
        let id = self.component_types.iter().position(|x| *x == id)?;
        let column = self.component_columns[id].borrow_mut();
        let column = RefMut::map(column, |c| c.downcast_mut::<Vec<T>>().unwrap());

        Some(ColumnMut { inner: column })
    }

    fn get_entities(&self) -> &Vec<Entity> {
        &self.entities
    }
}

pub trait Bundle {
    fn component_types() -> Vec<TypeId>;
    fn empty_columns() -> Vec<Box<RefCell<dyn Any>>>;
    fn insert_into_archetype(self, row_index: usize, archetype: &mut Archetype);
}

macro_rules! impl_bundle_for_tuple {
    ($($ty:ident),*) => {
        impl<$($ty: 'static),*> Bundle for ($($ty,)*) {
            fn component_types() -> Vec<TypeId> {
                vec![$(TypeId::of::<$ty>()),*]
            }

            fn empty_columns() -> Vec<Box<RefCell<dyn Any>>> {
                vec![
                    $(
                        Box::new(RefCell::new(Vec::<$ty>::new())),
                    )*
                ]
            }

            #[allow(non_snake_case)]
            #[allow(unused_variables)]
            fn insert_into_archetype(self, row_index: usize, archetype: &mut Archetype) {
                let ($($ty,)*) = self;
                $(
                    archetype.insert_component_at_index(row_index, $ty);
                )*
            }
        }
    };
}

impl_bundle_for_tuple!();
impl_bundle_for_tuple!(A);
impl_bundle_for_tuple!(A, B);
impl_bundle_for_tuple!(A, B, C);
impl_bundle_for_tuple!(A, B, C, D);
impl_bundle_for_tuple!(A, B, C, D, E);
impl_bundle_for_tuple!(A, B, C, D, E, F);
impl_bundle_for_tuple!(A, B, C, D, E, F, G);
impl_bundle_for_tuple!(A, B, C, D, E, F, G, H);
impl_bundle_for_tuple!(A, B, C, D, E, F, G, H, I);
impl_bundle_for_tuple!(A, B, C, D, E, F, G, H, I, J);
impl_bundle_for_tuple!(A, B, C, D, E, F, G, H, I, J, K);
impl_bundle_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L);
impl_bundle_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M);
impl_bundle_for_tuple!(A, B, C, D, E, F, G, H, I, J, K, L, M, N);
