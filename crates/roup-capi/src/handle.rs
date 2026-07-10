//! Safe typed handles backed by a generational arena.
//!
//! Handles never contain pointers into arena storage. Reusing a vacant slot
//! increments its generation, so a handle from a removed object cannot access
//! the replacement object in that slot.

use core::fmt;
use core::marker::PhantomData;

const FIRST_GENERATION: u64 = 1;

/// A handle whose marker type identifies the kind of stored object.
///
/// `T` is used only at compile time. A `Handle<Parser>` cannot be passed to an
/// arena of directives, even though both handles have the same raw shape.
pub struct Handle<T> {
    index: u32,
    generation: u64,
    marker: PhantomData<fn() -> T>,
}

impl<T> Handle<T> {
    /// Reconstruct a typed handle from its ABI representation.
    pub fn from_raw_parts(index: u32, generation: u64) -> Result<Self, HandleError> {
        if generation == 0 {
            return Err(HandleError::ZeroGeneration);
        }

        Ok(Self {
            index,
            generation,
            marker: PhantomData,
        })
    }

    /// Return the zero-based slot index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Return the slot generation captured by this handle.
    #[must_use]
    pub const fn generation(self) -> u64 {
        self.generation
    }
}

impl<T> Copy for Handle<T> {}

impl<T> Clone for Handle<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for Handle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}

impl<T> Eq for Handle<T> {}

impl<T> fmt::Debug for Handle<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Handle")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

/// A hard failure while creating or resolving a handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HandleError {
    /// Generation zero is reserved for the ABI's null handle.
    ZeroGeneration,
    /// Allocating another addressable slot would exceed the `u32` index space.
    CapacityExhausted,
    /// The arena's private reuse list referred to a missing or occupied slot.
    CorruptFreeList { index: u32 },
    /// A live slot existed while the arena's private live count was zero.
    CorruptLiveCount,
    /// The handle's index has never existed in this arena.
    IndexOutOfRange { index: u32 },
    /// The slot exists but belongs to a different object generation.
    GenerationMismatch {
        index: u32,
        expected: u64,
        provided: u64,
    },
    /// The slot has no live object for its current generation.
    Vacant { index: u32, generation: u64 },
}

impl fmt::Display for HandleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroGeneration => formatter.write_str("handle generation zero is invalid"),
            Self::CapacityExhausted => formatter.write_str("handle arena capacity exhausted"),
            Self::CorruptFreeList { index } => {
                write!(formatter, "handle arena free-list entry {index} is corrupt")
            }
            Self::CorruptLiveCount => formatter.write_str("handle arena live count is corrupt"),
            Self::IndexOutOfRange { index } => {
                write!(formatter, "handle index {index} is out of range")
            }
            Self::GenerationMismatch {
                index,
                expected,
                provided,
            } => write!(
                formatter,
                "handle index {index} has generation {expected}, not {provided}"
            ),
            Self::Vacant { index, generation } => write!(
                formatter,
                "handle index {index} is vacant at generation {generation}"
            ),
        }
    }
}

impl std::error::Error for HandleError {}

#[derive(Debug)]
struct Slot<T> {
    generation: u64,
    value: Option<T>,
}

/// An arena that owns values and resolves typed generational handles.
#[derive(Debug)]
pub struct GenerationalArena<T> {
    slots: Vec<Slot<T>>,
    free: Vec<u32>,
    len: usize,
}

impl<T> GenerationalArena<T> {
    /// Create an empty arena.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            slots: Vec::new(),
            free: Vec::new(),
            len: 0,
        }
    }

    /// Insert one value and return its typed handle.
    pub fn insert(&mut self, value: T) -> Result<Handle<T>, HandleError> {
        if let Some(index) = self.free.pop() {
            let Some(slot) = self.slots.get_mut(index as usize) else {
                return Err(HandleError::CorruptFreeList { index });
            };
            if slot.value.is_some() {
                return Err(HandleError::CorruptFreeList { index });
            }
            slot.value = Some(value);
            self.len += 1;
            return Handle::from_raw_parts(index, slot.generation);
        }

        // Reserve `u32::MAX` for the ABI's static emergency diagnostic handle.
        // Normal arenas therefore address slots `0..u32::MAX` exclusively.
        if self.slots.len() >= u32::MAX as usize {
            return Err(HandleError::CapacityExhausted);
        }
        let index = u32::try_from(self.slots.len()).map_err(|_| HandleError::CapacityExhausted)?;
        self.slots.push(Slot {
            generation: FIRST_GENERATION,
            value: Some(value),
        });
        self.len += 1;
        Handle::from_raw_parts(index, FIRST_GENERATION)
    }

    /// Borrow the live value identified by `handle`.
    pub fn get(&self, handle: Handle<T>) -> Result<&T, HandleError> {
        let slot = self.resolve_slot(handle)?;
        slot.value.as_ref().ok_or(HandleError::Vacant {
            index: handle.index,
            generation: handle.generation,
        })
    }

    /// Mutably borrow the live value identified by `handle`.
    #[cfg(test)]
    pub fn get_mut(&mut self, handle: Handle<T>) -> Result<&mut T, HandleError> {
        let slot = self.resolve_slot_mut(handle)?;
        slot.value.as_mut().ok_or(HandleError::Vacant {
            index: handle.index,
            generation: handle.generation,
        })
    }

    /// Remove and return the live value identified by `handle`.
    pub fn remove(&mut self, handle: Handle<T>) -> Result<T, HandleError> {
        let live_count = self.len;
        let (value, recycle) = {
            let slot = self.resolve_slot_mut(handle)?;
            if slot.value.is_none() {
                return Err(HandleError::Vacant {
                    index: handle.index,
                    generation: handle.generation,
                });
            }
            if live_count == 0 {
                return Err(HandleError::CorruptLiveCount);
            }
            let value = slot.value.take().ok_or(HandleError::Vacant {
                index: handle.index,
                generation: handle.generation,
            })?;
            let recycle = if let Some(next_generation) = slot.generation.checked_add(1) {
                slot.generation = next_generation;
                true
            } else {
                false
            };
            (value, recycle)
        };
        self.len = live_count - 1;
        if recycle {
            self.free.push(handle.index);
        }

        Ok(value)
    }

    fn resolve_slot(&self, handle: Handle<T>) -> Result<&Slot<T>, HandleError> {
        let slot = self
            .slots
            .get(handle.index as usize)
            .ok_or(HandleError::IndexOutOfRange {
                index: handle.index,
            })?;
        Self::check_generation(slot, handle)?;
        Ok(slot)
    }

    fn resolve_slot_mut(&mut self, handle: Handle<T>) -> Result<&mut Slot<T>, HandleError> {
        let slot =
            self.slots
                .get_mut(handle.index as usize)
                .ok_or(HandleError::IndexOutOfRange {
                    index: handle.index,
                })?;
        Self::check_generation(slot, handle)?;
        Ok(slot)
    }

    fn check_generation(slot: &Slot<T>, handle: Handle<T>) -> Result<(), HandleError> {
        if slot.generation != handle.generation {
            return Err(HandleError::GenerationMismatch {
                index: handle.index,
                expected: slot.generation,
                provided: handle.generation,
            });
        }
        Ok(())
    }
}

impl<T> Default for GenerationalArena<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removed_handle_is_stale_after_slot_reuse() {
        let mut arena = GenerationalArena::new();
        let removed = arena.insert(String::from("old")).unwrap();

        assert_eq!(arena.remove(removed).unwrap(), "old");
        let replacement = arena.insert(String::from("new")).unwrap();

        assert_eq!(removed.index(), replacement.index());
        assert_eq!(removed.generation() + 1, replacement.generation());
        assert_eq!(
            arena.get(removed),
            Err(HandleError::GenerationMismatch {
                index: removed.index(),
                expected: replacement.generation(),
                provided: removed.generation(),
            })
        );
        assert_eq!(arena.get(replacement).unwrap(), "new");
    }

    #[test]
    fn wrong_generation_cannot_read_mutate_or_remove() {
        let mut arena = GenerationalArena::new();
        let valid = arena.insert(41_u32).unwrap();
        let wrong = Handle::from_raw_parts(valid.index(), valid.generation() + 7).unwrap();
        let expected = HandleError::GenerationMismatch {
            index: valid.index(),
            expected: valid.generation(),
            provided: wrong.generation(),
        };

        assert_eq!(arena.get(wrong), Err(expected));
        assert_eq!(arena.get_mut(wrong), Err(expected));
        assert_eq!(arena.remove(wrong), Err(expected));
        assert_eq!(arena.get(valid), Ok(&41));
    }

    #[test]
    fn current_generation_of_vacant_slot_is_a_hard_error() {
        let mut arena = GenerationalArena::new();
        let handle = arena.insert(5_u8).unwrap();
        arena.remove(handle).unwrap();
        let vacant = Handle::from_raw_parts(handle.index(), handle.generation() + 1).unwrap();

        assert_eq!(
            arena.get(vacant),
            Err(HandleError::Vacant {
                index: vacant.index(),
                generation: vacant.generation(),
            })
        );
    }

    #[test]
    fn remove_preserves_handle_errors_and_checks_live_count_before_mutation() {
        let unknown = Handle::from_raw_parts(0, FIRST_GENERATION).unwrap();
        let mut empty = GenerationalArena::<u8>::new();
        assert_eq!(
            empty.remove(unknown),
            Err(HandleError::IndexOutOfRange { index: 0 })
        );

        let live = Handle::from_raw_parts(0, FIRST_GENERATION).unwrap();
        let mut corrupt = GenerationalArena {
            slots: vec![Slot {
                generation: FIRST_GENERATION,
                value: Some(7_u8),
            }],
            free: Vec::new(),
            len: 0,
        };
        assert_eq!(corrupt.remove(live), Err(HandleError::CorruptLiveCount));
        assert_eq!(corrupt.get(live), Ok(&7));
    }

    #[test]
    fn zero_generation_and_unknown_index_are_rejected() {
        assert_eq!(
            Handle::<u8>::from_raw_parts(0, 0),
            Err(HandleError::ZeroGeneration)
        );

        let arena = GenerationalArena::<u8>::new();
        let unknown = Handle::from_raw_parts(9, FIRST_GENERATION).unwrap();
        assert_eq!(
            arena.get(unknown),
            Err(HandleError::IndexOutOfRange { index: 9 })
        );
    }

    #[test]
    fn generation_overflow_retires_the_slot() {
        let mut arena = GenerationalArena {
            slots: vec![Slot {
                generation: u64::MAX,
                value: Some("last generation"),
            }],
            free: Vec::new(),
            len: 1,
        };
        let last = Handle::from_raw_parts(0, u64::MAX).unwrap();

        assert_eq!(arena.remove(last).unwrap(), "last generation");
        let next = arena.insert("new slot").unwrap();

        assert_eq!(next.index(), 1);
        assert_eq!(
            arena.get(last),
            Err(HandleError::Vacant {
                index: 0,
                generation: u64::MAX,
            })
        );
    }
}
