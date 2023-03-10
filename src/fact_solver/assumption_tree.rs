use std::{
    fmt::{Display, Formatter},
    ops::{Index, IndexMut},
};

use thiserror::Error;

struct AssumptionTreeNode<T> {
    pub parent: Option<AssumptionTreeNodeHandle>,
    pub data: T,
    children: Vec<AssumptionTreeNodeHandle>,
}

#[derive(Copy, Clone)]
pub struct AssumptionTreeNodeHandle(usize);

pub struct AssumptionTree<T> {
    nodes: Vec<AssumptionTreeNode<T>>,
}

#[derive(Debug, Error)]
pub enum AssumptionTreeError {
    #[error("Node {0} does not exist")]
    UnknownNode(usize),
}

impl<T> AssumptionTree<T> {
    pub fn new(initial: T) -> (Self, AssumptionTreeNodeHandle) {
        (
            Self {
                nodes: vec![AssumptionTreeNode {
                    parent: None,
                    data: initial,
                    children: vec![],
                }],
            },
            AssumptionTreeNodeHandle(0),
        )
    }

    pub fn insert_child(
        &mut self,
        parent: AssumptionTreeNodeHandle,
        child: T,
    ) -> AssumptionTreeNodeHandle {
        self.nodes.push(AssumptionTreeNode {
            parent: Some(parent),
            data: child,
            children: vec![],
        });

        let child_handle = AssumptionTreeNodeHandle(self.nodes.len() - 1);

        self.nodes[parent.0].children.push(child_handle);

        return child_handle;
    }

    pub fn get_handle(&self, node: usize) -> Result<AssumptionTreeNodeHandle, AssumptionTreeError> {
        if node >= self.nodes.len() {
            return Err(AssumptionTreeError::UnknownNode(node));
        }
        return Ok(AssumptionTreeNodeHandle(node));
    }

    pub fn parent_of(&self, node: AssumptionTreeNodeHandle) -> Option<AssumptionTreeNodeHandle> {
        self.nodes[node.0].parent
    }
}

impl<T> Index<AssumptionTreeNodeHandle> for AssumptionTree<T> {
    type Output = T;

    fn index(&self, index: AssumptionTreeNodeHandle) -> &Self::Output {
        &self.nodes.index(index.0).data
    }
}

impl<T> IndexMut<AssumptionTreeNodeHandle> for AssumptionTree<T> {
    fn index_mut(&mut self, index: AssumptionTreeNodeHandle) -> &mut Self::Output {
        &mut self.nodes.index_mut(index.0).data
    }
}

impl<T> Display for AssumptionTree<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        fn do_fmt<U: Display>(
            handle: AssumptionTreeNodeHandle,
            nodes: &AssumptionTree<U>,
            indent: usize,
            f: &mut Formatter<'_>,
        ) -> std::fmt::Result {
            write!(f, "{0:1$} - ({3}) {2}\n", "", indent, nodes[handle], handle)?;
            for child in nodes.nodes[handle.0].children.iter() {
                do_fmt(*child, nodes, indent + 2, f)?;
            }
            Ok(())
        }

        do_fmt(AssumptionTreeNodeHandle(0), self, 0, f)
    }
}

impl Display for AssumptionTreeNodeHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}
