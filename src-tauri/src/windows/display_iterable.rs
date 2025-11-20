use std::cell::RefCell;
use windows::Win32::Foundation::E_BOUNDS;
use windows::{core::*, Devices::Display::Core::DisplayTarget};
use windows_collections::{IIterable, IIterable_Impl, IIterator, IIterator_Impl};

#[implement(IIterable<DisplayTarget>)]
pub struct DisplayTargetIterable {
    items: Vec<DisplayTarget>,
}

impl DisplayTargetIterable {
    pub fn new(items: Vec<DisplayTarget>) -> Self {
        Self { items }
    }

    pub fn from_single(item: DisplayTarget) -> Self {
        Self { items: vec![item] }
    }
}

impl IIterable_Impl<DisplayTarget> for DisplayTargetIterable_Impl {
    fn First(&self) -> Result<IIterator<DisplayTarget>> {
        Ok(DisplayTargetIterator::new(self.items.clone()).into())
    }
}

#[implement(IIterator<DisplayTarget>)]
pub struct DisplayTargetIterator {
    items: Vec<DisplayTarget>,
    current: RefCell<usize>,
}

impl DisplayTargetIterator {
    fn new(items: Vec<DisplayTarget>) -> Self {
        Self {
            items,
            current: RefCell::new(0),
        }
    }
}

impl IIterator_Impl<DisplayTarget> for DisplayTargetIterator_Impl {
    fn Current(&self) -> Result<DisplayTarget> {
        let idx = *self.current.borrow();
        if idx < self.items.len() {
            Ok(self.items[idx].clone())
        } else {
            Err(Error::new(E_BOUNDS, "Iterator out of bounds"))
        }
    }

    fn HasCurrent(&self) -> Result<bool> {
        let idx = *self.current.borrow();
        Ok(idx < self.items.len())
    }

    fn MoveNext(&self) -> Result<bool> {
        let mut idx = self.current.borrow_mut();
        if *idx < self.items.len() {
            *idx += 1;
        }
        Ok(*idx < self.items.len())
    }

    fn GetMany(
        &self,
        items: &mut [std::option::Option<windows::Devices::Display::Core::DisplayTarget>],
    ) -> Result<u32> {
        let mut idx = self.current.borrow_mut();
        let mut count = 0u32;

        for item in items.iter_mut() {
            if *idx < self.items.len() {
                *item = Option::from(self.items[*idx].clone());
                *idx += 1;
                count += 1;
            } else {
                break;
            }
        }

        Ok(count)
    }
}
