//! Stage 2.5: parent/child nesting of regions.
//!
//! A region that sits inside another region is a hole, not a separate path. The
//! hierarchy walks every pair of bounds once and assigns each region a single
//! parent — the smallest region that strictly contains it. Regions with no
//! parent are roots; everything else is a hole of its parent.

use crate::segment::regions::Region;

/// One region's place in the nesting tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    /// Index into the region slice this node was built from.
    pub region: usize,
    /// Index of the containing region, or `u32::MAX` for a root.
    pub parent: u32,
    /// Children, in the same order as the source slice.
    pub children: Vec<u32>,
}

/// Build the nesting forest for a slice of regions.
///
/// A region becomes a child of the smallest region whose bounds strictly
/// contain it. Two regions whose bounds touch at an edge never nest; containment
/// requires strict inequality on all four sides so diagonally touching shapes
/// stay siblings.
pub fn build(regions: &[Region]) -> Vec<Node> {
    let mut nodes: Vec<Node> = regions
        .iter()
        .enumerate()
        .map(|(index, _)| Node {
            region: index,
            parent: u32::MAX,
            children: Vec::new(),
        })
        .collect();
    let mut links: Vec<(usize, usize)> = Vec::new();
    for (index, region) in regions.iter().enumerate() {
        if let Some((container, _)) = regions
            .iter()
            .enumerate()
            .filter(|(other, container)| {
                *other != index && container.bounds.strictly_contains(&region.bounds)
            })
            .min_by_key(|(_, container)| container.bounds.area())
        {
            nodes[index].parent = container as u32;
            links.push((container, index));
        }
    }
    for (parent, child) in links {
        nodes[parent].children.push(child as u32);
    }
    nodes
}

/// Indices of the top-level regions — those with no containing parent.
pub fn roots(nodes: &[Node]) -> Vec<usize> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| node.parent == u32::MAX)
        .map(|(index, _)| index)
        .collect()
}

/// Indices of every region nested inside `root`, directly or transitively.
pub fn descendants(nodes: &[Node], root: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let mut stack = nodes[root].children.clone();
    while let Some(child) = stack.pop() {
        out.push(child as usize);
        stack.extend_from_slice(&nodes[child as usize].children);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::segment::regions::Bounds;
    use crate::segment::regions::Region;

    fn region(id: u32, label: u32, bounds: Bounds) -> Region {
        Region {
            id,
            label,
            pixels: Vec::new(),
            bounds,
        }
    }

    #[test]
    fn nested_regions_get_a_parent() {
        let regions = vec![
            region(
                0,
                0,
                Bounds {
                    min_x: 0,
                    min_y: 0,
                    max_x: 10,
                    max_y: 10,
                },
            ),
            region(
                1,
                1,
                Bounds {
                    min_x: 2,
                    min_y: 2,
                    max_x: 5,
                    max_y: 5,
                },
            ),
        ];
        let nodes = build(&regions);
        assert_eq!(nodes[0].parent, u32::MAX);
        assert_eq!(nodes[1].parent, 0);
        assert_eq!(nodes[0].children, vec![1]);
    }

    #[test]
    fn touching_edges_do_not_nest() {
        let regions = vec![
            region(
                0,
                0,
                Bounds {
                    min_x: 0,
                    min_y: 0,
                    max_x: 10,
                    max_y: 10,
                },
            ),
            region(
                1,
                1,
                Bounds {
                    min_x: 10,
                    min_y: 0,
                    max_x: 20,
                    max_y: 10,
                },
            ),
        ];
        let nodes = build(&regions);
        assert_eq!(nodes[0].parent, u32::MAX);
        assert_eq!(nodes[1].parent, u32::MAX);
        assert!(nodes[0].children.is_empty());
        assert!(nodes[1].children.is_empty());
    }

    #[test]
    fn smallest_containing_region_wins() {
        let regions = vec![
            region(
                0,
                0,
                Bounds {
                    min_x: 0,
                    min_y: 0,
                    max_x: 100,
                    max_y: 100,
                },
            ),
            region(
                1,
                1,
                Bounds {
                    min_x: 10,
                    min_y: 10,
                    max_x: 90,
                    max_y: 90,
                },
            ),
            region(
                2,
                2,
                Bounds {
                    min_x: 20,
                    min_y: 20,
                    max_x: 30,
                    max_y: 30,
                },
            ),
        ];
        let nodes = build(&regions);
        assert_eq!(nodes[2].parent, 1);
        assert_eq!(nodes[1].parent, 0);
        assert_eq!(nodes[0].parent, u32::MAX);
    }

    #[test]
    fn roots_are_the_uncontained_regions() {
        let regions = vec![
            region(
                0,
                0,
                Bounds {
                    min_x: 0,
                    min_y: 0,
                    max_x: 10,
                    max_y: 10,
                },
            ),
            region(
                1,
                1,
                Bounds {
                    min_x: 1,
                    min_y: 1,
                    max_x: 5,
                    max_y: 5,
                },
            ),
            region(
                2,
                2,
                Bounds {
                    min_x: 20,
                    min_y: 20,
                    max_x: 30,
                    max_y: 30,
                },
            ),
        ];
        let nodes = build(&regions);
        assert_eq!(roots(&nodes), vec![0, 2]);
    }

    #[test]
    fn descendants_walk_the_whole_subtree() {
        let regions = vec![
            region(
                0,
                0,
                Bounds {
                    min_x: 0,
                    min_y: 0,
                    max_x: 100,
                    max_y: 100,
                },
            ),
            region(
                1,
                1,
                Bounds {
                    min_x: 10,
                    min_y: 10,
                    max_x: 90,
                    max_y: 90,
                },
            ),
            region(
                2,
                2,
                Bounds {
                    min_x: 20,
                    min_y: 20,
                    max_x: 30,
                    max_y: 30,
                },
            ),
        ];
        let nodes = build(&regions);
        assert_eq!(descendants(&nodes, 0), vec![1, 2]);
        assert_eq!(descendants(&nodes, 1), vec![2]);
        assert!(descendants(&nodes, 2).is_empty());
    }
}
