use indexmap::IndexMap;
use roxmltree::Node;
use std::convert::TryInto;

use super::{array_of_bool, map_from_node, object::Object, Finder, SaveResult};

use crate::common::Point;

#[derive(Debug)]
pub struct Location {
    pub name: String,
    pub bundles: Option<IndexMap<i32, Vec<bool>>>,
    pub objects: IndexMap<Point<i32>, Object>,
}

impl Location {
    pub(crate) fn from_node<'a, 'input: 'a>(
        node: Node<'a, 'input>,
    ) -> SaveResult<'a, 'input, Location> {
        let name = node.child("name").try_into()?;
        let bundles = match node.child("bundles").try_into().ok() {
            Some(n) => Some(map_from_node(n, "int", array_of_bool)?),
            None => None,
        };
        let objects = map_from_node(node.child("objects").try_into()?, "Vector2", |node| {
            Object::from_node(node.child("Object").try_into()?)
        });

        Ok(Location {
            name,
            bundles,
            objects: objects?,
        })
    }
}
