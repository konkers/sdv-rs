use anyhow::{anyhow, Result};
use indexmap::IndexMap;
use nom::{multi::many1, IResult};
use serde::{Deserialize, Serialize};
use std::path::Path;

use super::{decimal, field, field_value, sub_field, sub_field_value};
use crate::common::{ItemType, ObjectOrCategory};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum UnlockCondition {
    Friendship { npc: String, hearts: i32 },
    Level { level: i32 },
    Skill { skill: String, level: i32 },
    Default,
    None,
}

impl UnlockCondition {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, first) = sub_field(i)?;
        match first {
            "f" => Self::parse_friendship(i),
            "l" => Self::parse_level(i),
            "s" => Self::parse_skill_level(i),
            "Combat" => Self::parse_alt_skill_level("Combat", i),
            "Farming" => Self::parse_alt_skill_level("Farming", i),
            "Fishing" => Self::parse_alt_skill_level("Fishing", i),
            "Foraging" => Self::parse_alt_skill_level("Foraging", i),
            "Luck" => Self::parse_alt_skill_level("Luck", i),
            "Mining" => Self::parse_alt_skill_level("Mining", i),
            "default" => Ok((i, Self::Default)),
            _ => Ok((i, Self::None)),
        }
    }

    fn parse_friendship(i: &str) -> IResult<&str, Self> {
        let (i, npc) = sub_field(i)?;
        let (i, hearts) = sub_field_value(decimal)(i)?;

        Ok((
            i,
            Self::Friendship {
                npc: npc.to_string(),
                hearts,
            },
        ))
    }

    fn parse_level(i: &str) -> IResult<&str, Self> {
        let (i, level) = sub_field_value(decimal)(i)?;

        Ok((i, Self::Level { level }))
    }

    fn parse_skill_level(i: &str) -> IResult<&str, Self> {
        let (i, skill) = sub_field(i)?;
        let (i, level) = sub_field_value(decimal)(i)?;

        Ok((
            i,
            Self::Skill {
                skill: skill.to_string(),
                level,
            },
        ))
    }

    fn parse_alt_skill_level<'a>(skill: &str, i: &'a str) -> IResult<&'a str, Self> {
        let (i, level) = sub_field_value(decimal)(i)?;

        Ok((
            i,
            Self::Skill {
                skill: skill.to_string(),
                level,
            },
        ))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Ingredient {
    pub item: ObjectOrCategory,
    pub quantity: i32,
}

impl Ingredient {
    fn parse(i: &str) -> IResult<&str, Self> {
        let (i, item) = sub_field_value(ObjectOrCategory::parse)(i)?;
        let (i, quantity) = sub_field_value(decimal)(i)?;
        Ok((i, Self { item, quantity }))
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Recipe {
    #[serde(skip)]
    pub name: String,
    pub ingredients: Vec<Ingredient>,
    pub yield_item: String,
    pub yield_quantity: i32,
    pub unlock_condition: UnlockCondition,
}

impl Recipe {
    fn load<P: AsRef<Path>>(file: P, craftable: bool) -> Result<IndexMap<String, Self>> {
        let data = std::fs::read(file)?;
        let entries: IndexMap<String, String> = xnb::from_bytes(&data)?;
        let mut recipies = IndexMap::new();

        for (k, v) in &entries {
            let (_, recipe) = if craftable {
                Self::parse_crafting(k, v)
                    .map_err(|e| anyhow!("Error parsing recipe \"{v}\": {e}"))?
            } else {
                Self::parse_cooking(k, v)
                    .map_err(|e| anyhow!("Error parsing recipe \"{v}\": {e}"))?
            };
            recipies.insert(k.clone(), recipe.clone());
        }

        Ok(recipies)
    }

    pub fn load_crafting<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, Self>> {
        Self::load(file, true)
    }

    pub fn load_cooking<P: AsRef<Path>>(file: P) -> Result<IndexMap<String, Self>> {
        Self::load(file, false)
    }

    fn parse_impl<'a>(name: &str, has_big_craftables: bool, i: &'a str) -> IResult<&'a str, Self> {
        let (i, ingredients) = field_value(many1(Ingredient::parse))(i)?;
        let (i, _) = field(i)?;
        let (i, yield_field) = field(i)?;
        let (yield_field, yield_item) = sub_field(yield_field)?;
        let yield_quantity = sub_field_value(decimal)(yield_field)
            .map(|(_, v)| v)
            .unwrap_or(1);

        let (i, prefix) = if has_big_craftables {
            let (i, is_big_craftable) = field(i)?;

            let prefix = match is_big_craftable {
                "true" => ItemType::BigCraftable.prefix(),
                "Ring" | "false" => ItemType::Object.prefix(),
                _ => todo!(),
            };

            (i, prefix)
        } else {
            (i, ItemType::Object.prefix())
        };

        let yield_item = format!("{prefix}{yield_item}");
        let (i, unlock_condition) = field_value(UnlockCondition::parse)(i)?;

        Ok((
            i,
            Self {
                name: name.to_string(),
                ingredients,
                yield_item,
                yield_quantity,
                unlock_condition,
            },
        ))
    }

    fn parse_cooking<'a>(name: &str, i: &'a str) -> IResult<&'a str, Self> {
        Self::parse_impl(name, false, i)
    }

    fn parse_crafting<'a>(name: &str, i: &'a str) -> IResult<&'a str, Self> {
        Self::parse_impl(name, true, i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn friendship_condition_parses() {
        assert_eq!(
            UnlockCondition::parse("f Emily 3"),
            Ok((
                "",
                UnlockCondition::Friendship {
                    npc: "Emily".to_string(),
                    hearts: 3
                }
            ))
        );
    }

    #[test]
    fn level_condition_parses() {
        assert_eq!(
            UnlockCondition::parse("l 18"),
            Ok(("", UnlockCondition::Level { level: 18 }))
        );
    }

    #[test]
    fn skill_level_conditions_parse() {
        assert_eq!(
            UnlockCondition::parse("s Combat 1"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Combat".to_string(),
                    level: 1
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("s Farming 2"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Farming".to_string(),
                    level: 2
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("s Fishing 3"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Fishing".to_string(),
                    level: 3
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("s Foraging 4"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Foraging".to_string(),
                    level: 4
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("s Luck 5"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Luck".to_string(),
                    level: 5
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("s Mining 6"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Mining".to_string(),
                    level: 6
                }
            ))
        );
    }

    #[test]
    fn alt_skill_level_conditions_parse() {
        assert_eq!(
            UnlockCondition::parse("Combat 1"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Combat".to_string(),
                    level: 1
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("Farming 2"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Farming".to_string(),
                    level: 2
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("Fishing 3"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Fishing".to_string(),
                    level: 3
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("Foraging 4"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Foraging".to_string(),
                    level: 4
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("Luck 5"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Luck".to_string(),
                    level: 5
                }
            ))
        );
        assert_eq!(
            UnlockCondition::parse("Mining 6"),
            Ok((
                "",
                UnlockCondition::Skill {
                    skill: "Mining".to_string(),
                    level: 6
                }
            ))
        );
    }

    #[test]
    fn default_condition_parses() {
        assert_eq!(
            UnlockCondition::parse("default"),
            Ok(("", UnlockCondition::Default))
        );
    }

    #[test]
    fn null_condition_parses() {
        assert_eq!(
            UnlockCondition::parse("null"),
            Ok(("", UnlockCondition::None))
        );
    }

    #[test]
    fn cooking_recipe_parses() {
        assert_eq!(
            Recipe::parse_cooking("Complete Breakfast", "194 1 -6 1 210 1 211 1/2 2/201/l 26"),
            Ok((
                "",
                Recipe {
                    name: "Complete Breakfast".to_string(),
                    ingredients: vec![
                        Ingredient {
                            item: ObjectOrCategory::Item("194".to_string()),
                            quantity: 1,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Category(crate::common::ObjectCategory::Milk),
                            quantity: 1,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("210".to_string()),
                            quantity: 1,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("211".to_string()),
                            quantity: 1,
                        },
                    ],
                    yield_item: "(O)201".to_string(),
                    yield_quantity: 1,
                    unlock_condition: UnlockCondition::Level { level: 26 },
                }
            ))
        );
    }

    #[test]
    fn cooking_recipe_with_non_integer_id_parses() {
        assert_eq!(
            Recipe::parse_cooking("Moss Soup", "Moss 20/1 10/MossSoup/s Foraging 3/"),
            Ok((
                "",
                Recipe {
                    name: "Moss Soup".to_string(),
                    ingredients: vec![Ingredient {
                        item: ObjectOrCategory::Item("Moss".to_string()),
                        quantity: 20,
                    },],
                    yield_item: "(O)MossSoup".to_string(),
                    yield_quantity: 1,
                    unlock_condition: UnlockCondition::Skill {
                        skill: "Foraging".to_string(),
                        level: 3
                    },
                }
            ))
        );
    }

    #[test]
    fn crafting_recipe_parses() {
        assert_eq!(
            Recipe::parse_crafting(
                "Cork Bobber",
                "388 10 709 5 766 10/Home/695/false/Fishing 7"
            ),
            Ok((
                "",
                Recipe {
                    name: "Cork Bobber".to_string(),
                    ingredients: vec![
                        Ingredient {
                            item: ObjectOrCategory::Item("388".to_string()),
                            quantity: 10,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("709".to_string()),
                            quantity: 5,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("766".to_string()),
                            quantity: 10,
                        },
                    ],
                    yield_item: "(O)695".to_string(),
                    yield_quantity: 1,
                    unlock_condition: UnlockCondition::Skill {
                        skill: "Fishing".to_string(),
                        level: 7
                    },
                }
            ))
        );
    }

    #[test]
    fn big_crafting_recipe_parses() {
        assert_eq!(
            Recipe::parse_crafting(
                "Crystalarium",
                "390 99 336 5 337 2 787 1/Home/21/true/Mining 9"
            ),
            Ok((
                "",
                Recipe {
                    name: "Crystalarium".to_string(),
                    ingredients: vec![
                        Ingredient {
                            item: ObjectOrCategory::Item("390".to_string()),
                            quantity: 99,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("336".to_string()),
                            quantity: 5,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("337".to_string()),
                            quantity: 2,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("787".to_string()),
                            quantity: 1,
                        },
                    ],
                    yield_item: "(BC)21".to_string(),
                    yield_quantity: 1,
                    unlock_condition: UnlockCondition::Skill {
                        skill: "Mining".to_string(),
                        level: 9
                    },
                }
            ))
        );
    }

    #[test]
    fn multi_quantity_recipe_parses() {
        assert_eq!(
            Recipe::parse_crafting("Magic Bait", "909 1 684 3/Home/908 5/false/null"),
            Ok((
                "",
                Recipe {
                    name: "Magic Bait".to_string(),
                    ingredients: vec![
                        Ingredient {
                            item: ObjectOrCategory::Item("909".to_string()),
                            quantity: 1,
                        },
                        Ingredient {
                            item: ObjectOrCategory::Item("684".to_string()),
                            quantity: 3,
                        },
                    ],
                    yield_item: "(O)908".to_string(),
                    yield_quantity: 5,
                    unlock_condition: UnlockCondition::None,
                }
            ))
        );
    }
}
