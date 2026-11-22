use std::{
    cell::{Ref, RefCell},
    collections::{HashSet, VecDeque},
    fmt,
    hash::{Hash, Hasher},
    rc::Rc,
};

#[derive(Debug)]
struct NodeData<T, U> {
    data: T,
    outgoing: Vec<Edge<T, U>>,
    incoming: Vec<Edge<T, U>>,
}

pub struct Node<T, U = ()> {
    node: Rc<RefCell<NodeData<T, U>>>,
}
impl<T, U> Node<T, U> {
    pub fn new(data: T) -> Self {
        Self {
            node: Rc::new(RefCell::new(NodeData {
                data,
                outgoing: Vec::new(),
                incoming: Vec::new(),
            })),
        }
    }

    pub fn data(&self) -> Ref<'_, T> {
        Ref::map(self.node.borrow(), |n| &n.data)
    }

    pub fn outgoing(&self) -> Ref<'_, [Edge<T, U>]> {
        Ref::map(self.node.borrow(), |n| n.outgoing.as_slice())
    }

    pub fn incoming(&self) -> Ref<'_, [Edge<T, U>]> {
        Ref::map(self.node.borrow(), |n| n.incoming.as_slice())
    }

    pub fn connect(&self, other: &Node<T, U>, edge_data: U) -> bool {
        if self
            .node
            .borrow()
            .outgoing
            .iter()
            .any(|edge| edge.node() == other)
        {
            return false;
        }
        let edge_data = Rc::new(edge_data);
        self.node
            .borrow_mut()
            .outgoing
            .push(Edge::new(other.clone(), edge_data.clone()));
        other
            .node
            .borrow_mut()
            .incoming
            .push(Edge::new(self.clone(), edge_data));
        true
    }

    pub fn disconnect(&self, other: &Node<T, U>) -> bool {
        let mut self_borrow = self.node.borrow_mut();
        let Some(self_index) = self_borrow
            .outgoing
            .iter()
            .position(|edge| edge.node() == other)
        else {
            return false;
        };
        self_borrow.outgoing.remove(self_index);
        drop(self_borrow);

        let mut other_borrow = other.node.borrow_mut();
        // Unwrapping here will never panic.
        // Node::connect guarantees that if A has an outgoing connection to B,
        // B will always havae an outgoing connection to A.
        let other_index = other_borrow
            .incoming
            .iter()
            .position(|edge| edge.node() == self)
            .unwrap();
        other_borrow.incoming.remove(other_index);
        true
    }

    pub fn walk<A: Clone, F: FnMut(A, &Node<T, U>, Option<&Edge<T, U>>) -> A>(
        &self,
        direction: Direction,
        acc: A,
        mut f: F,
    ) {
        let mut path = Vec::new();
        self.walk_inner(direction, acc, None, &mut f, &mut path);
    }

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
        Self {
            node: self.node.clone(),
        }
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
        match self.node.try_borrow() {
            Ok(n) => f
                .debug_struct("Node")
                .field("id", &Rc::as_ptr(&self.node))
                .field("data", &n.data)
                .field(
                    "outgoing",
                    &n.outgoing
                        .iter()
                        .map(|x| Rc::as_ptr(&x.node.node))
                        .collect::<Vec<_>>(),
                )
                .field(
                    "incoming",
                    &n.incoming
                        .iter()
                        .map(|x| Rc::as_ptr(&x.node.node))
                        .collect::<Vec<_>>(),
                )
                .finish(),
            Err(_) => f.write_str("Node { <mutably borrowed> }"),
        }
    }
}

#[derive(Debug)]
pub struct Edge<T, U = ()> {
    node: Node<T, U>,
    data: Rc<U>,
}
impl<T, U> Edge<T, U> {
    fn new(node: Node<T, U>, data: Rc<U>) -> Self {
        Self { node, data }
    }

    pub fn node(&self) -> &Node<T, U> {
        &self.node
    }

    pub fn data(&self) -> &Rc<U> {
        &self.data
    }
}
impl<T, U> Clone for Edge<T, U> {
    fn clone(&self) -> Self {
        Self {
            node: self.node.clone(),
            data: self.data.clone(),
        }
    }
}

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

#[derive(Clone, Copy)]
pub enum Traversal {
    BreadthFirst,
    DepthFirst,
}

#[cfg(test)]
mod tests {
    use super::*;

    type N = Node<&'static str>;

    fn node(name: &'static str) -> N {
        Node::new(name)
    }

    fn walk_names(start: &N, dir: Direction) -> Vec<&'static str> {
        let mut out = Vec::new();
        start.walk(dir, (), |_, n, _| {
            out.push(*n.data());
        });
        out
    }

    /// Every outgoing edge must have a mirror incoming edge sharing the same
    /// data allocation, and vice versa.
    fn assert_mirrored(nodes: &[&N]) {
        for a in nodes {
            for e in a.outgoing().iter() {
                assert!(
                    e.node()
                        .incoming()
                        .iter()
                        .any(|i| i.node() == *a && Rc::ptr_eq(i.data(), e.data())),
                    "outgoing edge has no mirror incoming edge"
                );
            }
            for e in a.incoming().iter() {
                assert!(
                    e.node()
                        .outgoing()
                        .iter()
                        .any(|o| o.node() == *a && Rc::ptr_eq(o.data(), e.data())),
                    "incoming edge has no mirror outgoing edge"
                );
            }
        }
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
        assert_eq!(**a.outgoing()[0].data(), "label");
        assert!(Rc::ptr_eq(a.outgoing()[0].data(), b.incoming()[0].data()));
    }

    #[test]
    fn walk_chain_forward_and_backward() {
        let (a, b, c, d) = (node("a"), node("b"), node("c"), node("d"));
        a.connect(&b, ());
        b.connect(&c, ());
        c.connect(&d, ());
        assert_eq!(walk_names(&a, Direction::Forward), ["a", "b", "c", "d"]);
        assert_eq!(walk_names(&d, Direction::Backward), ["d", "c", "b", "a"]);
        assert_eq!(walk_names(&d, Direction::Forward), ["d"]);
    }

    #[test]
    fn walk_threads_accumulator_along_paths() {
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
    fn walk_passes_the_edge_just_crossed() {
        let (a, b, c) = (Node::new("a"), Node::new("b"), Node::new("c"));
        a.connect(&b, "x");
        b.connect(&c, "y");
        let mut via = Vec::new();
        a.walk(Direction::Forward, (), |_, _, e| {
            via.push(e.map(|e| **e.data()));
        });
        assert_eq!(via, [None, Some("x"), Some("y")]);
    }

    #[test]
    fn walk_self_loop_terminates() {
        let a = node("a");
        a.connect(&a, ());
        assert_eq!(walk_names(&a, Direction::Forward), ["a"]);
        assert_eq!(walk_names(&a, Direction::Backward), ["a"]);
    }

    #[test]
    fn walk_two_node_cycle_terminates() {
        let (a, b) = (node("a"), node("b"));
        a.connect(&b, ());
        b.connect(&a, ());
        assert_eq!(walk_names(&a, Direction::Forward), ["a", "b"]);
    }

    #[test]
    fn walk_ring_terminates() {
        let (a, b, c) = (node("a"), node("b"), node("c"));
        a.connect(&b, ());
        b.connect(&c, ());
        c.connect(&a, ());
        assert_eq!(walk_names(&a, Direction::Forward), ["a", "b", "c"]);
        assert_eq!(walk_names(&a, Direction::Backward), ["a", "c", "b"]);
    }

    #[test]
    fn walk_diamond_visits_shared_node_once_per_path() {
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
    fn walk_callback_may_mutate_the_graph() {
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

    #[test]
    fn find_checks_the_start_node() {
        let a = node("a");
        let found = a.find(Direction::Forward, Traversal::BreadthFirst, |n| {
            *n.data() == "a"
        });
        assert!(found == Some(a.clone()));
    }

    #[test]
    fn find_returns_none_when_nothing_matches() {
        let (a, ..) = diamond();
        let found = a.find(Direction::Forward, Traversal::BreadthFirst, |n| {
            *n.data() == "z"
        });
        assert!(found.is_none());
    }

    #[test]
    fn find_respects_direction() {
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
    fn find_bfs_and_dfs_pick_different_matches() {
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
    fn find_visits_each_node_once_even_in_a_diamond_or_cycle() {
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
        let _guard = a.node.borrow_mut();
        assert!(format!("{a:?}").contains("mutably borrowed"));
    }
}
