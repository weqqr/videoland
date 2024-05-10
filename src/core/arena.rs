use std::marker::PhantomData;

pub struct Arena<T> {
    cells: Vec<ArenaCell<T>>,
    free_cells: Vec<usize>,
    len: usize,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Self {
            cells: Vec::new(),
            free_cells: Vec::new(),
            len: 0,
        }
    }

    pub fn insert(&mut self, item: T) -> ArenaHandle<T> {
        self.len += 1;

        match self.free_cells.pop() {
            Some(index) => {
                self.cells[index].item = Some(item);
                self.cells[index].generation += 1;

                ArenaHandle {
                    index: index as u32,
                    generation: self.cells[index].generation,
                    _pd: PhantomData,
                }
            }
            None => {
                let index = self.cells.len() as u32;
                let generation = 1;
                self.cells.push(ArenaCell {
                    item: Some(item),
                    generation,
                });
                ArenaHandle {
                    index,
                    generation,
                    _pd: PhantomData,
                }
            }
        }
    }

    pub fn remove(&mut self, handle: ArenaHandle<T>) -> Option<T> {
        assert!(handle.index <= self.cells.len() as u32);

        let cell = &mut self.cells[handle.index as usize];

        assert!(cell.generation == handle.generation);

        let mut item = None;

        std::mem::swap(&mut cell.item, &mut item);

        self.free_cells.push(handle.index as usize);

        self.len -= 1;

        item
    }

    pub fn get(&self, handle: ArenaHandle<T>) -> Option<&T> {
        assert!(handle.index <= self.cells.len() as u32);

        let cell = &self.cells[handle.index as usize];

        assert!(cell.generation == handle.generation);

        return cell.item.as_ref();
    }

    pub fn get_mut(&mut self, handle: ArenaHandle<T>) -> Option<&mut T> {
        assert!(handle.index <= self.cells.len() as u32);

        let cell = &mut self.cells[handle.index as usize];

        assert!(cell.generation == handle.generation);

        return cell.item.as_mut();
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = (ArenaHandle<T>, &T)> {
        self.cells.iter().enumerate().filter_map(|(index, cell)| {
            let handle = ArenaHandle {
                index: index as u32,
                generation: cell.generation,
                _pd: PhantomData,
            };

            cell.item.as_ref().map(|value| (handle, value))
        })
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (ArenaHandle<T>, &mut T)> {
        self.cells
            .iter_mut()
            .enumerate()
            .filter_map(|(index, cell)| {
                let handle = ArenaHandle {
                    index: index as u32,
                    generation: cell.generation,
                    _pd: PhantomData,
                };

                cell.item.as_mut().map(|value| (handle, value))
            })
    }
}

struct ArenaCell<T> {
    item: Option<T>,
    generation: u32,
}

pub struct ArenaHandle<T> {
    index: u32,
    generation: u32,
    _pd: PhantomData<fn() -> *mut T>,
}

impl<T> ArenaHandle<T> {
    pub const NONE: ArenaHandle<T> = ArenaHandle {
        index: 0,
        generation: 0,
        _pd: PhantomData,
    };
}

impl<T> PartialEq for ArenaHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for ArenaHandle<T> {}

impl<T> Clone for ArenaHandle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for ArenaHandle<T> {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_get() {
        let mut arena = Arena::new();

        let a1 = arena.insert("a1");
        let b1 = arena.insert("b1");
        let c1 = arena.insert("c1");

        assert_eq!(arena.get(a1), Some(&"a1"));
        assert_eq!(arena.get(b1), Some(&"b1"));
        assert_eq!(arena.get(c1), Some(&"c1"));
    }

    #[test]
    fn insert_remove_len() {
        let mut arena = Arena::new();
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());

        let a1 = arena.insert("a1");
        assert_eq!(arena.len(), 1);

        let a2 = arena.insert("a2");
        assert_eq!(arena.len(), 2);

        let a1_value = arena.remove(a1);
        assert_eq!(arena.get(a1), None);
        assert_eq!(arena.get(a2), Some(&"a2"));
        assert_eq!(a1_value, Some("a1"));

        assert_eq!(arena.len(), 1);

        arena.remove(a2);
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
    }

    #[test]
    fn mut_inplace() {
        let mut arena = Arena::new();

        let a1 = arena.insert("a1");

        assert_eq!(arena.get(a1), Some(&"a1"));

        *arena.get_mut(a1).unwrap() = "b1";

        assert_eq!(arena.get(a1), Some(&"b1"));
    }

    #[test]
    fn free_list_works() {
        let mut arena = Arena::new();

        let a1 = arena.insert("a1");
        let a2 = arena.insert("a2");
        assert_eq!(arena.len(), 2);

        arena.remove(a1);
        assert_eq!(arena.free_cells.len(), 1);

        let a3 = arena.insert("a3");

        assert_eq!(arena.cells.len(), 2);
        assert_eq!(arena.free_cells.len(), 0);
    }
}
