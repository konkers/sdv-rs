use std::convert::{TryFrom, TryInto};

use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use roxmltree::Node;
use strum::EnumString;

use super::{
    array_of_bool, child_node, child_node_text, map_from_node, map_from_node_parse_key, Finder,
    NodeFinder,
};
use crate::common::{ObjectCategory, Point, Rect};

#[derive(Debug, EnumString, Eq, PartialEq)]
#[strum(ascii_case_insensitive)]
pub enum ObjectType {
    Unknown,
    Arch,
    Asdf,
    Basic,
    Cooking,
    Crafting,
    Fish,
    Interactive,
    Minerals,
    Quest,
    Ring,
    Seeds,
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for ObjectType {
    type Error = anyhow::Error;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        finder.convert()
    }
}

#[derive(Debug, PartialEq)]
pub struct Object {
    pub is_lost: bool,
    pub category: ObjectCategory,
    pub has_been_in_inventory: bool,
    pub name: String,
    pub parent_sheet_index: Option<i32>,
    pub initial_sheet_index: Option<i32>,
    pub current_sheet_index: Option<i32>,
    pub preserved_parent_sheet_index: Option<i32>,
    pub special_item: bool,
    pub special_variable: i32,
    pub display_name: String,
    pub name2: String,
    pub stack: i32,
    pub tile_location: Option<Point<i32>>,
    pub owner: Option<i64>,
    pub ty: ObjectType,
    pub can_be_set_down: Option<bool>,
    pub can_be_grabbed: Option<bool>,
    pub is_hoe_dirt: Option<bool>,
    pub is_spawned_object: Option<bool>,
    pub quest_item: Option<bool>,
    pub quest_id: Option<i32>,
    pub is_on: Option<bool>,
    pub fragility: Option<i32>,
    pub price: Option<i32>,
    pub edibility: Option<i32>,
    pub stack2: Option<i32>,
    pub quality: Option<i32>,
    pub big_craftable: Option<bool>,
    pub set_outdoors: Option<bool>,
    pub set_indoors: Option<bool>,
    pub ready_for_harvest: Option<bool>,
    pub show_next_index: Option<bool>,
    pub flipped: Option<bool>,
    pub has_been_picked_by_farmer: Option<bool>,
    pub is_recipe: Option<bool>,
    pub is_lamp: Option<bool>,
    pub minutes_until_ready: Option<i32>,
    pub bounding_box: Option<Rect<i32>>,
    pub scale: Option<Point<f32>>,
    pub uses: Option<i32>,
    pub destory_overnight: Option<bool>,
    pub coins: Option<i64>,
    pub items: Option<Vec<Object>>,
}

impl Object {
    pub(crate) fn from_node(node: &Node) -> Result<Object> {
        let items = match node.child("items").node().ok() {
            Some(node) => {
                let mut items = Vec::new();
                for item in node.children().filter(|n| n.tag_name().name() == "Item") {
                    let object = Object::from_node(&item).map_err(|e| {
                        let nodes: Vec<_> = item
                            .children()
                            .map(|n| (n.tag_name().name(), n.text()))
                            .collect();
                        anyhow!("error parsing object: {:#?}: {}", nodes, e)
                    })?;
                    items.push(object);
                }
                Some(items)
            }
            None => None,
        };

        Ok(Object {
            is_lost: node.child("isLostItem").try_into()?,
            category: node.child("category").try_into()?,
            has_been_in_inventory: node.child("hasBeenInInventory").try_into()?,
            name: node.child("name").try_into()?,
            parent_sheet_index: node.child("parentSheetIndex").try_into().ok(),
            initial_sheet_index: node.child("initialParentSheetIndex").try_into().ok(),
            current_sheet_index: node.child("currentParentSheetIndex").try_into().ok(),
            special_item: node.child("specialItem").try_into()?,
            special_variable: node.child("SpecialVariable").try_into()?,
            display_name: node.child("DisplayName").try_into()?,
            name2: node.child("Name").try_into()?,
            stack: node.child("Stack").try_into()?,
            tile_location: node.child("tileLocation").try_into().ok(),
            owner: node.child("owner").try_into().ok(),
            ty: node.child("type").try_into().unwrap_or(ObjectType::Unknown),
            can_be_set_down: node.child("canBeSetDown").try_into().ok(),
            can_be_grabbed: node.child("canBeGrabbed").try_into().ok(),
            is_hoe_dirt: node.child("isHoedirt").try_into().ok(),
            is_spawned_object: node.child("isSpawnedObject").try_into().ok(),
            quest_item: node.child("questItem").try_into().ok(),
            quest_id: node.child("questId").try_into().ok(),
            is_on: node.child("isOn").try_into().ok(),
            fragility: node.child("fragility").try_into().ok(),
            price: node.child("price").try_into().ok(),
            edibility: node.child("edibility").try_into().ok(),
            stack2: node.child("stack").try_into().ok(),
            quality: node.child("quality").try_into().ok(),
            big_craftable: node.child("bigCraftable").try_into().ok(),
            set_outdoors: node.child("setOutdoors").try_into().ok(),
            set_indoors: node.child("setIndoors").try_into().ok(),
            ready_for_harvest: node.child("readyForHarvest").try_into().ok(),
            show_next_index: node.child("showNextIndex").try_into().ok(),
            flipped: node.child("flipped").try_into().ok(),
            has_been_picked_by_farmer: node.child("hasBeenPickedUpByFarmer").try_into().ok(),
            is_recipe: node.child("isRecipe").try_into().ok(),
            is_lamp: node.child("isLamp").try_into().ok(),
            minutes_until_ready: node.child("minutesUntilReady").try_into().ok(),
            bounding_box: node.child("boundingBox").try_into().ok(),
            scale: node.child("scale").try_into().ok(),
            uses: node.child("uses").try_into().ok(),
            preserved_parent_sheet_index: node.child("preservedParentSheetIndex").try_into().ok(),
            destory_overnight: node.child("destroyOvernight").try_into().ok(),
            coins: node.child("coins").try_into().ok(),
            items,
        })
    }
}

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
        let objects = map_from_node_parse_key(
            &node.child("objects").try_into()?,
            |node| node.child("Vector2").try_into(),
            |node| Object::from_node(&node.child("Object").try_into()?),
        );

        Ok(Location {
            name,
            bundles,
            objects: objects?,
        })
    }
}
