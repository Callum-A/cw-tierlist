use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub admin_address: String,
}

/// Tierlist item having a name and an optional image
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TierlistItem {
    pub name: String,
    pub image_url: Option<String>,
}

/// Tierlist template AKA providing the name and the items the people tier.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct TierlistTemplate {
    pub id: u64,
    pub title: String,
    pub items: Vec<TierlistItem>,
    pub creator: String,
}

/// A tierlist a user is completing
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, JsonSchema)]
pub struct Tierlist {
    pub template_id: u64,
    pub items_to_tiers: Vec<(TierlistItem, String)>, // Unassigned items point to a blank string
}

impl Tierlist {
    pub fn from_template(template: TierlistTemplate) -> Tierlist {
        let mut items_to_tiers: Vec<(TierlistItem, String)> = vec![];
        for item in template.items {
            items_to_tiers.push((item.clone(), "".to_string()));
        }
        Tierlist {
            items_to_tiers,
            template_id: template.id,
        }
    }

    pub fn validate_against_template(self, template: TierlistTemplate) -> bool {
        if self.template_id != template.id {
            return false;
        }

        let mut items: Vec<_> = self
            .items_to_tiers
            .iter()
            .map(|i| -> TierlistItem { i.clone().0 })
            .collect();
        let mut template_items = template.items;
        items.sort_by(|a, b| a.name.cmp(&b.name));
        template_items.sort_by(|a, b| a.name.cmp(&b.name));
        items == template_items
    }

    pub fn assign(&mut self, item: TierlistItem, tier: String) {
        let it: Vec<_> = self
            .items_to_tiers
            .iter()
            .map(|i| -> (TierlistItem, String) {
                let cloned_item = item.clone();
                let cloned_tier = tier.clone();
                if i.0 == cloned_item {
                    (cloned_item, cloned_tier)
                } else {
                    (cloned_item, i.clone().1)
                }
            })
            .collect();
        self.items_to_tiers = it;
    }

    pub fn get_tier(&self, item: TierlistItem) -> String {
        let idx = self
            .items_to_tiers
            .iter()
            .position(|i| i.0 == item)
            .unwrap();
        self.items_to_tiers[idx].1.clone()
    }
}

/// General config
pub const CONFIG: Item<Config> = Item::new("config");

/// ID helper for tierlists
pub const NEXT_ID: Item<u64> = Item::new("next_id");

/// Allows people to make templates for others to use.
pub const TIERLIST_TEMPLATES: Map<u64, TierlistTemplate> = Map::new("tierlist_templates");

/// Peoples in progress and complete tierlists
pub const TIERLISTS: Map<(String, u64), Tierlist> = Map::new("tierlists");

#[cfg(test)]
mod tests {
    use crate::state::{Tierlist, TierlistItem, TierlistTemplate};

    pub const ADDR1: &str = "addr1";

    fn make_items() -> Vec<TierlistItem> {
        vec![
            TierlistItem {
                name: "A".to_string(),
                image_url: None,
            },
            TierlistItem {
                name: "B".to_string(),
                image_url: None,
            },
            TierlistItem {
                name: "C".to_string(),
                image_url: None,
            },
        ]
    }

    fn make_tiered_items() -> Vec<(TierlistItem, String)> {
        vec![
            (
                TierlistItem {
                    name: "A".to_string(),
                    image_url: None,
                },
                "".to_string(),
            ),
            (
                TierlistItem {
                    name: "B".to_string(),
                    image_url: None,
                },
                "".to_string(),
            ),
            (
                TierlistItem {
                    name: "C".to_string(),
                    image_url: None,
                },
                "".to_string(),
            ),
        ]
    }

    fn make_template() -> TierlistTemplate {
        TierlistTemplate {
            id: 0,
            title: "Some tierlist".to_string(),
            items: make_items(),
            creator: ADDR1.to_string(),
        }
    }

    #[test]
    fn test_from_template() {
        let template = make_template();
        let populated = Tierlist::from_template(template.clone());
        assert_eq!(populated.template_id, template.id);
        assert_eq!(populated.items_to_tiers.len(), template.items.len());
    }

    #[test]
    fn test_validate_against_template() {
        let template = make_template();
        // Valid
        let populated = Tierlist::from_template(template.clone());
        assert!(populated.validate_against_template(template.clone()));

        // Mismatched IDs
        let corrupted = Tierlist {
            template_id: 1,
            items_to_tiers: make_tiered_items(),
        };
        assert!(!corrupted.validate_against_template(template.clone()));

        // Item missing
        let corrupted = Tierlist {
            template_id: 1,
            items_to_tiers: vec![
                (
                    TierlistItem {
                        name: "A".to_string(),
                        image_url: None,
                    },
                    "".to_string(),
                ),
                (
                    TierlistItem {
                        name: "B".to_string(),
                        image_url: None,
                    },
                    "".to_string(),
                ),
            ],
        };
        assert!(!corrupted.validate_against_template(template.clone()));

        // Item added
        let corrupted = Tierlist {
            template_id: 1,
            items_to_tiers: vec![
                (
                    TierlistItem {
                        name: "A".to_string(),
                        image_url: None,
                    },
                    "".to_string(),
                ),
                (
                    TierlistItem {
                        name: "B".to_string(),
                        image_url: None,
                    },
                    "".to_string(),
                ),
                (
                    TierlistItem {
                        name: "C".to_string(),
                        image_url: None,
                    },
                    "".to_string(),
                ),
                (
                    TierlistItem {
                        name: "D".to_string(),
                        image_url: None,
                    },
                    "".to_string(),
                ),
            ],
        };
        assert!(!corrupted.validate_against_template(template))
    }

    #[test]
    fn test_assign() {
        let template = make_template();
        let mut populated = Tierlist::from_template(template);
        let item = TierlistItem {
            name: "A".to_string(),
            image_url: None,
        };

        // Blank for no tier
        assert_eq!(populated.get_tier(item.clone()), "".to_string());

        // Initial assign
        populated.assign(item.clone(), "S".to_string());
        assert_eq!(populated.get_tier(item.clone()), "S".to_string());

        // Edit
        populated.assign(item.clone(), "A".to_string());
        assert_eq!(populated.get_tier(item.clone()), "A".to_string());

        // Remove
        populated.assign(item.clone(), "".to_string());
        assert_eq!(populated.get_tier(item), "".to_string());
    }
}
