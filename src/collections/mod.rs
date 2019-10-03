//! The Rust standard collections
//!
//! This library provides efficient implementations of the most common general purpose programming
//! data structures.

pub mod vec_deque;
pub mod hash_map;
pub mod hash_set;
pub mod btree_map;
pub mod btree_set;
pub mod binary_heap;

pub use vec_deque::VecDeque;
pub use hash_map::HashMap;
pub use hash_set::HashSet;
pub use btree_map::BTreeMap;
pub use btree_set::BTreeSet;
pub use binary_heap::BinaryHeap;
