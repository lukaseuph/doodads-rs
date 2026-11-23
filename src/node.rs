use std::{
    cell::{Cell, Ref, RefCell},
    collections::{HashSet, VecDeque},
    fmt,
    hash::{Hash, Hasher},
    rc::{Rc, Weak},
};

/// A node in a directed graph, holding data of type `T`.
///
/// `Node` is a shared handle: cloning it produces another handle to the same
/// node, and two nodes are equal only if they are the same node.
///
/// Nodes are connected to each other with [`Edge`]s, which carry data of type `U`.
/// Each edge is recorded at both of its ends, so graphs can be walked forward
/// and backward. Self-loops are allowed; duplicate edges between the same pair
/// of nodes are not.
pub struct Node<T, U = ()> {
    node: Rc<NodeInner<T, U>>,
}
impl<T, U> Node<T, U> {
    /// Creates a new node holding `data`, with no edges.
    ///
    /// # Examples
    ///
    /// ```
    /// use doodads::node::Node;
    /// let a: Node<&str> = Node::new("a");
    /// assert!(a.outgoing().is_empty());
    /// assert!(a.incoming().is_empty());
    /// ```
    pub fn new(data: T) -> Self {
        let island = Rc::new(Island {
            nodes: RefCell::new(Vec::new()),
            external_refs: Cell::new(1), // the handle being returned
        });
        let node = Self {
            node: Rc::new(NodeInner {
                data: RefCell::new(NodeData {
                    data,
                    outgoing: Vec::new(),
                    incoming: Vec::new(),
                }),
                island: RefCell::new(island.clone()),
            }),
        };
        island.nodes.borrow_mut().push(Rc::downgrade(&node.node));
        node
    }

    fn to_link(&self) -> Link<T, U> {
        Link(Rc::clone(&self.node))
    }

    /// Returns a reference to the data stored in this node.
    ///
    /// The node stays borrowed for as long as the returned guard is alive.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed. That only happens
    /// inside [`connect`](Node::connect) and [`disconnect`](Node::disconnect),
    /// so it is not normally reachable from user code.
    pub fn data(&self) -> Ref<'_, T> {
        Ref::map(self.node.data.borrow(), |n| &n.data)
    }

    /// Returns the edges leaving this node, in the order they were connected.
    ///
    /// Each [`Edge::node`] is the edge's target. The node stays borrowed for as
    /// long as the returned guard is alive, so drop it (or copy out what you
    /// need) before calling [`connect`](Node::connect) or
    /// [`disconnect`](Node::disconnect) on this node, which would panic.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn outgoing(&self) -> Ref<'_, [Edge<T, U>]> {
        Ref::map(self.node.data.borrow(), |n| n.outgoing.as_slice())
    }

    /// Returns the edges entering this node, in the order they were connected.
    ///
    /// Each [`Edge::node`] is the edge's *source*, the node the edge came from.
    /// See [`outgoing`](Node::outgoing) for the borrowing rules.
    ///
    /// # Panics
    ///
    /// Panics if the node is currently mutably borrowed.
    pub fn incoming(&self) -> Ref<'_, [Edge<T, U>]> {
        Ref::map(self.node.data.borrow(), |n| n.incoming.as_slice())
    }

    /// Adds a directed edge from this node to `other`, carrying `edge_data`.
    ///
    /// Returns `true` if the edge was added, or `false` if this node already has
    /// an edge to `other`, in which case nothing changes and `edge_data` is
    /// dropped. Connecting a node to itself is allowed.
    ///
    /// The edge is recorded at both ends: it appears in this node's
    /// [`outgoing`](Node::outgoing) edges and in `other`'s
    /// [`incoming`](Node::incoming) edges, and both entries share the same
    /// `edge_data`. Edges are directed, so connecting `a` to `b` does not connect
    /// `b` to `a`.
    ///
    /// # Panics
    ///
    /// Panics if either node is currently borrowed, for example while a guard
    /// returned by [`outgoing`](Node::outgoing) is still alive.
    ///
    /// # Examples
    ///
    /// ```
    /// use doodads::node::Node;
    ///
    /// let a: Node<&str> = Node::new("a");
    /// let b = Node::new("b");
    /// assert!(a.connect(&b, ()));
    /// assert!(!a.connect(&b, ())); // duplicate
    /// assert!(b.connect(&a, ()));  // opposite direction is a different edge
    /// ```
    pub fn connect(&self, other: &Node<T, U>, edge_data: U) -> bool {
        // Reject duplicates first, so a failed connect never merges islands.
        if self
            .node
            .data
            .borrow()
            .outgoing
            .iter()
            .any(|edge| edge.points_to(other))
        {
            return false;
        }

        // Merge before taking any `borrow_mut`; clone the `Rc`s out so no guard
        // is alive while `merge` repoints the nodes' island cells.
        let a = Rc::clone(&self.node.island.borrow());
        let b = Rc::clone(&other.node.island.borrow());
        Island::merge(&a, &b);

        let edge_data = Rc::new(edge_data);
        // Build both edges before taking any `borrow_mut`.
        let outgoing = Edge::new(other, Rc::clone(&edge_data));
        let incoming = Edge::new(self, edge_data);

        // Separate statements, so each `borrow_mut` is released before the next.
        self.node.data.borrow_mut().outgoing.push(outgoing);
        other.node.data.borrow_mut().incoming.push(incoming);
        true
    }

    /// Removes the edge from this node to `other`, at both ends.
    ///
    /// Returns `true` if the edge existed and was removed, or `false` if this
    /// node has no edge to `other`, in which case nothing changes. Edges are
    /// directed, so disconnecting `a` from `b` leaves any edge from `b` to `a`
    /// in place. Disconnecting a self-loop works the same way, with `other` being
    /// the node itself.
    ///
    /// Removing the edge releases the references the two nodes held to each
    /// other. A node that is connected to itself holds a reference to itself, so
    /// it is never freed until that self-loop is disconnected.
    ///
    /// The order of the remaining edges is preserved.
    ///
    /// # Panics
    ///
    /// Panics if either node is currently borrowed, for example while a guard
    /// returned by [`outgoing`](Node::outgoing) or [`incoming`](Node::incoming)
    /// is still alive.
    ///
    /// # Examples
    ///
    /// ```
    /// use doodads::node::Node;
    ///
    /// let a: Node<&str> = Node::new("a");
    /// let b = Node::new("b");
    /// a.connect(&b, ());
    /// assert!(a.disconnect(&b));
    /// assert!(!a.disconnect(&b)); // already gone
    /// ```
    pub fn disconnect(&self, other: &Node<T, U>) -> bool {
        let mut self_borrow = self.node.data.borrow_mut();
        let Some(self_index) = self_borrow
            .outgoing
            .iter()
            .position(|edge| edge.node() == *other)
        else {
            return false;
        };
        self_borrow.outgoing.remove(self_index);
        drop(self_borrow);

        let mut other_borrow = other.node.data.borrow_mut();
        // Unwrapping here will never panic.
        // Node::connect guarantees that if A has an outgoing connection to B,
        // B will always havae an outgoing connection to A.
        let other_index = other_borrow
            .incoming
            .iter()
            .position(|edge| edge.node() == *self)
            .unwrap();
        other_borrow.incoming.remove(other_index);
        true
    }

    pub fn orphan(&self) {
        for edge in self.outgoing().as_ref() {
            self.disconnect(&edge.node());
        }

        for edge in self.incoming().as_ref() {
            edge.node().disconnect(self);
        }
    }

    /// Walks every path starting from this node, calling `f` at each node visited.
    ///
    /// `direction` chooses which edges to follow: [`Direction::Forward`] follows
    /// [`outgoing`](Node::outgoing) edges, [`Direction::Backward`] follows
    /// [`incoming`](Node::incoming) edges.
    ///
    /// `f` receives the accumulator, the node being visited, and the edge that
    /// was just crossed to reach it (`None` for the starting node). It returns
    /// the accumulator to pass on to the neighbors. Each branch gets its own
    /// clone of the accumulator, so values flow down a path and never sideways
    /// between branches. The walk is depth-first, following edges in the order
    /// they were connected.
    ///
    /// A node is never visited twice on the same path, so cycles and self-loops
    /// terminate: when a path reaches a node it has already visited, that branch
    /// stops. Nodes reachable by several different paths, such as the bottom of a
    /// diamond, are visited once per path. This can be exponential in the size of
    /// the graph; use [`find`](Node::find) if you only need to visit each node once.
    ///
    /// No borrow is held while `f` runs, so the callback may freely call
    /// [`connect`](Node::connect) and [`disconnect`](Node::disconnect). Edges
    /// added during the walk may or may not be followed, because each node's
    /// edge list is copied when the walk arrives at it.
    ///
    /// # Examples
    ///
    /// ```
    /// use doodads::node::{Direction, Node};
    ///
    /// let a: Node<&str> = Node::new("a");
    /// let b = Node::new("b");
    /// let c = Node::new("c");
    /// a.connect(&b, ());
    /// b.connect(&c, ());
    ///
    /// // Record each node's depth.
    /// let mut depths = Vec::new();
    /// a.walk(Direction::Forward, 0, |depth, node, _| {
    ///     depths.push((*node.data(), depth));
    ///     depth + 1
    /// });
    /// assert_eq!(depths, [("a", 0), ("b", 1), ("c", 2)]);
    /// ```
    pub fn walk<A: Clone, F: FnMut(A, &Node<T, U>, Option<&Edge<T, U>>) -> A>(
        &self,
        direction: Direction,
        acc: A,
        mut f: F,
    ) {
        let mut path = Vec::new();
        self.walk_inner(direction, acc, None, &mut f, &mut path);
    }

    /// The inner logic of [`Node::walk`].
    fn walk_inner<A: Clone, F: FnMut(A, &Node<T, U>, Option<&Edge<T, U>>) -> A>(
        &self,
        direction: Direction,
        acc: A,
        previous_edge: Option<&Edge<T, U>>,
        f: &mut F,
        path: &mut Vec<Node<T, U>>,
    ) {
        // Recursion guard. A path will never reach the same node twice.
        if path.contains(self) {
            return;
        }
        path.push(self.clone());

        let result = f(acc, self, previous_edge);
        let edges = direction.edges(self).to_vec();
        for edge in edges {
            edge.node()
                .walk_inner(direction, result.clone(), Some(&edge), f, path)
        }

        // When a branch ends, it recursively unwinds the stack.
        path.pop();
    }

    /// Returns the first node, starting with this one, for which `pred` returns `true`.
    ///
    /// `direction` chooses which edges to follow: [`Direction::Forward`] follows
    /// [`outgoing`](Node::outgoing) edges, [`Direction::Backward`] follows
    /// [`incoming`](Node::incoming) edges. `order` chooses the traversal:
    /// [`Traversal::BreadthFirst`] finds the match closest to this node (fewest
    /// edges away), while [`Traversal::DepthFirst`] dives down one branch before
    /// trying others, so it may find a different match when several exist.
    ///
    /// Each node is visited at most once, so cycles and diamonds are handled in
    /// time linear in the size of the graph. Returns `None` if no reachable node
    /// matches. This node is checked first, so a match on the starting node is
    /// returned immediately.
    ///
    /// No borrow is held while `pred` runs, so it may call
    /// [`connect`](Node::connect) and [`disconnect`](Node::disconnect). The
    /// search does not recurse, so deep graphs cannot overflow the stack. Use
    /// [`walk`](Node::walk) if you need to visit every path rather than every node.
    ///
    /// # Panics
    ///
    /// Panics if `pred` panics.
    ///
    /// # Examples
    ///
    /// ```
    /// use doodads::node::{Direction, Node, Traversal};
    ///
    /// let a: Node<&str> = Node::new("a");
    /// let b = Node::new("b");
    /// let c = Node::new("c");
    /// a.connect(&b, ());
    /// b.connect(&c, ());
    ///
    /// let found = a.find(Direction::Forward, Traversal::BreadthFirst, |n| *n.data() == "c");
    /// assert!(found == Some(c.clone()));
    /// assert!(a.find(Direction::Backward, Traversal::BreadthFirst, |n| *n.data() == "c").is_none());
    /// ```
    pub fn find(
        &self,
        direction: Direction,
        order: Traversal,
        mut pred: impl FnMut(&Node<T, U>) -> bool,
    ) -> Option<Node<T, U>> {
        let mut visited = HashSet::new();
        let mut frontier = VecDeque::new();
        visited.insert(self.clone());
        frontier.push_back(self.clone());

        loop {
            let node = match order {
                Traversal::BreadthFirst => frontier.pop_front(),
                Traversal::DepthFirst => frontier.pop_back(),
            }?;
            if pred(&node) {
                return Some(node);
            }
            for edge in direction.edges(&node).iter() {
                let next = edge.node();
                if visited.insert(next.clone()) {
                    frontier.push_back(next.clone());
                }
            }
        }
    }
}
impl<T, U> Clone for Node<T, U> {
    fn clone(&self) -> Self {
        self.node.island.borrow().acquire();
        Self {
            node: self.node.clone(),
        }
    }
}
impl<T, U> Drop for Node<T, U> {
    fn drop(&mut self) {
        // Split statements so that the Ref guards have been dropped before island is released.
        let island = Rc::clone(&self.node.island.borrow());
        island.release();
    }
}
impl<T, U> PartialEq for Node<T, U> {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.node, &other.node)
    }
}
impl<T, U> Eq for Node<T, U> {}
impl<T, U> Hash for Node<T, U> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.node).hash(state);
    }
}
impl<T: fmt::Debug, U> fmt::Debug for Node<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.node.data.try_borrow() {
            Ok(n) => f
                .debug_struct("Node")
                .field("id", &Rc::as_ptr(&self.node))
                .field("data", &n.data)
                .field(
                    "outgoing",
                    &n.outgoing
                        .iter()
                        .map(|x| Rc::as_ptr(&x.link.to_node().node))
                        .collect::<Vec<_>>(),
                )
                .field(
                    "incoming",
                    &n.incoming
                        .iter()
                        .map(|x| Rc::as_ptr(&x.link.to_node().node))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            Err(_) => f.write_str("Node { <mutably borrowed> }"),
        }
    }
}

struct Link<T, U>(Rc<NodeInner<T, U>>);
impl<T, U> Link<T, U> {
    fn to_node(&self) -> Node<T, U> {
        self.0.island.borrow().acquire();
        Node {
            node: Rc::clone(&self.0),
        }
    }
}
impl<T, U> Clone for Link<T, U> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
impl<T, U> fmt::Debug for Link<T, U> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Link").field(&Rc::as_ptr(&self.0)).finish()
    }
}

/// A directed connection between two [`Node`]s, carrying data of type `U`.
///
/// Edges are created by [`Node::connect`] and removed by [`Node::disconnect`]. You get them from
/// [`Node::outgoing`], [`Node::incoming`], and the callback passed to [`Node::walk`].
///
/// Each edge is stored at both of its ends, and the two entries share the same data. In a node's
/// outgoing list, [`Edge::node`] is the edge's target; in its incoming list, it is the edge's
/// source. Cloning an `Edge` produces another handle to the same connection, not a new one.
#[derive(Debug)]
pub struct Edge<T, U = ()> {
    link: Link<T, U>,
    data: Rc<U>,
}
impl<T, U> Edge<T, U> {
    fn new(node: &Node<T, U>, data: Rc<U>) -> Self {
        Self {
            link: node.to_link(),
            data,
        }
    }

    /// Returns the node at the other end of this edge.
    ///
    /// This is the target when the edge came from [`Node::outgoing`], and the source when it came
    /// from [`Node::incoming`].
    pub fn node(&self) -> Node<T, U> {
        self.link.to_node()
    }

    /// Returns the data carried by this edge.
    ///
    /// Both ends of the edge share this same allocation, so [`Rc::ptr_eq`] on two edges' data tells
    /// you whether they are the two ends of one connection.
    pub fn data(&self) -> &U {
        &self.data
    }

    fn points_to(&self, node: &Node<T, U>) -> bool {
        Rc::ptr_eq(&self.link.0, &node.node)
    }
}
impl<T, U> Clone for Edge<T, U> {
    fn clone(&self) -> Self {
        Self {
            link: self.link.clone(),
            data: self.data.clone(),
        }
    }
}

#[derive(Debug)]
struct NodeInner<T, U> {
    data: RefCell<NodeData<T, U>>,
    island: RefCell<Rc<Island<T, U>>>,
}

/// The shared state behind a [`Node`]: its data and its edges.
///
/// This type is private so that all edges are created and removed through
/// [`Node::connect`] and [`Node::disconnect`], which keep every edge recorded at
/// both of its ends. `Node::disconnect` relies on that invariant.
#[derive(Debug)]
struct NodeData<T, U> {
    data: T,
    outgoing: Vec<Edge<T, U>>,
    incoming: Vec<Edge<T, U>>,
}

#[derive(Debug)]
struct Island<T, U> {
    nodes: RefCell<Vec<Weak<NodeInner<T, U>>>>,
    external_refs: Cell<usize>,
}
impl<T, U> Island<T, U> {
    // Merge before taking any borrows in Node::connect to avoid ownership panic.
    fn merge(this: &Rc<Self>, other: &Rc<Self>) -> Rc<Self> {
        // If both pointers are the same
        if Rc::ptr_eq(this, other) {
            return Rc::clone(this);
        }
        // Pick bigger and smaller of the two Islands
        let (big, small) = if this.nodes.borrow().len() >= other.nodes.borrow().len() {
            (this, other)
        } else {
            (other, this)
        };
        // Update small island nodes to use the big Island
        for node in small.nodes.borrow().as_slice() {
            if let Some(rc) = node.upgrade() {
                *rc.island.borrow_mut() = Rc::clone(big);
            }
        }
        // Flush small nodes into big Island
        let moved = small.nodes.take();
        big.nodes
            .borrow_mut()
            .extend(moved.into_iter().filter(|w| w.strong_count() > 0));
        // Add small external references to big Island
        big.external_refs
            .set(big.external_refs.get() + small.external_refs.get());
        // Reset small island counter
        small.external_refs.set(0);
        Rc::clone(big)
    }

    fn acquire(&self) {
        self.external_refs.set(self.external_refs.get() + 1);
    }

    fn release(&self) {
        let n = self.external_refs.get();
        debug_assert!(n > 0, "release() with external_refs == 0");
        let n = self.external_refs.get() - 1;
        self.external_refs.set(n);
        if n == 0 {
            self.purge_nodes();
        }
    }

    fn purge_nodes(&self) {
        // Take the list so no borrow on `self.nodes` is held while dropping.
        let nodes = self.nodes.take();
        for weak in nodes {
            if let Some(rc) = weak.upgrade() {
                // Move the edges out; the guard is released at the end of this block.
                let (outgoing, incoming) = {
                    let mut borrowed = rc.data.borrow_mut();
                    (
                        std::mem::take(&mut borrowed.outgoing),
                        std::mem::take(&mut borrowed.incoming),
                    )
                };
                // Edges drop here, with no borrow on any node held by this function.
                drop(outgoing);
                drop(incoming);
            }
        }
    }
}

/// Which edges to follow when moving between nodes.
///
/// [`Direction::Forward`] follows the outgoing edges of a [`Node`], while
/// [`Direction::Backward`] follows the incoming edges. Used by [`Node::walk`]
/// and [`Node::find`].
#[derive(Clone, Copy)]
pub enum Direction {
    Forward,
    Backward,
}
impl Direction {
    fn edges<'a, T, U>(&self, node: &'a Node<T, U>) -> Ref<'a, [Edge<T, U>]> {
        match self {
            Direction::Forward => node.outgoing(),
            Direction::Backward => node.incoming(),
        }
    }
}

/// The order in which [`Node::find`] visits nodes.
///
/// Both orders visit each node at most once, so they differ only in which
/// match is returned when several nodes satisfy the predicate.
#[derive(Clone, Copy)]
pub enum Traversal {
    /// Visit nodes in order of distance from the starting node, so the match
    /// with the fewest edges between it and the start is found first.
    BreadthFirst,
    /// Follow one branch as deep as it goes before trying others, so the
    /// match found may be farther away than the closest one.
    DepthFirst,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::rc::Weak;

    // Shared helpers (visible to every submodule via `use super::*`)

    type N = Node<&'static str>;

    fn node(name: &'static str) -> N {
        Node::new(name)
    }

    fn diamond() -> (N, N, N, N) {
        //  a -> b -> d
        //  a -> c -> d
        let (a, b, c, d) = (node("a"), node("b"), node("c"), node("d"));
        a.connect(&b, ());
        a.connect(&c, ());
        b.connect(&d, ());
        c.connect(&d, ());
        (a, b, c, d)
    }

    fn island_of(n: &N) -> Rc<Island<&'static str, ()>> {
        Rc::clone(&n.node.island.borrow())
    }

    fn weak(n: &N) -> Weak<NodeInner<&'static str, ()>> {
        Rc::downgrade(&n.node)
    }

    fn refs(n: &N) -> usize {
        island_of(n).external_refs.get()
    }

    mod connect_disconnect {
        use super::*;

        /// Every outgoing edge must have a mirror incoming edge sharing the same
        /// data allocation, and vice versa.
        fn assert_mirrored(nodes: &[&N]) {
            for a in nodes {
                for e in a.outgoing().iter() {
                    assert!(
                        e.node()
                            .incoming()
                            .iter()
                            .any(|i| i.points_to(a) && Rc::ptr_eq(&i.data, &e.data)),
                        "outgoing edge has no mirror incoming edge"
                    );
                }
                for e in a.incoming().iter() {
                    assert!(
                        e.node()
                            .outgoing()
                            .iter()
                            .any(|o| o.points_to(a) && Rc::ptr_eq(&o.data, &e.data)),
                        "incoming edge has no mirror outgoing edge"
                    );
                }
            }
        }

        #[test]
        fn connect_mirrors_edge() {
            let (a, b) = (node("a"), node("b"));
            assert!(a.connect(&b, ()));
            assert_eq!(a.outgoing().len(), 1);
            assert_eq!(b.incoming().len(), 1);
            assert!(a.incoming().is_empty());
            assert!(b.outgoing().is_empty());
            assert_mirrored(&[&a, &b]);
        }

        #[test]
        fn connect_rejects_duplicates() {
            let (a, b) = (node("a"), node("b"));
            assert!(a.connect(&b, ()));
            assert!(!a.connect(&b, ()));
            assert_eq!(a.outgoing().len(), 1);
            assert_eq!(b.incoming().len(), 1);
        }

        #[test]
        fn opposite_direction_is_a_different_edge() {
            let (a, b) = (node("a"), node("b"));
            assert!(a.connect(&b, ()));
            assert!(b.connect(&a, ()));
            assert_mirrored(&[&a, &b]);
        }

        #[test]
        fn disconnect_removes_both_ends() {
            let (a, b) = (node("a"), node("b"));
            a.connect(&b, ());
            assert!(a.disconnect(&b));
            assert!(a.outgoing().is_empty());
            assert!(b.incoming().is_empty());
        }

        #[test]
        fn disconnect_missing_edge_returns_false() {
            let (a, b) = (node("a"), node("b"));
            assert!(!a.disconnect(&b));
            a.connect(&b, ());
            // Edges are directed: b -> a was never made.
            assert!(!b.disconnect(&a));
            assert_eq!(a.outgoing().len(), 1);
        }

        #[test]
        fn self_loop_connect_and_disconnect() {
            let a = node("a");
            assert!(a.connect(&a, ()));
            assert!(!a.connect(&a, ()));
            assert_eq!(a.outgoing().len(), 1);
            assert_eq!(a.incoming().len(), 1);
            assert_mirrored(&[&a]);

            assert!(a.disconnect(&a));
            assert!(a.outgoing().is_empty());
            assert!(a.incoming().is_empty());
        }

        #[test]
        fn disconnect_releases_references() {
            let (a, b) = (node("a"), node("b"));
            a.connect(&b, ());
            assert_eq!(Rc::strong_count(&b.node), 2); // b + a.outgoing
            a.disconnect(&b);
            assert_eq!(Rc::strong_count(&a.node), 1);
            assert_eq!(Rc::strong_count(&b.node), 1);

            // Self-loop: the node holds itself twice, so it leaks unless disconnected.
            a.connect(&a, ());
            assert_eq!(Rc::strong_count(&a.node), 3);
            a.disconnect(&a);
            assert_eq!(Rc::strong_count(&a.node), 1);
        }

        #[test]
        fn edge_data_is_shared_between_ends() {
            let (a, b) = (Node::new("a"), Node::new("b"));
            a.connect(&b, "label");
            assert_eq!(*a.outgoing()[0].data(), "label");
            assert!(Rc::ptr_eq(&a.outgoing()[0].data, &b.incoming()[0].data));
        }
    }

    mod walk {
        use super::*;

        fn walk_names(start: &N, dir: Direction) -> Vec<&'static str> {
            let mut out = Vec::new();
            start.walk(dir, (), |_, n, _| {
                out.push(*n.data());
            });
            out
        }

        #[test]
        fn chain_forward_and_backward() {
            let (a, b, c, d) = (node("a"), node("b"), node("c"), node("d"));
            a.connect(&b, ());
            b.connect(&c, ());
            c.connect(&d, ());
            assert_eq!(walk_names(&a, Direction::Forward), ["a", "b", "c", "d"]);
            assert_eq!(walk_names(&d, Direction::Backward), ["d", "c", "b", "a"]);
            assert_eq!(walk_names(&d, Direction::Forward), ["d"]);
        }

        #[test]
        fn threads_accumulator_along_paths() {
            let (a, b, c, d) = (node("a"), node("b"), node("c"), node("d"));
            a.connect(&b, ());
            a.connect(&c, ());
            c.connect(&d, ());
            let mut seen = Vec::new();
            a.walk(Direction::Forward, 0, |depth, n, _| {
                seen.push((*n.data(), depth));
                depth + 1
            });
            assert_eq!(seen, [("a", 0), ("b", 1), ("c", 1), ("d", 2)]);
        }

        #[test]
        fn passes_the_edge_just_crossed() {
            let (a, b, c) = (Node::new("a"), Node::new("b"), Node::new("c"));
            a.connect(&b, "x");
            b.connect(&c, "y");
            let mut via = Vec::new();
            a.walk(Direction::Forward, (), |_, _, e| {
                via.push(e.map(|e| *e.data()));
            });
            assert_eq!(via, [None, Some("x"), Some("y")]);
        }

        #[test]
        fn self_loop_terminates() {
            let a = node("a");
            a.connect(&a, ());
            assert_eq!(walk_names(&a, Direction::Forward), ["a"]);
            assert_eq!(walk_names(&a, Direction::Backward), ["a"]);
        }

        #[test]
        fn two_node_cycle_terminates() {
            let (a, b) = (node("a"), node("b"));
            a.connect(&b, ());
            b.connect(&a, ());
            assert_eq!(walk_names(&a, Direction::Forward), ["a", "b"]);
        }

        #[test]
        fn ring_terminates() {
            let (a, b, c) = (node("a"), node("b"), node("c"));
            a.connect(&b, ());
            b.connect(&c, ());
            c.connect(&a, ());
            assert_eq!(walk_names(&a, Direction::Forward), ["a", "b", "c"]);
            assert_eq!(walk_names(&a, Direction::Backward), ["a", "c", "b"]);
        }

        #[test]
        fn diamond_visits_shared_node_once_per_path() {
            let (a, _b, _c, d) = diamond();
            assert_eq!(
                walk_names(&a, Direction::Forward),
                ["a", "b", "d", "c", "d"]
            );
            assert_eq!(
                walk_names(&d, Direction::Backward),
                ["d", "b", "a", "c", "a"]
            );
        }

        #[test]
        fn callback_may_mutate_the_graph() {
            let (a, b, x) = (node("a"), node("b"), node("x"));
            a.connect(&b, ());
            let mut out = Vec::new();
            a.walk(Direction::Forward, (), |_, n, _| {
                out.push(*n.data());
                if *n.data() == "a" {
                    // Would panic if a borrow were held across the callback.
                    n.connect(&x, ());
                }
            });
            assert_eq!(out, ["a", "b", "x"]);
        }
    }

    mod find {
        use super::*;

        #[test]
        fn checks_the_start_node() {
            let a = node("a");
            let found = a.find(Direction::Forward, Traversal::BreadthFirst, |n| {
                *n.data() == "a"
            });
            assert!(found == Some(a.clone()));
        }

        #[test]
        fn returns_none_when_nothing_matches() {
            let (a, ..) = diamond();
            let found = a.find(Direction::Forward, Traversal::BreadthFirst, |n| {
                *n.data() == "z"
            });
            assert!(found.is_none());
        }

        #[test]
        fn respects_direction() {
            let (a, _b, _c, d) = diamond();
            assert!(
                d.find(Direction::Forward, Traversal::BreadthFirst, |n| *n.data()
                    == "a")
                    .is_none()
            );
            assert!(
                d.find(Direction::Backward, Traversal::BreadthFirst, |n| *n.data()
                    == "a")
                    .is_some()
            );
            assert!(
                a.find(Direction::Forward, Traversal::DepthFirst, |n| *n.data()
                    == "d")
                    .is_some()
            );
        }

        #[test]
        fn bfs_and_dfs_pick_different_matches() {
            // a -> c, a -> b -> d. Matching "c" or "d":
            // BFS reaches c (depth 1) first; DFS dives into b's branch and reaches d.
            let (a, b, c, d) = (node("a"), node("b"), node("c"), node("d"));
            a.connect(&c, ());
            a.connect(&b, ());
            b.connect(&d, ());
            let pred = |n: &N| matches!(*n.data(), "c" | "d");
            let bfs = a.find(Direction::Forward, Traversal::BreadthFirst, pred);
            let dfs = a.find(Direction::Forward, Traversal::DepthFirst, pred);
            assert_eq!(bfs.map(|n| *n.data()), Some("c"));
            assert_eq!(dfs.map(|n| *n.data()), Some("d"));
        }

        #[test]
        fn visits_each_node_once_even_in_a_diamond_or_cycle() {
            let (a, _b, _c, d) = diamond();
            d.connect(&a, ()); // close the loop
            for order in [Traversal::BreadthFirst, Traversal::DepthFirst] {
                let mut calls = 0;
                let found = a.find(Direction::Forward, order, |_| {
                    calls += 1;
                    false
                });
                assert!(found.is_none());
                assert_eq!(calls, 4);
            }
        }
    }

    mod traits {
        use super::*;

        #[test]
        fn node_equality_is_identity() {
            let (a1, a2) = (node("a"), node("a"));
            assert!(a1 == a1.clone());
            assert!(a1 != a2); // same data, different node
            let set: HashSet<N> = [a1.clone(), a1.clone(), a2].into_iter().collect();
            assert_eq!(set.len(), 2);
        }

        #[test]
        fn debug_on_cyclic_graph_does_not_recurse() {
            let (a, b) = (node("a"), node("b"));
            a.connect(&b, ());
            b.connect(&a, ());
            a.connect(&a, ());
            let s = format!("{a:?}");
            assert!(s.contains("Node"));
            let _ = format!("{:?}", a.outgoing());
        }

        #[test]
        fn debug_while_mutably_borrowed_does_not_panic() {
            let a = node("a");
            let _guard = a.node.data.borrow_mut();
            assert!(format!("{a:?}").contains("mutably borrowed"));
        }
    }

    mod island {
        use super::*;

        #[test]
        fn new_node_has_its_own_island_with_one_ref() {
            let (a, b) = (node("a"), node("b"));
            assert_eq!(refs(&a), 1);
            assert_eq!(refs(&b), 1);
            assert!(!Rc::ptr_eq(&island_of(&a), &island_of(&b)));
            assert_eq!(island_of(&a).nodes.borrow().len(), 1);
        }

        #[test]
        fn clone_and_drop_adjust_the_counter() {
            let a = node("a");
            let a2 = a.clone();
            assert_eq!(refs(&a), 2);
            let a3 = a2.clone();
            assert_eq!(refs(&a), 3);
            drop(a2);
            drop(a3);
            assert_eq!(refs(&a), 1);
        }

        #[test]
        fn edge_accessors_are_counted_handles() {
            let (a, b) = (node("a"), node("b"));
            a.connect(&b, ());
            let before = refs(&a);
            {
                let out = a.outgoing();
                let held = out[0].node();
                assert_eq!(refs(&a), before + 1);
                drop(held);
            }
            assert_eq!(refs(&a), before);
        }

        #[test]
        fn connect_merges_islands_and_sums_counters() {
            let (a, b) = (node("a"), node("b"));
            a.connect(&b, ());
            assert!(Rc::ptr_eq(&island_of(&a), &island_of(&b)));
            assert_eq!(refs(&a), 2);
            assert_eq!(island_of(&a).nodes.borrow().len(), 2);
        }

        #[test]
        fn rejected_connect_does_not_merge() {
            let (a, b, c) = (node("a"), node("b"), node("c"));
            a.connect(&b, ());
            assert!(!a.connect(&b, ())); // duplicate
            assert_eq!(refs(&a), 2);
            assert!(!Rc::ptr_eq(&island_of(&a), &island_of(&c)));
        }

        #[test]
        fn connect_within_one_island_leaves_counter_alone() {
            let (a, b, c) = (node("a"), node("b"), node("c"));
            a.connect(&b, ());
            b.connect(&c, ());
            assert_eq!(refs(&a), 3);
            c.connect(&a, ()); // already the same island
            assert_eq!(refs(&a), 3);
            assert_eq!(island_of(&a).nodes.borrow().len(), 3);
        }

        #[test]
        fn self_connect_does_not_change_counter() {
            let a = node("a");
            a.connect(&a, ());
            assert_eq!(refs(&a), 1);
            assert_eq!(island_of(&a).nodes.borrow().len(), 1);
        }

        #[test]
        fn chained_merges_end_in_one_island() {
            let (a, b, c, d) = (node("a"), node("b"), node("c"), node("d"));
            a.connect(&b, ());
            c.connect(&d, ());
            assert!(!Rc::ptr_eq(&island_of(&a), &island_of(&c)));
            b.connect(&c, ());

            let shared = island_of(&a);
            for n in [&b, &c, &d] {
                assert!(Rc::ptr_eq(&shared, &island_of(n)));
            }
            assert_eq!(shared.external_refs.get(), 4);
            assert_eq!(shared.nodes.borrow().len(), 4);
        }

        #[test]
        fn registry_entries_all_point_at_their_island() {
            let (a, b, c, d) = (node("a"), node("b"), node("c"), node("d"));
            a.connect(&b, ());
            c.connect(&d, ());
            d.connect(&a, ());
            let shared = island_of(&a);
            for w in shared.nodes.borrow().iter() {
                let rc = w.upgrade().expect("registered node should be alive");
                assert!(Rc::ptr_eq(&rc.island.borrow(), &shared));
            }
        }
    }

    mod freeing {
        use super::*;

        #[test]
        fn isolated_node_is_freed_on_drop() {
            let a = node("a");
            let w = weak(&a);
            drop(a);
            assert!(w.upgrade().is_none());
        }

        #[test]
        fn connected_pair_is_freed_once_all_handles_drop() {
            let (a, b) = (node("a"), node("b"));
            let (wa, wb) = (weak(&a), weak(&b));
            a.connect(&b, ());
            drop(a);
            drop(b);
            assert!(wa.upgrade().is_none());
            assert!(wb.upgrade().is_none());
        }

        #[test]
        fn nothing_is_freed_while_a_handle_remains() {
            let (a, b) = (node("a"), node("b"));
            let (wa, wb) = (weak(&a), weak(&b));
            a.connect(&b, ());
            drop(a);

            // `a` has no handle, but it is in b's island, so it stays alive
            // and the edge between them is intact.
            assert!(wa.upgrade().is_some());
            assert!(wb.upgrade().is_some());
            assert_eq!(b.incoming().len(), 1);
            assert_eq!(*b.incoming()[0].node().data(), "a");

            drop(b);
            assert!(wa.upgrade().is_none());
            assert!(wb.upgrade().is_none());
        }

        #[test]
        fn self_loop_is_freed_on_drop() {
            let a = node("a");
            let w = weak(&a);
            a.connect(&a, ());
            drop(a);
            assert!(w.upgrade().is_none());
        }

        #[test]
        fn ring_is_freed_on_drop() {
            let (a, b, c) = (node("a"), node("b"), node("c"));
            let ws = [weak(&a), weak(&b), weak(&c)];
            a.connect(&b, ());
            b.connect(&c, ());
            c.connect(&a, ());
            drop((a, b, c));
            assert!(ws.iter().all(|w| w.upgrade().is_none()));
        }

        #[test]
        fn merged_islands_are_freed_only_after_every_handle_drops() {
            // Try several drop orders of three handles.
            for order in [[0, 1, 2], [2, 1, 0], [1, 0, 2], [1, 2, 0]] {
                let (a, b, c) = (node("a"), node("b"), node("c"));
                let ws = [weak(&a), weak(&b), weak(&c)];
                a.connect(&b, ());
                c.connect(&b, ()); // two islands merge through b
                let mut handles = [Some(a), Some(b), Some(c)];

                for (i, &idx) in order.iter().enumerate() {
                    handles[idx] = None;
                    let all_gone = ws.iter().all(|w| w.upgrade().is_none());
                    assert_eq!(all_gone, i == 2, "order {order:?}, after drop {i}");
                }
            }
        }

        #[test]
        fn surviving_clone_keeps_the_graph_alive() {
            let (a, b) = (node("a"), node("b"));
            let (wa, wb) = (weak(&a), weak(&b));
            a.connect(&b, ());
            let a2 = a.clone();
            drop(a);
            drop(b);
            assert!(wa.upgrade().is_some());
            assert!(wb.upgrade().is_some());
            assert_eq!(a2.outgoing().len(), 1);
            drop(a2);
            assert!(wa.upgrade().is_none());
            assert!(wb.upgrade().is_none());
        }

        #[test]
        fn disconnect_does_not_split_the_island() {
            let (a, b) = (node("a"), node("b"));
            a.connect(&b, ());
            a.disconnect(&b);
            // Islands only grow: still shared, still counted together.
            assert!(Rc::ptr_eq(&island_of(&a), &island_of(&b)));
            assert_eq!(refs(&a), 2);

            let (wa, wb) = (weak(&a), weak(&b));
            drop(a);
            // `a` has no edges, so it is freed by its own refcount;
            // `b` survives because its handle is still alive.
            assert!(wa.upgrade().is_none());
            assert!(wb.upgrade().is_some());
            assert_eq!(refs(&b), 1);
        }

        #[test]
        fn purge_drops_user_data() {
            // The marker's strong count shows whether the node data was freed.
            let marker = Rc::new(());
            let a: Node<Rc<()>, ()> = Node::new(Rc::clone(&marker));
            let b = Node::new(Rc::clone(&marker));
            a.connect(&b, ());
            a.connect(&a, ());
            b.connect(&a, ());
            assert_eq!(Rc::strong_count(&marker), 3);
            drop(a);
            drop(b);
            assert_eq!(Rc::strong_count(&marker), 1);
        }
    }
}
