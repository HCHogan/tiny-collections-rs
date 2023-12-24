mod stack;

use super::node::{Node, SearchResult::*};
use stack::{PartialSearchStack, PushResult::*};
use std::mem;
// use std::collections::VecDeque;

pub struct BTreeMap<K: Ord, V> {
    root: Node<K, V>,
    length: usize,
    depth: usize,
    b: usize,
}

impl<K: Ord, V> Default for BTreeMap<K, V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Ord, V> BTreeMap<K, V> {
    /// Makes a new empty BTreeMap with a reasonable choice for B.
    pub fn new() -> BTreeMap<K, V> {
        BTreeMap::with_b(6)
    }

    /// Makes a new empty BTreeMap with the given B.
    pub fn with_b(b: usize) -> BTreeMap<K, V> {
        assert!(b > 1, "B must be greater than 1");
        BTreeMap {
            length: 0,
            depth: 1,
            root: Node::make_leaf_root(b),
            b,
        }
    }
    pub fn find(&self, key: &K) -> Option<&V> {
        let mut cur_node = &self.root;
        loop {
            match cur_node.search(key) {
                Found(i) => return cur_node.val(i),
                GoDown(i) => match cur_node.edge(i) {
                    None => return None,
                    Some(next_node) => {
                        cur_node = next_node;
                        continue;
                    }
                },
            }
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // Insertion in a B-Tree is a bit complicated.
        //
        // First we do the same kind of search described in `find`. But we need to maintain a stack of
        // all the nodes/edges in our search path. If we find a match for the key we're trying to
        // insert, just swap the vals and return the old ones. However, when we bottom out in a leaf,
        // we attempt to insert our key-value pair at the same location we would want to follow another
        // edge.
        //
        // If the node has room, then this is done in the obvious way by shifting elements. However,
        // if the node itself is full, we split node into two, and give its median key-value
        // pair to its parent to insert the new node with. Of course, the parent may also be
        // full, and insertion can propagate until we reach the root. If we reach the root, and
        // it is *also* full, then we split the root and place the two nodes under a newly made root.
        //
        // Note that we subtly deviate from Open Data Structures in our implementation of split.
        // ODS describes inserting into the node *regardless* of its capacity, and then
        // splitting *afterwards* if it happens to be overfull. However, this is inefficient.
        // Instead, we split beforehand, and then insert the key-value pair into the appropriate
        // result node. This has two consequences:
        //
        // 1) While ODS produces a left node of size B-1, and a right node of size B,
        // we may potentially reverse this. However, this shouldn't effect the analysis.
        //
        // 2) While ODS may potentially return the pair we *just* inserted after
        // the split, we will never do this. Again, this shouldn't effect the analysis.
        // let stack = VecDeque::new();
        let mut stack = PartialSearchStack::new(self);
        loop {
            match stack.next().search(&key) {
                Found(i) => unsafe {
                    let next = stack.into_next();
                    return Some(mem::replace(next.unsafe_val_mut(i), value));
                },
                GoDown(i) => {
                    stack = match stack.push(i) {
                        Done(new_stack) => {
                            new_stack.insert(key, value);
                            return None;
                        }
                        Grew(new_stack) => new_stack,
                    };
                }
            }
        }
    }

    // Deletion is the most complicated operation for a B-Tree.
    //
    // First we do the same kind of search described in
    // `find`. But we need to maintain a stack of all the nodes/edges in our search path.
    // If we don't find the key, then we just return `None` and do nothing. If we do find the
    // key, we perform two operations: remove the item, and then possibly handle underflow.
    //
    // # removing the item
    //      If the node is a leaf, we just remove the item, and shift
    //      any items after it back to fill the hole.
    //
    //      If the node is an internal node, we *swap* the item with the smallest item in
    //      in its right subtree (which must reside in a leaf), and then revert to the leaf
    //      case
    //
    // # handling underflow
    //      After removing an item, there may be too few items in the node. We want nodes
    //      to be mostly full for efficiency, although we make an exception for the root, which
    //      may have as few as one item. If this is the case, we may first try to steal
    //      an item from our left or right neighbour.
    //
    //      To steal from the left (right) neighbour,
    //      we take the largest (smallest) item and child from it. We then swap the taken item
    //      with the item in their mutual parent that separates them, and then insert the
    //      parent's item and the taken child into the first (last) index of the underflowed node.
    //
    //      However, stealing has the possibility of underflowing our neighbour. If this is the
    //      case, we instead *merge* with our neighbour. This of course reduces the number of
    //      children in the parent. Therefore, we also steal the item that separates the now
    //      merged nodes, and insert it into the merged node.
    //
    //      Merging may cause the parent to underflow. If this is the case, then we must repeat
    //      the underflow handling process on the parent. If merging merges the last two children
    //      of the root, then we replace the root with the merged node.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let mut stack = PartialSearchStack::new(self);
        loop {
            match stack.next().search(key) {
                Found(i) => {
                    // exact match
                    return Some(stack.seal(i).remove());
                },
                GoDown(i) => {
                    stack = match stack.push(i) {
                        Grew(new_stack) => {
                            new_stack
                        },
                        Done(_) => return None,
                    }
                }
            };
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn small_test1() {
        let mut bt = BTreeMap::new();
        (0..100).for_each(|i| {
            bt.insert(i, i);
        });

        (0..100).for_each(|i| {
            assert_eq!(Some(&i), bt.find(&i));
        });
    }
}

