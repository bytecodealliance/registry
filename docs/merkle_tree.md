# Sparse Merkle Tree

The Sparse Merkle Tree used in the Warg protocol is based on those used in [Revocation Transparency](https://github.com/google/trillian/blob/master/docs/papers/RevocationTransparency.pdf).

## Basic Structure

The tree is a fully balanced full height binary tree with 256 levels of branches (one for every bit in the key hash) and one level of leaves.

The hash of a given node of the tree is defined as follows.
```
hash(Branch(left, right)) == hash(0x01 || hash(left) || hash(right))

hash(Leaf(value)) == hash(0x00 || value)
```

With this structure, changing the value of any leaf or changing the order of any two values will produce a different hash. So the hash of the tree identifies its values and structure uniquely.

In this tree the path from root to leaf, represented as 0 for left and 1 for right, encodes the hash of the key. This means that there is one unique leaf position for every possible key and you can verify that a key/value pair is present using a merkle audit path following that path.

The tree is sparse because most of the values in leaves will be the same "zero" value (we'll call it `<zero-value>`) and non-zero values will be exceedingly rare.

## Empty Subtree Optimization

We can represent a subtree which is empty (its leaves are all the zero value) as a single node `Empty(h)` where `h` is the height of the subtree. The hash of this subtree is a simple recursive function of `h`.

```
Empty(h)

hash(Empty(h)) = {
   hash(0x01 || hash(Empty(h-1)) || hash(Empty(h-1))) if h != 0
   hash(0x00 || <zero-value>) if h == 0
}
```

Since the only valid values of `h` are 0-256, we can iteratively pre-compute all of the possible values trivially and populate a table with them.

```rust
let heights: Vec<_> = (0..=256).scan(<zero-value>, |old, _| {
    *old = hash(*old, *old);
    Some(*old)
}).collect();

fn empty_tree_hash(h: usize) -> Hash<D> {
    heights[h].to_owned()
}
```

If needed, the left and right children of an `Empty` node can be computed as follows.

```
where h > 0,
left_child(Empty(h)) == right_child(Empty(h)) == Empty(h-1)
```

## Singleton Subtree Optimization

We can represent a subtree with exactly one non-zero leaf as a single node `Singleton(h, key, value)` where `h` is the height of the subtree and `key`/`value` are the key and value of that one non-zero leaf entry.

```
Singleton(h, key, value)

hash(Singleton(h, key, value)) = {
   hash(0x00 || value) if h == 0
   hash(0x01 || hash(Singleton(h-1, key, value)) || hash(Empty(h)) if bit(h, key) == 0
   hash(0x01 || hash(Empty(h) || hash(Singleton(h-1, key, value))) if bit(h, key) == 1
}
```

If needed, the left and right children of an `Singleton` node can be computed as follows.

```
where h > 0,

left_child(Singleton(h, key, value)) = {
   Singleton(h-1, key, value) if bit(h, key) == 0,
   Empty(h-1) if bit(h, key) == 1
}

and

right_child(Singleton(h, key, value)) = {
   Empty(h-1) if bit(h, key) == 0,
   Singleton(h-1, key, value) if bit(h, key) == 1
}
```

In the following diagram, the subtree with prefix `0` is a Singleton subtree of height 3, with entry key `000` and entry value `v`. It can be represented as a single node `Singleton(3, "000", v)`  shown in purple.

![](https://hackmd.io/_uploads/Bkl2nrFS2.png)

## Insertion Case Analysis

Trees are created by inserting/updating entries in existing trees using the recursive function `insert(node, h, key, value)` where `node` is initially the root and `h` is initially 256.

The structure of the resulting tree depends on what kind of node `root` and its children are.

```
insert(Empty(h), h, key, value) == Singleton(h, key, value)

insert(Singleton(h, key*, value*), h, key, value) = {
   Branch(Singleton(h-1, key, value), Singleton(h-1, key*, value*))
       if bit(h, key) == 0 and bit(h, key*) == 1,

   Branch(Singleton(h-1, key*, value*), Singleton(h-1, key, value))
       if bit(h, key) == 1 and bit(h, key*) == 0,

   Branch(insert(Singleton(h-1, key*, value*), h-1, key, value), Empty(h-1))
       if bit(h, key) == bit(h, key*) == 0,

   Branch(Empty(h-1), insert(Singleton(h-1, key*, value*), h-1, key, value))
       if bit(h, key) == bit(h, key*) == 1,
}

insert(Branch(left, right), h, key, value) = {
   Branch(insert(left, key, value), right) if bit(h, key) == 0,
   Branch(left, insert(right, key, value)) if bit(h, key) == 0
}
```

The basic cases below show insertion into empty and singleton nodes.

![](https://hackmd.io/_uploads/SJjAhSFSn.png)

## Proof Compression

An inclusion proof corresponds to a "merkle audit path" from the leaf node being included up to the root. This path is made up of the sibling of the input node, its parent sibling, ... and so on up the tree. We refer to these as the "peers" of the node. The peers represent the data of the proof and by encoding/representing them more compactly we can reduce the data we send to clients.

Since our tree has 2^256 potential leaves, which represent 2^256 potential key values, a given leaf has 256 ancestors (one branch node for each bit). The length of the proof is 1 (sibling leaf) + 256 (ancestors) - 1 (the root hash being derived)

Naively, we could encode a proof as the hash of every peer node, which is shown as column A in the following diagram. Columns B and C can be derived by taking advantage of empty subtree elision and column D by using singleton subtree elision.

![](https://hackmd.io/_uploads/ryeyf5QLh.png)

To do this, consider a recursive formulation of the peer list construction where again `node` is initially the root `h` is initially 256.

```
peers(node, h, key) = {
   [right_child(node), *peers(left_child(node), h-1, key)] if bit(h, key) == 0
   [left_child(node), *peers(right_child(node), h-1, key)] if bit(h, key) == 1
}
```

### Empty Subtree Optimization (A->B->C)

Since the height of the subtree represented by a peer is directly proportional to its index (because peers are ordered) `Empty(h)` subtrees can be encoded as `None` instead of as their hashes to save space and we can always recompute `h` from `256-index` to reverse the process.

```
peers(node, h, key) = {
   [None, *peers(left_child(node), h-1, key)]
      if bit(h, key) == 0 and right_child(node) == Empty(h)

   [None, *peers(right_child(node), h-1, key)]
      if bit(h, key) == 1 and left_child(node) == Empty(h)
}
```

### Singleton Subtree Optimization (C->D)

If a given subtree contains exactly one value, which we represent above as `Singleton(h, key, value)` and the key in question is the same `key`, then all of the peers in that subtree will be empty trees since only one path contains non-empty values.

```
peers(node, h, key) = {
   [] if node == Singleton(h, key, value)
}
```
