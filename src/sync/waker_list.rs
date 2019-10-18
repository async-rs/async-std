use crate::task::Waker;

use std::num::NonZeroUsize;

struct WakerNode {
    /// Previous `WakerNode` in the queue. If this node is the first node, it shall point to the last node.
    prev_in_queue: *mut WakerNode,
    /// Next `WakerNode` in the queue. If this node is the last node, it shall be null.
    next_in_queue: *mut WakerNode,
    waker: Option<Waker>,
}

pub struct WakerList {
    head: *mut WakerNode,
}

unsafe impl Send for WakerList {}
unsafe impl Sync for WakerList {}

impl WakerList {
    /// Create a new empty `WakerList`
    pub fn new() -> Self {
        Self {
            head: std::ptr::null_mut(),
        }
    }

    /// Insert a waker to the back of the list, and return its key.
    pub fn insert(&mut self, waker: Option<Waker>) -> NonZeroUsize {
        let node = Box::into_raw(Box::new(WakerNode {
            waker,
            next_in_queue: std::ptr::null_mut(),
            prev_in_queue: std::ptr::null_mut(),
        }));

        if self.head.is_null() {
            unsafe {
                (*node).prev_in_queue = node;
            }
            self.head = node;
        } else {
            unsafe {
                let prev = std::mem::replace(&mut (*self.head).prev_in_queue, node);
                (*prev).next_in_queue = node;
                (*node).prev_in_queue = prev;
            }
        }

        unsafe { NonZeroUsize::new_unchecked(node as usize) }
    }

    /// Remove a waker by its key.
    ///
    /// # Safety
    /// This function is unsafe because there is no guarantee that key is the previously returned
    /// key, and that the key is only removed once.
    pub unsafe fn remove(&mut self, key: NonZeroUsize) -> Option<Waker> {
        let node = key.get() as *mut WakerNode;
        let prev = (*node).prev_in_queue;
        let next = (*node).next_in_queue;

        // Special treatment on removing first node
        if self.head == node {
            self.head = next;
        } else {
            std::mem::replace(&mut (*prev).next_in_queue, next);
        }

        // Special treatment on removing last node
        if next.is_null() {
            if !self.head.is_null() {
                std::mem::replace(&mut (*self.head).prev_in_queue, prev);
            }
        } else {
            std::mem::replace(&mut (*next).prev_in_queue, prev);
        }

        Box::from_raw(node).waker
    }

    /// Get a waker by its key.
    ///
    /// # Safety
    /// This function is unsafe because there is no guarantee that key is the previously returned
    /// key, and that the key is not removed.
    pub unsafe fn get(&mut self, key: NonZeroUsize) -> &mut Option<Waker> {
        &mut (*(key.get() as *mut WakerNode)).waker
    }

    /// Check if this list is empty.
    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// Get an iterator over all wakers.
    pub fn iter_mut(&mut self) -> Iter<'_> {
        Iter {
            ptr: self.head,
            _marker: std::marker::PhantomData,
        }
    }

    /// Wake the first waker in the list, and convert it to `None`. This function is named `weak` as
    /// nothing is performed when the first waker is waken already.
    pub fn wake_one_weak(&mut self) {
        if let Some(opt_waker) = self.iter_mut().next() {
            if let Some(w) = opt_waker.take() {
                w.wake();
            }
        }
    }
}

pub struct Iter<'a> {
    ptr: *mut WakerNode,
    _marker: std::marker::PhantomData<&'a ()>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = &'a mut Option<Waker>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr.is_null() {
            return None;
        }
        let next = unsafe { (*self.ptr).next_in_queue };
        let ptr = std::mem::replace(&mut self.ptr, next);
        Some(unsafe { &mut (*ptr).waker })
    }
}
