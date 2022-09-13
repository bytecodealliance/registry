use alloc::vec::Vec;

/// Represents a node in a tree by its index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Node(pub(crate) usize);

/// What side of its parent a given node is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

impl Node {
    #[inline]
    pub fn index(&self) -> usize {
        self.0
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.index().trailing_ones()
    }

    #[inline]
    fn delta(&self) -> usize {
        2usize.pow(self.height())
    }

    #[inline]
    pub fn side(&self) -> Side {
        let height = self.height();
        let shift = height + 1;
        let shifted = self.index() >> shift;
        let masked = shifted & 1;
        if masked == 0 {
            Side::Left
        } else {
            Side::Right
        }
    }

    #[inline]
    pub fn left_sibling(&self) -> Node {
        assert_eq!(self.side(), Side::Right);
        let delta = self.delta();
        Node(self.index() - delta - delta)
    }

    #[inline]
    pub fn right_sibling(&self) -> Node {
        assert_eq!(self.side(), Side::Left);
        let delta = self.delta();
        Node(self.index() + delta + delta)
    }

    #[inline]
    pub fn sibling(&self) -> Node {
        match self.side() {
            Side::Left => self.right_sibling(),
            Side::Right => self.left_sibling(),
        }
    }

    /// Finds the parent of a given node index.
    /// The parent for a left node is after it
    /// and the parent for a right node is before it.
    #[inline]
    pub fn parent(&self) -> Node {
        let parent_index = match self.side() {
            Side::Left => self.index() + self.delta(),
            Side::Right => self.index() - self.delta(),
        };
        Node(parent_index)
    }

    #[inline]
    pub fn children(&self) -> (Node, Node) {
        assert_ne!(self.height(), 0);
        let index = self.index();
        let child_delta = self.delta() / 2;
        (Node(index - child_delta), Node(index + child_delta))
    }

    #[inline]
    pub fn rightmost_descendent(&self) -> Node {
        let offset = 2usize.pow(self.height()) - 1;
        Node(self.index() + offset)
    }

    #[inline]
    pub fn leftmost_descendent(&self) -> Node {
        let offset = 2usize.pow(self.height()) - 1;
        Node(self.index() - offset)
    }

    #[inline]
    pub fn exists_at_length(&self, length: usize) -> bool {
        let last_child = self.rightmost_descendent();
        let required_entries = last_child.index() / 2;
        required_entries < length
    }

    #[inline]
    pub fn has_children_at_length(&self, length: usize) -> bool {
        self.leftmost_descendent().exists_at_length(length)
    }

    #[inline]
    pub fn next_node_with_height(&self, height: u32) -> Node {
        assert!(
            self.height() >= height,
            "This algorithm is designed to only work for smaller or equal successors"
        );
        let first_with_height = Self::first_node_with_height(height);
        let next_leaf = self.rightmost_descendent().index() + 2;
        Node(first_with_height.index() + next_leaf)
    }

    /// Compute the left-most node which has a given height.
    #[inline]
    pub fn first_node_with_height(height: u32) -> Node {
        Node(2usize.pow(height) - 1)
    }

    /// Compute the balanced roots for a log with a given
    /// log length in number of leaves.
    #[inline]
    pub fn broots_for_len(length: usize) -> Vec<Node> {
        let mut value = length;
        let mut broot_heights = Vec::new();
        for i in 0..usize::BITS {
            let present = (value & 1) == 1;
            if present {
                broot_heights.push(i);
            }

            value = value >> 1;
        }

        let mut broots = Vec::new();
        let mut current: Option<Node> = None;
        for broot_height in broot_heights.into_iter().rev() {
            let next = match current {
                None => Self::first_node_with_height(broot_height),
                Some(last) => last.next_node_with_height(broot_height),
            };
            broots.push(next);
            current = Some(next);
        }

        broots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_height_delta() {
        // Heights for nodes at indices
        assert_eq!(Node(0).height(), 0);
        assert_eq!(Node(1).height(), 1);
        assert_eq!(Node(2).height(), 0);
        assert_eq!(Node(3).height(), 2);
        assert_eq!(Node(4).height(), 0);
        assert_eq!(Node(5).height(), 1);
        assert_eq!(Node(6).height(), 0);
        assert_eq!(Node(7).height(), 3);
        assert_eq!(Node(8).height(), 0);
        assert_eq!(Node(9).height(), 1);
        assert_eq!(Node(10).height(), 0);
        assert_eq!(Node(11).height(), 2);
        assert_eq!(Node(12).height(), 0);
        assert_eq!(Node(13).height(), 1);
        assert_eq!(Node(14).height(), 0);

        // Deltas for nodes at indices
        assert_eq!(Node(0).height(), 0);
        assert_eq!(Node(1).height(), 1);
        assert_eq!(Node(2).height(), 0);
        assert_eq!(Node(3).height(), 2);
        assert_eq!(Node(4).height(), 0);
        assert_eq!(Node(5).height(), 1);
        assert_eq!(Node(6).height(), 0);
        assert_eq!(Node(7).height(), 3);
        assert_eq!(Node(8).height(), 0);
        assert_eq!(Node(9).height(), 1);
        assert_eq!(Node(10).height(), 0);
        assert_eq!(Node(11).height(), 2);
        assert_eq!(Node(12).height(), 0);
        assert_eq!(Node(13).height(), 1);
        assert_eq!(Node(14).height(), 0);
    }

    #[test]
    fn test_node_neighbors() {
        // Whether each node is a left or right side child
        assert_eq!(Node(0).side(), Side::Left);
        assert_eq!(Node(1).side(), Side::Left);
        assert_eq!(Node(2).side(), Side::Right);
        assert_eq!(Node(3).side(), Side::Left);
        assert_eq!(Node(4).side(), Side::Left);
        assert_eq!(Node(5).side(), Side::Right);
        assert_eq!(Node(6).side(), Side::Right);
        assert_eq!(Node(7).side(), Side::Left);
        assert_eq!(Node(8).side(), Side::Left);
        assert_eq!(Node(9).side(), Side::Left);
        assert_eq!(Node(10).side(), Side::Right);
        assert_eq!(Node(11).side(), Side::Right);
        assert_eq!(Node(12).side(), Side::Left);
        assert_eq!(Node(13).side(), Side::Right);
        assert_eq!(Node(14).side(), Side::Right);

        // Sibling index for each node
        assert_eq!(Node(0).right_sibling(), Node(2));
        assert_eq!(Node(1).right_sibling(), Node(5));
        assert_eq!(Node(2).left_sibling(), Node(0));
        assert_eq!(Node(3).right_sibling(), Node(11));
        assert_eq!(Node(4).right_sibling(), Node(6));
        assert_eq!(Node(5).left_sibling(), Node(1));
        assert_eq!(Node(6).left_sibling(), Node(4));
        assert_eq!(Node(7).right_sibling(), Node(23));
        assert_eq!(Node(8).right_sibling(), Node(10));
        assert_eq!(Node(9).right_sibling(), Node(13));
        assert_eq!(Node(10).left_sibling(), Node(8));
        assert_eq!(Node(11).left_sibling(), Node(3));
        assert_eq!(Node(12).right_sibling(), Node(14));
        assert_eq!(Node(13).left_sibling(), Node(9));
        assert_eq!(Node(14).left_sibling(), Node(12));

        // Parent index for each node
        assert_eq!(Node(0).parent(), Node(1));
        assert_eq!(Node(1).parent(), Node(3));
        assert_eq!(Node(2).parent(), Node(1));
        assert_eq!(Node(3).parent(), Node(7));
        assert_eq!(Node(4).parent(), Node(5));
        assert_eq!(Node(5).parent(), Node(3));
        assert_eq!(Node(6).parent(), Node(5));
        assert_eq!(Node(7).parent(), Node(15));
        assert_eq!(Node(8).parent(), Node(9));
        assert_eq!(Node(9).parent(), Node(11));
        assert_eq!(Node(10).parent(), Node(9));
        assert_eq!(Node(11).parent(), Node(7));
        assert_eq!(Node(12).parent(), Node(13));
        assert_eq!(Node(13).parent(), Node(11));
        assert_eq!(Node(14).parent(), Node(13));

        // Children indices for each branch node
        assert_eq!(Node(1).children(), (Node(0), Node(2)));
        assert_eq!(Node(3).children(), (Node(1), Node(5)));
        assert_eq!(Node(5).children(), (Node(4), Node(6)));
        assert_eq!(Node(7).children(), (Node(3), Node(11)));
        assert_eq!(Node(9).children(), (Node(8), Node(10)));
        assert_eq!(Node(11).children(), (Node(9), Node(13)));
        assert_eq!(Node(13).children(), (Node(12), Node(14)));
    }

    #[test]
    fn test_node_existence() {
        // The rightmost descendent of each branch node
        assert_eq!(Node(1).rightmost_descendent(), Node(2));
        assert_eq!(Node(3).rightmost_descendent(), Node(6));
        assert_eq!(Node(5).rightmost_descendent(), Node(6));
        assert_eq!(Node(7).rightmost_descendent(), Node(14));
        assert_eq!(Node(9).rightmost_descendent(), Node(10));
        assert_eq!(Node(11).rightmost_descendent(), Node(14));
        assert_eq!(Node(13).rightmost_descendent(), Node(14));

        // Whether each branch node exists at a given length
        let cases = [(1, 2), (3, 4), (5, 4), (7, 8), (9, 6), (11, 8), (13, 8)];
        for (index, min_len) in cases {
            let node = Node(index);
            for len in 0..=8 {
                if len >= min_len {
                    assert!(
                        node.exists_at_length(len),
                        "Node {} should exist when length is {}",
                        index,
                        len
                    );
                } else {
                    assert!(
                        !node.exists_at_length(len),
                        "Node {} should not exist when length is {}",
                        index,
                        len
                    );
                }
            }
        }
    }

    #[test]
    fn test_first_nodes() {
        // First node with each height
        let first_0 = Node::first_node_with_height(0);
        assert_eq!(first_0, Node(0));
        assert_eq!(first_0.next_node_with_height(0), Node(2));

        let first_1 = Node::first_node_with_height(1);
        assert_eq!(first_1, Node(1));
        assert_eq!(first_1.next_node_with_height(0), Node(4));
        assert_eq!(first_1.next_node_with_height(1), Node(5));

        let first_2 = Node::first_node_with_height(2);
        assert_eq!(first_2, Node(3));
        assert_eq!(first_2.next_node_with_height(0), Node(8));
        assert_eq!(first_2.next_node_with_height(1), Node(9));
        assert_eq!(first_2.next_node_with_height(2), Node(11));

        let first_3 = Node::first_node_with_height(3);
        assert_eq!(first_3, Node(7));
        assert_eq!(first_3.next_node_with_height(0), Node(16));
        assert_eq!(first_3.next_node_with_height(1), Node(17));
        assert_eq!(first_3.next_node_with_height(2), Node(19));
        assert_eq!(first_3.next_node_with_height(3), Node(23));

        assert_eq!(Node::first_node_with_height(4), Node(15));
    }

    #[test]
    fn test_broots() {
        use alloc::vec;

        // This math is used when computing which roots are available
        assert_eq!(Node::broots_for_len(0), vec![]);
        assert_eq!(Node::broots_for_len(1), vec![Node(0)]);
        assert_eq!(Node::broots_for_len(2), vec![Node(1)]);
        assert_eq!(Node::broots_for_len(3), vec![Node(1), Node(4)]);
        assert_eq!(Node::broots_for_len(4), vec![Node(3)]);
        assert_eq!(Node::broots_for_len(5), vec![Node(3), Node(8)]);
        assert_eq!(Node::broots_for_len(6), vec![Node(3), Node(9)]);
        assert_eq!(Node::broots_for_len(7), vec![Node(3), Node(9), Node(12)]);
        assert_eq!(Node::broots_for_len(8), vec![Node(7)]);
    }
}
