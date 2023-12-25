use self::{InsertionResult::*, SearchResult::*};
use std::{cmp::Ordering::*, mem, ptr};

#[derive(Clone)]
pub struct Node<K, V> {
    keys: Vec<K>,
    edges: Vec<Node<K, V>>,
    vals: Vec<V>,
}

// public funtions
impl<K: Ord, V> Node<K, V> {
    pub fn search(&self, key: &K) -> SearchResult {
        self.search_linear(key)
    }

    // make a new internal node
    pub fn new_internal(capacity: usize) -> Node<K, V> {
        Node {
            keys: Vec::with_capacity(capacity),
            vals: Vec::with_capacity(capacity),
            edges: Vec::with_capacity(capacity + 1),
        }
    }

    // make a leaf node
    pub fn new_leaf(capacity: usize) -> Node<K, V> {
        Node {
            keys: Vec::with_capacity(capacity),
            edges: Vec::new(),
            vals: Vec::with_capacity(capacity),
        }
    }

    /// Make a leaf root from scratch
    pub fn make_leaf_root(b: usize) -> Node<K, V> {
        Node::new_leaf(capacity_from_b(b))
    }

    // make an internal root and swap with an old root
    pub fn make_internal_root(
        left_and_out: &mut Node<K, V>,
        b: usize,
        key: K,
        value: V,
        right: Node<K, V>,
    ) {
        let mut node = Node::new_internal(capacity_from_b(b));
        mem::swap(left_and_out, &mut node);
        left_and_out.keys.push(key);
        left_and_out.vals.push(value);
        left_and_out.edges.push(node);
        left_and_out.edges.push(right);
    }

    pub fn len(&self) -> usize {
        self.keys.len()
    }

    pub fn capacity(&self) -> usize {
        self.keys.capacity()
    }

    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    pub fn is_underfull(&self) -> bool {
        self.keys.len() < min_load_from_capacity(self.capacity())
    }

    pub unsafe fn unsafe_swap(&mut self, index: usize, key: &mut K, val: &mut V) {
        mem::swap(self.keys.get_unchecked_mut(index), key);
        mem::swap(self.vals.get_unchecked_mut(index), val);
    }

    pub fn key(&self, idx: usize) -> Option<&K> {
        self.keys.get(idx)
    }

    pub fn key_mut(&mut self, idx: usize) -> Option<&mut K> {
        self.keys.get_mut(idx)
    }

    pub unsafe fn unsafe_key_mut(&mut self, idx: usize) -> &mut K {
        self.keys.get_unchecked_mut(idx)
    }

    pub fn val(&self, idx: usize) -> Option<&V> {
        self.vals.get(idx)
    }

    pub fn val_mut(&mut self, idx: usize) -> Option<&mut V> {
        self.vals.get_mut(idx)
    }

    pub unsafe fn unsafe_val_mut(&mut self, idx: usize) -> &mut V {
        self.vals.get_unchecked_mut(idx)
    }

    pub fn edge(&self, idx: usize) -> Option<&Self> {
        self.edges.get(idx)
    }

    pub fn edge_mut(&mut self, idx: usize) -> Option<&mut Node<K, V>> {
        self.edges.get_mut(idx)
    }

    pub unsafe fn unsafe_edge_mut(&mut self, idx: usize) -> &mut Node<K, V> {
        self.edges.get_unchecked_mut(idx)
    }

    pub fn pop_edge(&mut self) -> Option<Node<K, V>> {
        self.edges.pop()
    }

    // If the node has any children
    pub fn is_leaf(&self) -> bool {
        self.edges.is_empty()
    }

    pub fn insert_as_leaf(
        &mut self,
        index: usize,
        key: K,
        value: V,
    ) -> (InsertionResult<K, V>, *mut V) {
        if !self.is_full() {
            self.insert_fit_as_leaf(index, key, value);
            (Fit, unsafe { self.unsafe_val_mut(index) as *mut _ })
        } else {
            // The new element can't fit, split
            let (new_key, new_val, mut new_right) = self.split();
            // now self is left
            let left_len = self.len();

            let ptr = if index <= left_len {
                self.insert_fit_as_leaf(index, key, value);
                unsafe { self.unsafe_val_mut(index) as *mut _ }
            } else {
                new_right.insert_fit_as_leaf(index - left_len - 1, key, value);
                unsafe { new_right.unsafe_val_mut(index - left_len - 1) as *mut _ }
            };
            (Split(new_key, new_val, new_right), ptr)
        }
    }

    pub fn insert_as_internal(
        &mut self,
        index: usize,
        key: K,
        value: V,
        right: Node<K, V>,
    ) -> InsertionResult<K, V> {
        if !self.is_full() {
            self.insert_fit_as_internal(index, key, value, right);
            Fit
        } else {
            // The new element can't fit.
            let (new_key, new_val, mut new_right) = self.split();
            let left_len = self.len();
            if index <= left_len {
                self.insert_fit_as_internal(index, key, value, right);
            } else {
                new_right.insert_fit_as_internal(index - left_len - 1, key, value, right);
            }
            Split(new_key, new_val, new_right)
        }
    }

    pub fn remove_as_leaf(&mut self, index: usize) -> (K, V) {
        (self.keys.remove(index), self.vals.remove(index))
    }

    pub fn handle_underflow(&mut self, underflowed_child_index: usize) {
        assert!(underflowed_child_index <= self.len());
        if underflowed_child_index > 0 {
            unsafe { self.handle_underflow_to_left(underflowed_child_index) };
        } else {
            unsafe { self.handle_underflow_to_right(underflowed_child_index) };
        }
    }
}

// private functions
impl<K, V> Node<K, V>
where
    K: Ord,
{
    fn search_linear(&self, key: &K) -> SearchResult {
        for (i, k) in self.keys.iter().enumerate() {
            match k.cmp(key) {
                Less => continue,
                Equal => return Found(i),
                Greater => return GoDown(i),
            };
        }
        GoDown(self.len())
    }

    fn search_binary(&self, key: &K) -> SearchResult {
        unimplemented!()
    }

    fn from_vecs(keys: Vec<K>, vals: Vec<V>, edges: Vec<Node<K, V>>) -> Node<K, V> {
        Node { keys, vals, edges }
    }

    fn insert_fit_as_leaf(&mut self, index: usize, key: K, val: V) {
        self.keys.insert(index, key);
        self.vals.insert(index, val);
    }

    fn insert_fit_as_internal(&mut self, index: usize, key: K, val: V, right: Node<K, V>) {
        self.keys.insert(index, key);
        self.vals.insert(index, val);
        self.edges.insert(index + 1, right);
    }

    // Node is full, so split it into two nodes, and yield the middle-most key-vale par
    fn split(&mut self) -> (K, V, Node<K, V>) {
        let r_keys = split(&mut self.keys);
        let r_vals = split(&mut self.vals);
        let r_edges = if self.edges.is_empty() {
            Vec::new()
        } else {
            split(&mut self.edges)
        };

        let right = Node::from_vecs(r_keys, r_vals, r_edges);
        let key = self.keys.pop().unwrap();
        let val = self.vals.pop().unwrap();

        (key, val, right)
    }

    // Right is underflowed, try to steal from left.
    // Merge if left is also underflowed.
    unsafe fn handle_underflow_to_left(&mut self, underflowed_child_index: usize) {
        let left_len = self.edges[underflowed_child_index - 1].len();
        if left_len > min_load_from_capacity(self.capacity()) {
            self.steal_to_left(underflowed_child_index);
        } else {
            self.merge_children(underflowed_child_index - 1);
        }
    }

    unsafe fn handle_underflow_to_right(&mut self, underflowed_child_index: usize) {
        let right_len = self.edges[underflowed_child_index + 1].len();
        if right_len > min_load_from_capacity(self.capacity()) {
            self.steal_to_right(underflowed_child_index);
        } else {
            self.merge_children(underflowed_child_index);
        }
    }

    /// Steal! Stealing is roughly analagous to a binary tree rotation.
    /// In this case, we're "rotating" right.
    unsafe fn steal_to_left(&mut self, underflowed_child_index: usize) {
        // Get the last kv pair from left
        let (mut key, mut val, edge) = {
            let left = self.unsafe_edge_mut(underflowed_child_index - 1);
            match (left.keys.pop(), left.vals.pop(), left.edges.pop()) {
                (Some(key), Some(val), edge) => (key, val, edge),
                _ => unreachable!(),
            }
        };

        // swap the parent's seperating kv pair node with left
        self.unsafe_swap(underflowed_child_index - 1, &mut key, &mut val);

        // put it to the begin of right node
        let right = self.unsafe_edge_mut(underflowed_child_index);
        right.keys.insert(0, key);
        right.vals.insert(0, val);
        if let Some(edge) = edge {
            right.edges.insert(0, edge);
        }
    }

    unsafe fn steal_to_right(&mut self, underflowed_child_index: usize) {
        // Get the first kv pair from right
        let (mut key, mut val, edge) = {
            let right = self.unsafe_edge_mut(underflowed_child_index + 1);
            if right.edges.is_empty() {
                (right.keys.remove(0), right.vals.remove(0), None)
            } else {
                (
                    right.keys.remove(0),
                    right.vals.remove(0),
                    Some(right.edges.remove(0)),
                )
            }
        };

        // swap the parent's seperating kv pair node.
        self.unsafe_swap(underflowed_child_index, &mut key, &mut val);

        // put it to the end of left node
        let left = self.unsafe_edge_mut(underflowed_child_index);
        left.keys.push(key);
        left.vals.push(val);
        if let Some(edge) = edge {
            left.edges.push(edge);
        }
    }

    unsafe fn merge_children(&mut self, left_index: usize) {
        let (key, val, right) = (
            self.keys.remove(left_index),
            self.vals.remove(left_index),
            self.edges.remove(left_index + 1),
        );
        let left = self.unsafe_edge_mut(left_index);
        left.absorb(key, val, right);
    }

    fn absorb(&mut self, key: K, val: V, right: Node<K, V>) {
        debug_assert!(self.len() + right.len() <= self.capacity());

        self.keys.push(key);
        self.vals.push(val);
        self.keys.extend(right.keys);
        self.vals.extend(right.vals);
        self.edges.extend(right.edges);
    }
}

// Takes a Vec, and splits half the elements into a new one.
fn split<T>(left: &mut Vec<T>) -> Vec<T> {
    let len = left.len();
    let right_len = len / 2;
    let left_len = len - right_len;
    let mut right = Vec::with_capacity(left.capacity());
    unsafe {
        let left_ptr = left.get_unchecked_mut(left_len) as *mut _;
        let right_ptr = right.as_mut_ptr();
        ptr::copy_nonoverlapping(left_ptr, right_ptr, right_len);
        left.set_len(left_len);
        right.set_len(right_len);
    }
    right
}

fn capacity_from_b(b: usize) -> usize {
    2 * b - 1
}

fn min_load_from_capacity(capacity: usize) -> usize {
    // b - 1
    capacity / 2
}

pub enum SearchResult {
    Found(usize),
    GoDown(usize),
}

pub enum InsertionResult<K, V> {
    Fit,
    Split(K, V, Node<K, V>),
}
