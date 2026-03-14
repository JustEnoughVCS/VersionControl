use std::{
    borrow::Borrow,
    collections::HashSet,
    ops::{Deref, Index},
};

use crate::{
    compare::compare_string,
    mapping::LocalMapping,
    sheet::{Sheet, SheetData},
};

pub struct SheetDataTree<'a> {
    raw_data: &'a SheetData,
    root: SheetDataTreeNode<'a>,
}

impl<'a> From<&'a Sheet> for SheetDataTree<'a> {
    fn from(value: &'a Sheet) -> Self {
        (&value.data).into()
    }
}

impl<'a> From<&'a SheetData> for SheetDataTree<'a> {
    fn from(value: &'a SheetData) -> Self {
        let mut root = HashSet::new();

        for mapping in value.mappings() {
            let path_parts = mapping.value();
            if path_parts.is_empty() {
                continue;
            }

            Self::insert_mapping_into_tree(&mut root, path_parts, mapping);
        }

        Self {
            raw_data: value,
            root: SheetDataTreeNode::Directory("", root),
        }
    }
}

impl<'a> SheetDataTree<'a> {
    /// Get the root of the tree for accessing the tree
    pub fn root(&self) -> &SheetDataTreeNode<'a> {
        &self.root
    }

    fn insert_mapping_into_tree(
        nodes: &mut HashSet<SheetDataTreeNode<'a>>,
        path_parts: &'a [String],
        mapping: &'a LocalMapping,
    ) {
        if path_parts.is_empty() {
            return;
        }

        let current_part = &path_parts[0];
        let remaining_parts = &path_parts[1..];

        if remaining_parts.is_empty() {
            nodes.insert(SheetDataTreeNode::Mapping(current_part, mapping));
        } else {
            let dir_node = SheetDataTreeNode::Directory(current_part, HashSet::new());

            if let Some(mut existing_dir) = nodes.take(&dir_node) {
                if let SheetDataTreeNode::Directory(_, ref mut children) = existing_dir {
                    Self::insert_mapping_into_tree(children, remaining_parts, mapping);
                }
                nodes.insert(existing_dir);
            } else {
                let mut children = HashSet::new();
                Self::insert_mapping_into_tree(&mut children, remaining_parts, mapping);
                nodes.insert(SheetDataTreeNode::Directory(current_part, children));
            }
        }
    }
}

impl<'a> Deref for SheetDataTree<'a> {
    type Target = SheetData;
    fn deref(&self) -> &Self::Target {
        self.raw_data
    }
}

impl<'a> AsRef<SheetData> for SheetDataTree<'a> {
    fn as_ref(&self) -> &SheetData {
        self.raw_data
    }
}

pub enum SheetDataTreeNode<'a> {
    Directory(&'a str, HashSet<SheetDataTreeNode<'a>>),
    Mapping(&'a str, &'a LocalMapping),
}

impl<'a> SheetDataTreeNode<'a> {
    /// Returns a reference to the child node with the given name, if it exists.
    /// Only works on directory nodes; returns `None` for mapping nodes.
    pub fn next(&self, name: &str) -> Option<&SheetDataTreeNode<'a>> {
        let SheetDataTreeNode::Directory(_, nodes) = self else {
            return None;
        };
        nodes.get(name)
    }

    /// Returns a reference to the `LocalMapping` if this node is a mapping node.
    /// Returns `None` if this node is a directory node.
    pub fn mapping(&self) -> Option<&'a LocalMapping> {
        match self {
            SheetDataTreeNode::Mapping(_, mapping) => Some(mapping),
            _ => None,
        }
    }

    /// Returns a reference to the children hash set if this node is a directory node.
    /// Returns `None` if this node is a mapping node.
    pub fn dir(&'a self) -> Option<&'a HashSet<SheetDataTreeNode<'a>>> {
        match self {
            SheetDataTreeNode::Directory(_, children) => Some(children),
            _ => None,
        }
    }

    /// Returns a reference to the `LocalMapping` if this node is a mapping node.
    /// Panics if called on a directory node.
    pub fn unwrap_mapping(&self) -> &'a LocalMapping {
        match self {
            SheetDataTreeNode::Mapping(_, mapping) => mapping,
            _ => panic!("called `unwrap_mapping` on a directory node"),
        }
    }

    /// Returns a reference to the children hash set if this node is a directory node.
    /// Panics if called on a mapping node.
    pub fn unwrap_dir(&'a self) -> &'a HashSet<SheetDataTreeNode<'a>> {
        match self {
            SheetDataTreeNode::Directory(_, children) => children,
            _ => panic!("called `unwrap_dir` on a mapping node"),
        }
    }

    /// Returns a reference to the `LocalMapping` if this node is a mapping node.
    /// Returns the provided default reference if this node is a directory node.
    pub fn unwrap_mapping_or(&self, default: &'a LocalMapping) -> &'a LocalMapping {
        match self {
            SheetDataTreeNode::Mapping(_, mapping) => mapping,
            _ => default,
        }
    }

    /// Returns a reference to the `LocalMapping` if this node is a mapping node.
    /// Otherwise, calls the provided closure and returns its result.
    pub fn unwrap_mapping_or_else<F>(&self, f: F) -> &'a LocalMapping
    where
        F: FnOnce() -> &'a LocalMapping,
    {
        match self {
            SheetDataTreeNode::Mapping(_, mapping) => mapping,
            _ => f(),
        }
    }

    /// Returns a reference to the children hash set if this node is a directory node.
    /// Returns the provided default reference if this node is a mapping node.
    pub fn unwrap_dir_or(
        &'a self,
        default: &'a HashSet<SheetDataTreeNode<'a>>,
    ) -> &'a HashSet<SheetDataTreeNode<'a>> {
        match self {
            SheetDataTreeNode::Directory(_, children) => children,
            _ => default,
        }
    }

    /// Returns a reference to the children hash set if this node is a directory node.
    /// Otherwise, calls the provided closure and returns its result.
    pub fn unwrap_dir_or_else<F>(&'a self, f: F) -> &'a HashSet<SheetDataTreeNode<'a>>
    where
        F: FnOnce() -> &'a HashSet<SheetDataTreeNode<'a>>,
    {
        match self {
            SheetDataTreeNode::Directory(_, children) => children,
            _ => f(),
        }
    }

    /// Returns `true` if this node is a mapping node.
    pub fn is_mapping(&self) -> bool {
        matches!(self, SheetDataTreeNode::Mapping(_, _))
    }

    /// Returns `true` if this node is a directory node.
    pub fn is_dir(&self) -> bool {
        matches!(self, SheetDataTreeNode::Directory(_, _))
    }
}

impl<'a> Index<&str> for SheetDataTreeNode<'a> {
    type Output = SheetDataTreeNode<'a>;

    fn index(&self, index: &str) -> &Self::Output {
        match self {
            SheetDataTreeNode::Directory(_, children) => children
                .get(index)
                .unwrap_or_else(|| panic!("No entry found for key: `{}`", index)),
            _ => panic!("Cannot index into a mapping node"),
        }
    }
}

impl<'a> PartialOrd for SheetDataTreeNode<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for SheetDataTreeNode<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        use SheetDataTreeNode::*;
        use std::cmp::Ordering;

        match (self, other) {
            (Directory(_, _), Mapping(_, _)) => Ordering::Less,
            (Mapping(_, _), Directory(_, _)) => Ordering::Greater,
            (Directory(name1, _), Directory(name2, _)) => compare_string(name1, name2),
            (Mapping(name1, _), Mapping(name2, _)) => compare_string(name1, name2),
        }
    }
}

impl<'a> PartialEq for SheetDataTreeNode<'a> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (SheetDataTreeNode::Directory(name1, _), SheetDataTreeNode::Directory(name2, _)) => {
                name1 == name2
            }
            (SheetDataTreeNode::Mapping(name1, _), SheetDataTreeNode::Mapping(name2, _)) => {
                name1 == name2
            }
            _ => false,
        }
    }
}

impl<'a> Eq for SheetDataTreeNode<'a> {}

impl<'a> PartialEq<&str> for SheetDataTreeNode<'a> {
    fn eq(&self, other: &&str) -> bool {
        match self {
            SheetDataTreeNode::Directory(name, _) => name == other,
            SheetDataTreeNode::Mapping(name, _) => name == other,
        }
    }
}

impl<'a> Borrow<str> for SheetDataTreeNode<'a> {
    fn borrow(&self) -> &str {
        match self {
            SheetDataTreeNode::Directory(name, _) => name,
            SheetDataTreeNode::Mapping(name, _) => name,
        }
    }
}

impl<'a> std::hash::Hash for SheetDataTreeNode<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            SheetDataTreeNode::Directory(name, _) => {
                name.hash(state);
            }
            SheetDataTreeNode::Mapping(name, _) => {
                name.hash(state);
            }
        }
    }
}

#[test]
fn test_sheet_tree_build() {
    let data = {
        let mut sheet = SheetData::empty().pack("sheet");
        (0..20).for_each(|n| {
            sheet
                .insert_mapping(
                    LocalMapping::new(
                        crate::lazy_node!["Assets", format!("Player_Frame_{}.png", n)],
                        crate::lazy_ridx!(n, 2),
                        crate::mapping::LocalMappingForward::Latest,
                    )
                    .unwrap(),
                )
                .unwrap();
        });
        sheet.apply().unwrap();
        sheet.unpack()
    };

    let tree: SheetDataTree = (&data).into();
    assert!(tree.root().next("Assets").unwrap().is_dir());
    assert!(
        tree.root()
            .next("Assets")
            .unwrap()
            .next("Player_Frame_5.png")
            .unwrap()
            .is_mapping()
    );
    assert!(tree.root().next("Binary").is_none());

    assert!(tree.root()["Assets"].is_dir());
    assert!(tree.root()["Assets"]["Player_Frame_12.png"].is_mapping());
    assert!(
        std::panic::catch_unwind(|| {
            let _ = &tree.root()["Binary"];
        })
        .is_err()
    );

    assert!(
        tree.root()["Assets"]["Player_Frame_15.png"]
            .unwrap_mapping()
            .index_source()
            .id()
            == 15
    );

    assert!(tree.root()["Assets"].unwrap_dir().len() == 20);
}
