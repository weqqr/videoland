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
}

impl Registry {
    pub fn new() -> Self {
        Self {
            resources: HashMap::default(),
            archetypes: Vec::new(),
        }
    }

    pub fn insert<R: 'static>(&mut self, r: R) {
        let id = TypeId::of::<R>();
        self.resources.insert(id, Box::new(RefCell::new(r)));
    }

    pub fn res<R: 'static>(&self) -> Ref<R> {
        let id = TypeId::of::<R>();
        let r = self.resources[&id].borrow();
        Ref::map(r, |x| x.downcast_ref().unwrap())
    }

    pub fn res_mut<R: 'static>(&self) -> RefMut<R> {
        let id = TypeId::of::<R>();
        let r = self.resources[&id].borrow_mut();
        RefMut::map(r, |x| x.downcast_mut().unwrap())
    }

    pub fn archetypes(&self) -> &Vec<Archetype> {
        &self.archetypes
    }
}

pub struct Entity {
    id: u32,
    generation: u32,
}

pub struct Archetype {
    // column types
    types: Vec<TypeId>,

    // len(entities) == len(component_table.column())
    entities: Vec<Entity>,
    component_columns: Vec<Box<RefCell<dyn Any>>>,
}

pub struct Column<'a, T> {
    inner: Ref<'a, Vec<T>>,
}

pub struct ColumnMut<'a, T> {
    inner: RefMut<'a, Vec<T>>,
}

impl Archetype {
    fn has_component<T: 'static>(&self) -> bool {
        let id = TypeId::of::<T>();
        self.types.contains(&id)
    }

    fn get_component_column_by_type<T: 'static>(&self) -> Column<T> {
        let id = TypeId::of::<T>();
        let id = self.types.iter().position(|x| *x == id).unwrap();
        let column = self.component_columns[id].borrow();
        let column = Ref::map(column, |c| c.downcast_ref::<Vec<T>>().unwrap());

        Column {
            inner: column,
        }
    }

    fn get_component_column_mut_by_type<T: 'static>(&self) -> ColumnMut<T> {
        let id = TypeId::of::<T>();
        let id = self.types.iter().position(|x| *x == id).unwrap();
        let column = self.component_columns[id].borrow_mut();
        let column = RefMut::map(column, |c| c.downcast_mut::<Vec<T>>().unwrap());

        ColumnMut {
            inner: column,
        }
    }

    fn get_entities(&self) -> &Vec<Entity> {
        &self.entities
    }
}
