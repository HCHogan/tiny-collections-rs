use self::PushResult::*;
use super::super::node::{InsertionResult::*, SearchResult::*};
use super::{BTreeMap, Node};

type StackItem<K, V> = (*mut Node<K, V>, usize);
type Stack<K, V> = Vec<StackItem<K, V>>;

/// A partitialsearchstack handles the construction of a search stack.
pub struct PartialSearchStack<'a, K: 'a + Ord, V: 'a> {
    map: &'a mut BTreeMap<K, V>,
    stack: Stack<K, V>,
    next: *mut Node<K, V>,
}

/// A search stack represents a full path to an element of interest. It provides methods for manipulating the element at the top of its stack.
pub struct SearchStack<'a, K: 'a + Ord, V: 'a> {
    map: &'a mut BTreeMap<K, V>,
    stack: Stack<K, V>,
    top: StackItem<K, V>,
}

impl<'a, K, V> PartialSearchStack<'a, K, V>
where
    K: Ord,
{
    pub fn new(map: &mut BTreeMap<K, V>) -> PartialSearchStack<K, V> {
        let depth = map.depth;
        let next = &mut map.root as *mut _;

        PartialSearchStack {
            map,
            stack: Vec::with_capacity(depth),
            next,
        }
    }

    pub fn next(&self) -> &Node<K, V> {
        unsafe { &*self.next }
    }

    pub fn into_next(mut self) -> &'a mut Node<K, V> {
        unsafe { &mut *self.next }
    }

    // Transform self to SearchStack
    pub fn seal(self, index: usize) -> SearchStack<'a, K, V> {
        SearchStack {
            map: self.map,
            stack: self.stack,
            top: (self.next as *mut _, index),
        }
    }

    // Pushes the requested child of the stack's current top on top of the stack. If the child exists, then a new PartialSearchStack is yielded. Otherwise, a full SearchStack is yielded.
    pub fn push(self, edge: usize) -> PushResult<'a, K, V> {
        let map = self.map;
        let mut stack = self.stack;
        let next_ptr = self.next;
        let next_node = unsafe {
            &mut *next_ptr
        };
        let to_insert = (next_ptr, edge);
        match next_node.edge_mut(edge)  {
            None => Done(SearchStack {
                map,
                stack,
                top: to_insert,
            }),
            Some(node) => {
                stack.push(to_insert);
                Grew(PartialSearchStack {
                    map,
                    stack,
                    next: node as *mut _,
                })
            }
        }
    }
}

pub enum PushResult<'a, K: 'a + Ord, V: 'a> {
    Grew(PartialSearchStack<'a, K, V>),
    Done(SearchStack<'a, K, V>),
}

impl<'a, K, V> SearchStack<'a, K, V>
where
    K: Ord,
{
    pub fn peek(&self) -> &V {
        let (leaf_ptr, index) = self.top;
        unsafe { (*leaf_ptr).unsafe_val_mut(index) }
    }

    pub fn peek_mut(&mut self) -> &mut V {
        let (leaf_ptr, index) = self.top;
        unsafe { (*leaf_ptr).unsafe_val_mut(index) }
    }

    pub fn into_top(self) -> &'a mut V {
        let (leaf_ptr, index) = self.top;
        unsafe { (*leaf_ptr).unsafe_val_mut(index) }
    }

    pub fn insert(self, key: K, val: V) -> &'a mut V {
        let map = self.map;
        map.length += 1;

        let mut stack = self.stack;
        let (node_ptr, index) = self.top;
        let (mut insertion, inserted_ptr) = unsafe { (*node_ptr).insert_as_leaf(index, key, val) };

        loop {
            match insertion {
                Fit => unsafe {
                    return &mut *inserted_ptr;
                },
                Split(key, val, right) => match stack.pop() {
                    // The last insertion triggered a split, so get the next element on the stack to recursively insert the split node into.
                    None => {
                        // The stack was empty, we've split to the root node.
                        Node::make_internal_root(&mut map.root, map.b, key, val, right);
                        map.depth += 1;
                        return unsafe { &mut *inserted_ptr };
                    }
                    Some((node, index)) => {
                        insertion = unsafe { (*node).insert_as_internal(index, key, val, right) };
                    }
                },
            }
        }
    }

    // Remove 'top' and handle underflow
    pub fn remove(mut self) -> V {
        self.leafify();

        let mut stack = self.stack;
        let map = self.map;
        map.length -= 1;
        // remove the kv pair the SearchStack points to.
        let (value, mut underflow) = unsafe {
            let (leaf_ptr, index) = self.top;
            let leaf = &mut *leaf_ptr;
            let (_key, value) = leaf.remove_as_leaf(index);
            (value, leaf.is_underfull())
        };

        loop {
            match stack.pop() {
                None => {
                    // Now we reached the root.
                    if map.root.len() == 0 && !map.root.is_leaf() {
                        map.depth -= 1;
                        map.root = map.root.pop_edge().unwrap();
                    }
                    return value;
                }
                Some((parent_ptr, index)) => {
                    if underflow {
                        let parent = unsafe { &mut *parent_ptr };
                        parent.handle_underflow(index);
                        underflow = parent.is_underfull();
                    } else {
                        // All done!
                        return value;
                    }
                }
            }
        }
    }
}

impl<'a, K, V> SearchStack<'a, K, V>
where
    K: Ord,
{
    fn leafify(&mut self) {
        let (node_ptr, index) = self.top;
        let node = unsafe { &mut *node_ptr };
        let (key_ptr, val_ptr) = unsafe {
            (
                (*node_ptr).unsafe_key_mut(index),
                (*node_ptr).unsafe_val_mut(index),
            )
        };

        match node.edge_mut(index + 1) {
            Some(mut temp_node) => {
                // we're not at proper leaf node
                self.stack.push((node_ptr, index + 1));
                loop {
                    let node = temp_node;
                    let node_ptr = node as *mut _;
                    if node.is_leaf() {
                        self.top = (node_ptr, 0);
                        unsafe { node.unsafe_swap(0, &mut *key_ptr, &mut *val_ptr) };
                        break;
                    } else {
                        // This node is internal, go deeper.
                        self.stack.push((node_ptr, 0));
                        temp_node = unsafe { node.unsafe_edge_mut(0) };
                    }
                }
            }
            None => {
                // we're at a proper leaf node, nothing to do.
            }
        }
    }
}

