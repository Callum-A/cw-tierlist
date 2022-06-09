use crate::state::{Tierlist, TierlistItem, TierlistTemplate};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub admin_address: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    CreateTemplate {
        title: String,
        items: Vec<TierlistItem>,
    },
    DeleteTemplate {
        id: u64,
    },
    EditTemplate {
        id: u64,
        title: String,
        items: Vec<TierlistItem>,
    },
    SaveTierlist {
        tierlist: Tierlist,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {
    Config {},
    Template {
        id: u64,
    },
    TierlistFromTemplate {
        id: u64,
    },
    Tierlist {
        address: String,
        id: u64,
    },
    TierlistsByAddress {
        address: String,
        start_after: Option<u64>,
        limit: Option<u32>,
    },
    Templates {
        start_after: Option<u64>,
        limit: Option<u32>,
    },
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TemplateResponse {
    pub template: Option<TierlistTemplate>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct TierlistResponse {
    pub tierlist: Option<Tierlist>,
}
