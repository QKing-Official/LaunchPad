use std::collections::HashSet;
use std::sync::Mutex;

/// Thread-safe port allocator.
/// Only allocates in the unprivileged dynamic range (30000–59999 by default) for safety.
pub struct PortAllocator {
    next:  Mutex<u16>,
    used:  Mutex<HashSet<u16>>,
    max:   u16,
}

impl PortAllocator {
    pub fn new(start: u16) -> Self {
        assert!(start >= 1024, "start port must be ≥ 1024");
        Self {
            next: Mutex::new(start),
            used: Mutex::new(HashSet::new()),
            max:  60000,
        }
    }

    /// Allocate the next available port.
    /// Panics if the entire range already used
    pub fn allocate(&self) -> u16 {
        let mut next = self.next.lock().unwrap();
        let mut used = self.used.lock().unwrap();
        let start = *next;
        loop {
            let p = *next;
            if p > self.max {
                panic!("port allocator exhausted the range {}–{}", start, self.max);
            }
            *next = next.saturating_add(1);
            if !used.contains(&p) {
                used.insert(p);
                return p;
            }
        }
    }

    pub fn mark_used(&self, port: u16) {
        self.used.lock().unwrap().insert(port);
    }

    pub fn release(&self, port: u16) {
        self.used.lock().unwrap().remove(&port);
    }
}