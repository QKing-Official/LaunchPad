use std::collections::HashSet;
use std::sync::Mutex;

/// Thread-safe port allocator with increments
pub struct PortAllocator {
    next:  Mutex<u16>,
    used:  Mutex<HashSet<u16>>,
}

impl PortAllocator {
    pub fn new(start: u16) -> Self {
        Self {
            next: Mutex::new(start),
            used: Mutex::new(HashSet::new()),
        }
    }

    /// Allocate the next available port and skip used ones
    pub fn allocate(&self) -> u16 {
        let mut next = self.next.lock().unwrap();
        let mut used = self.used.lock().unwrap();
        loop {
            let p = *next;
            *next += 1;
            if !used.contains(&p) {
                used.insert(p);
                return p;
            }
        }
    }

    // Mark port as used
    pub fn mark_used(&self, port: u16) {
        self.used.lock().unwrap().insert(port);
    }

    /// Release it back into the free wide port world!
    pub fn release(&self, port: u16) {
        self.used.lock().unwrap().remove(&port);
    }
}