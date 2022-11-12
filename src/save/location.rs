use std::convert::TryInto;

use anyhow::Result;
use indexmap::IndexMap;
use roxmltree::Node;

use super::{array_of_bool, child_node, child_node_text, map_from_node, object::Object, Finder};

use crate::common::Point;

#[derive(Debug)]
pub struct Location {
    pub name: String,
    pub bundles: Option<IndexMap<i32, Vec<bool>>>,
    pub objects: IndexMap<Point<i32>, Object>,
}

impl Location {
    pub(crate) fn from_node(node: &Node) -> Result<Location> {
        let name = child_node_text(node, "name")?;
        let bundles = match child_node(node, "bundles").ok() {
            Some(n) => Some(map_from_node(&n, "int", array_of_bool)?),
            None => None,
        };
        let objects = map_from_node(&node.child("objects").try_into()?, "Vector2", |node| {
            Object::from_node(&node.child("Object").try_into()?)
        });

        Ok(Location {
            name,
            bundles,
            objects: objects?,
        })
    }
}
