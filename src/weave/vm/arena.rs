use std::marker::PhantomData;

/// A generational arena for managing object lifetimes safely
/// 
/// Objects are stored in dense vectors with handles that include generation counters
/// to detect use-after-free and enable safe copying of handles.
pub struct Arena<T> {
    /// Dense storage of objects (Some = alive, None = freed)
    objects: Vec<Option<T>>,
    /// Generation counter for each slot (incremented on free)
    generations: Vec<u32>,
    /// Free list of available slots for reuse
    free_list: Vec<usize>,
    /// Next generation counter for new allocations
    next_generation: u32,
}

/// A handle to an object in the arena with generation checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handle<T> {
    /// Index into the arena's objects vector
    index: usize,
    /// Expected generation - must match arena's generation for this slot
    generation: u32,
    /// Phantom data to tie handle to specific arena type
    _phantom: PhantomData<T>,
}

impl<T> Arena<T> {
    /// Create a new empty arena
    pub fn new() -> Self {
        Self {
            objects: Vec::new(),
            generations: Vec::new(),
            free_list: Vec::new(),
            next_generation: 1, // Start at 1 so 0 can be invalid
        }
    }
    
    /// Create a new arena with pre-allocated capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            objects: Vec::with_capacity(capacity),
            generations: Vec::with_capacity(capacity),
            free_list: Vec::new(),
            next_generation: 1,
        }
    }
    
    /// Insert an object into the arena and return a handle to it
    pub fn insert(&mut self, object: T) -> Handle<T> {
        let generation = self.next_generation;
        
        if let Some(index) = self.free_list.pop() {
            // Reuse a freed slot
            self.objects[index] = Some(object);
            self.generations[index] = generation;
            self.next_generation = self.next_generation.wrapping_add(1);
            
            Handle {
                index,
                generation,
                _phantom: PhantomData,
            }
        } else {
            // Allocate a new slot
            let index = self.objects.len();
            self.objects.push(Some(object));
            self.generations.push(generation);
            self.next_generation = self.next_generation.wrapping_add(1);
            
            Handle {
                index,
                generation,
                _phantom: PhantomData,
            }
        }
    }
    
    /// Get a reference to an object by handle
    /// Returns None if the handle is invalid (generation mismatch or freed)
    pub fn get(&self, handle: Handle<T>) -> Option<&T> {
        if handle.index >= self.objects.len() {
            return None;
        }
        
        if self.generations[handle.index] != handle.generation {
            return None; // Handle is stale
        }
        
        self.objects[handle.index].as_ref()
    }
    
    /// Get a mutable reference to an object by handle
    /// Returns None if the handle is invalid (generation mismatch or freed)
    pub fn get_mut(&mut self, handle: Handle<T>) -> Option<&mut T> {
        if handle.index >= self.objects.len() {
            return None;
        }
        
        if self.generations[handle.index] != handle.generation {
            return None; // Handle is stale
        }
        
        self.objects[handle.index].as_mut()
    }
    
    /// Remove an object from the arena, making its handle invalid
    /// Returns the object if the handle was valid
    pub fn remove(&mut self, handle: Handle<T>) -> Option<T> {
        if handle.index >= self.objects.len() {
            return None;
        }
        
        if self.generations[handle.index] != handle.generation {
            return None; // Handle is stale
        }
        
        let object = self.objects[handle.index].take();
        if object.is_some() {
            // Increment generation to invalidate existing handles
            self.generations[handle.index] = self.generations[handle.index].wrapping_add(1);
            self.free_list.push(handle.index);
        }
        
        object
    }
    
    /// Check if a handle is valid (object exists and generation matches)
    pub fn is_valid(&self, handle: Handle<T>) -> bool {
        handle.index < self.objects.len() 
            && self.generations[handle.index] == handle.generation
            && self.objects[handle.index].is_some()
    }
    
    /// Clear all objects from the arena, invalidating all handles
    pub fn clear(&mut self) {
        self.objects.clear();
        self.generations.clear();
        self.free_list.clear();
        self.next_generation = 1;
    }
    
    /// Get the number of live objects in the arena
    pub fn len(&self) -> usize {
        self.objects.iter().filter(|obj| obj.is_some()).count()
    }
    
    /// Check if the arena is empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    
    /// Get the total capacity (including freed slots)
    pub fn capacity(&self) -> usize {
        self.objects.len()
    }
    
    /// Iterator over all live objects and their handles
    pub fn iter(&self) -> ArenaIterator<T> {
        ArenaIterator {
            arena: self,
            index: 0,
        }
    }
}

impl<T> Default for Arena<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Handle<T> {
    /// Get the raw index (for debugging or serialization)
    pub fn index(self) -> usize {
        self.index
    }
    
    /// Get the generation (for debugging or serialization)
    pub fn generation(self) -> u32 {
        self.generation
    }
    
    /// Create a handle from raw parts (unsafe - no validation)
    /// This should only be used for deserialization or testing
    pub unsafe fn from_raw_parts(index: usize, generation: u32) -> Self {
        Self {
            index,
            generation,
            _phantom: PhantomData,
        }
    }
    
    /// Pack handle into a u64 for storage in NanBoxedValue
    /// Lower 32 bits: index, Upper 32 bits: generation
    pub fn to_u64(self) -> u64 {
        ((self.generation as u64) << 32) | (self.index as u64)
    }
    
    /// Unpack handle from u64 (for NanBoxedValue integration)
    pub fn from_u64(value: u64) -> Self {
        let index = (value & 0xFFFFFFFF) as usize;
        let generation = (value >> 32) as u32;
        Self {
            index,
            generation,
            _phantom: PhantomData,
        }
    }
}

/// Iterator over live objects in an arena
pub struct ArenaIterator<'a, T> {
    arena: &'a Arena<T>,
    index: usize,
}

impl<'a, T> Iterator for ArenaIterator<'a, T> {
    type Item = (Handle<T>, &'a T);
    
    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.arena.objects.len() {
            if let Some(ref object) = self.arena.objects[self.index] {
                let handle = Handle {
                    index: self.index,
                    generation: self.arena.generations[self.index],
                    _phantom: PhantomData,
                };
                self.index += 1;
                return Some((handle, object));
            }
            self.index += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_arena_operations() {
        let mut arena = Arena::new();
        
        // Insert some objects
        let handle1 = arena.insert("hello");
        let handle2 = arena.insert("world");
        
        // Verify we can retrieve them
        assert_eq!(arena.get(handle1), Some(&"hello"));
        assert_eq!(arena.get(handle2), Some(&"world"));
        
        // Verify arena state
        assert_eq!(arena.len(), 2);
        assert!(!arena.is_empty());
    }
    
    #[test]
    fn test_handle_invalidation() {
        let mut arena = Arena::new();
        
        let handle = arena.insert(42);
        assert_eq!(arena.get(handle), Some(&42));
        assert!(arena.is_valid(handle));
        
        // Remove the object
        let removed = arena.remove(handle);
        assert_eq!(removed, Some(42));
        
        // Handle should now be invalid
        assert_eq!(arena.get(handle), None);
        assert!(!arena.is_valid(handle));
    }
    
    #[test]
    fn test_slot_reuse() {
        let mut arena = Arena::new();
        
        // Insert and remove an object
        let handle1 = arena.insert("first");
        assert_eq!(handle1.index(), 0);
        arena.remove(handle1);
        
        // Insert another object - should reuse the slot but with new generation
        let handle2 = arena.insert("second");
        assert_eq!(handle2.index(), 0); // Same index
        assert_ne!(handle2.generation(), handle1.generation()); // Different generation
        
        // Old handle should be invalid, new handle should work
        assert!(!arena.is_valid(handle1));
        assert!(arena.is_valid(handle2));
        assert_eq!(arena.get(handle2), Some(&"second"));
    }
    
    #[test]
    fn test_handle_serialization() {
        let mut arena = Arena::new();
        let handle = arena.insert(123);
        
        // Test u64 round-trip
        let packed = handle.to_u64();
        let unpacked = Handle::from_u64(packed);
        
        assert_eq!(handle, unpacked);
        assert_eq!(arena.get(unpacked), Some(&123));
    }
    
    #[test]
    fn test_iterator() {
        let mut arena = Arena::new();
        
        let h1 = arena.insert("a");
        let h2 = arena.insert("b");
        let h3 = arena.insert("c");
        
        // Remove middle element
        arena.remove(h2);
        
        // Iterator should only return live objects
        let mut items: Vec<_> = arena.iter().collect();
        items.sort_by_key(|(h, _)| h.index());
        
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].1, &"a");
        assert_eq!(items[1].1, &"c");
    }
    
    #[test]
    fn test_clear() {
        let mut arena = Arena::new();
        
        let h1 = arena.insert(1);
        let h2 = arena.insert(2);
        
        assert_eq!(arena.len(), 2);
        
        arena.clear();
        
        assert_eq!(arena.len(), 0);
        assert!(arena.is_empty());
        assert!(!arena.is_valid(h1));
        assert!(!arena.is_valid(h2));
    }
}