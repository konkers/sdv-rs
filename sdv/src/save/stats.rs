use roxmltree::Node;
use std::convert::{TryFrom, TryInto};

use super::{Finder, NodeFinder, SaveError, SaveResult};

#[derive(Debug)]
pub struct Stats {
    pub seeds_sown: u32,
    pub items_shipped: u32,
    pub items_cooked: u32,
    pub items_crafted: u32,
    pub chicken_eggs_layed: u32,
    pub duck_eggs_layed: u32,
    pub cow_milk_produced: u32,
    pub goat_milk_produced: u32,
    pub rabbit_wool_produced: u32,
    pub sheep_wool_produced: u32,
    pub cheese_made: u32,
    pub goat_cheese_made: u32,
    pub truffles_found: u32,
    pub stone_gathered: u32,
    pub rocks_crushed: u32,
    pub dirt_hoed: u32,
    pub gifts_given: u32,
    pub times_unconscious: u32,
    pub average_bedtime: u32,
    pub times_fished: u32,
    pub fish_caught: u32,
    pub boulders_cracked: u32,
    pub stumps_chopped: u32,
    pub steps_taken: u32,
    pub monsters_killed: u32,
    pub diamonds_found: u32,
    pub prismatic_shards_found: u32,
    pub other_precious_gems_found: u32,
    pub cave_carrots_found: u32,
    pub copper_found: u32,
    pub iron_found: u32,
    pub coal_found: u32,
    pub coins_found: u32,
    pub gold_found: u32,
    pub iridium_found: u32,
    pub bars_smelted: u32,
    pub beverages_made: u32,
    pub preserves_made: u32,
    pub pieces_of_trash_recycled: u32,
    pub mystic_stones_crushed: u32,
    pub days_played: u32,
    pub weeds_eliminated: u32,
    pub sticks_chopped: u32,
    pub notes_found: u32,
    pub quests_completed: u32,
    pub star_level_crops_shipped: u32,
    pub crops_shipped: u32,
    pub items_foraged: u32,
    pub slimes_killed: u32,
    pub geodes_cracked: u32,
    pub good_friends: u32,
    pub total_money_gifted: u32,
    pub individual_money_earned: u32,
}

impl Stats {
    fn from_node<'a, 'input: 'a>(node: Node<'a, 'input>) -> SaveResult<'a, 'input, Stats> {
        Ok(Stats {
            seeds_sown: node.child("seedsSown").try_into().unwrap_or_default(),
            items_shipped: node.child("itemsShipped").try_into().unwrap_or_default(),
            items_cooked: node.child("itemsCooked").try_into().unwrap_or_default(),
            items_crafted: node.child("itemsCrafted").try_into().unwrap_or_default(),
            chicken_eggs_layed: node
                .child("chickenEggsLayed")
                .try_into()
                .unwrap_or_default(),
            duck_eggs_layed: node.child("duckEggsLayed").try_into().unwrap_or_default(),
            cow_milk_produced: node.child("cowMilkProduced").try_into().unwrap_or_default(),
            goat_milk_produced: node
                .child("goatMilkProduced")
                .try_into()
                .unwrap_or_default(),
            rabbit_wool_produced: node
                .child("rabbitWoolProduced")
                .try_into()
                .unwrap_or_default(),
            sheep_wool_produced: node
                .child("sheepWoolProduced")
                .try_into()
                .unwrap_or_default(),
            cheese_made: node.child("cheeseMade").try_into().unwrap_or_default(),
            goat_cheese_made: node.child("goatCheeseMade").try_into().unwrap_or_default(),
            truffles_found: node.child("trufflesFound").try_into().unwrap_or_default(),
            stone_gathered: node.child("stoneGathered").try_into().unwrap_or_default(),
            rocks_crushed: node.child("rocksCrushed").try_into().unwrap_or_default(),
            dirt_hoed: node.child("dirtHoed").try_into().unwrap_or_default(),
            gifts_given: node.child("giftsGiven").try_into().unwrap_or_default(),
            times_unconscious: node
                .child("timesUnconscious")
                .try_into()
                .unwrap_or_default(),
            average_bedtime: node.child("averageBedtime").try_into().unwrap_or_default(),
            times_fished: node.child("timesFished").try_into().unwrap_or_default(),
            fish_caught: node.child("fishCaught").try_into().unwrap_or_default(),
            boulders_cracked: node.child("bouldersCracked").try_into().unwrap_or_default(),
            stumps_chopped: node.child("stumpsChopped").try_into().unwrap_or_default(),
            steps_taken: node.child("stepsTaken").try_into().unwrap_or_default(),
            monsters_killed: node.child("monstersKilled").try_into().unwrap_or_default(),
            diamonds_found: node.child("diamondsFound").try_into().unwrap_or_default(),
            prismatic_shards_found: node
                .child("prismaticShardsFound")
                .try_into()
                .unwrap_or_default(),
            other_precious_gems_found: node
                .child("otherPreciousGemsFound")
                .try_into()
                .unwrap_or_default(),
            cave_carrots_found: node
                .child("caveCarrotsFound")
                .try_into()
                .unwrap_or_default(),
            copper_found: node.child("copperFound").try_into().unwrap_or_default(),
            iron_found: node.child("ironFound").try_into().unwrap_or_default(),
            coal_found: node.child("coalFound").try_into().unwrap_or_default(),
            coins_found: node.child("coinsFound").try_into().unwrap_or_default(),
            gold_found: node.child("goldFound").try_into().unwrap_or_default(),
            iridium_found: node.child("iridiumFound").try_into().unwrap_or_default(),
            bars_smelted: node.child("barsSmelted").try_into().unwrap_or_default(),
            beverages_made: node.child("beveragesMade").try_into().unwrap_or_default(),
            preserves_made: node.child("preservesMade").try_into().unwrap_or_default(),
            pieces_of_trash_recycled: node
                .child("piecesOfTrashRecycled")
                .try_into()
                .unwrap_or_default(),
            mystic_stones_crushed: node
                .child("mysticStonesCrushed")
                .try_into()
                .unwrap_or_default(),
            days_played: node.child("daysPlayed").try_into().unwrap_or_default(),
            weeds_eliminated: node.child("weedsEliminated").try_into().unwrap_or_default(),
            sticks_chopped: node.child("sticksChopped").try_into().unwrap_or_default(),
            notes_found: node.child("notesFound").try_into().unwrap_or_default(),
            quests_completed: node.child("questsCompleted").try_into().unwrap_or_default(),
            star_level_crops_shipped: node
                .child("starLevelCropsShipped")
                .try_into()
                .unwrap_or_default(),
            crops_shipped: node.child("cropsShipped").try_into().unwrap_or_default(),
            items_foraged: node.child("itemsForaged").try_into().unwrap_or_default(),
            slimes_killed: node.child("slimesKilled").try_into().unwrap_or_default(),
            geodes_cracked: node.child("geodesCracked").try_into().unwrap_or_default(),
            good_friends: node.child("goodFriends").try_into().unwrap_or_default(),
            total_money_gifted: node
                .child("totalMoneyGifted")
                .try_into()
                .unwrap_or_default(),
            individual_money_earned: node
                .child("individualMoneyEarned")
                .try_into()
                .unwrap_or_default(),
        })
    }
}

impl<'a, 'input: 'a> TryFrom<NodeFinder<'a, 'input>> for Stats {
    type Error = SaveError<'a, 'input>;
    fn try_from(finder: NodeFinder<'a, 'input>) -> Result<Self, Self::Error> {
        Self::from_node(finder.node()?)
    }
}
