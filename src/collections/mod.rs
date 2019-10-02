//! The Rust standard collections
//!
//! This library provides efficient implementations of the most common general purpose programming
//! data structures.

pub mod vec_deque;
pub mod btree_map;

pub use vec_deque::VecDeque;
pub use btree_map::BTreeMap;
