#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};
use cw2::set_contract_version;
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TemplateResponse, TierlistResponse};
use crate::state::{
    Config, Tierlist, TierlistItem, TierlistTemplate, CONFIG, NEXT_ID, TIERLISTS,
    TIERLIST_TEMPLATES,
};

const DEFAULT_LIMIT: u32 = 10;

// version info for migration info
const CONTRACT_NAME: &str = "crates.io:cw-tierlist";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    deps.api.addr_validate(&msg.admin_address)?;
    let config = Config {
        admin_address: msg.admin_address.clone(),
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("admin", msg.admin_address))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::CreateTemplate { title, items } => {
            execute_create_template(deps, env, info, title, items)
        }
        ExecuteMsg::DeleteTemplate { id } => execute_delete_template(deps, env, info, id),
        ExecuteMsg::EditTemplate { id, title, items } => {
            execute_edit_template(deps, env, info, id, title, items)
        }
        ExecuteMsg::SaveTierlist { tierlist } => execute_save_tierlist(deps, env, info, tierlist),
    }
}

pub fn execute_create_template(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    title: String,
    items: Vec<TierlistItem>,
) -> Result<Response, ContractError> {
    let id = NEXT_ID.may_load(deps.storage)?.unwrap_or_default();
    NEXT_ID.save(deps.storage, &(id + 1))?;

    let template = TierlistTemplate {
        id,
        title,
        items,
        creator: info.sender.to_string(),
    };
    TIERLIST_TEMPLATES.save(deps.storage, id, &template)?;
    Ok(Response::new())
}

pub fn execute_delete_template(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: u64,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let template = TIERLIST_TEMPLATES.load(deps.storage, id)?;
    if info.sender != template.creator && info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    TIERLIST_TEMPLATES.remove(deps.storage, id);
    Ok(Response::new())
}

pub fn execute_edit_template(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    id: u64,
    title: String,
    items: Vec<TierlistItem>,
) -> Result<Response, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let existing_template = TIERLIST_TEMPLATES.load(deps.storage, id)?;
    if info.sender != existing_template.creator && info.sender != config.admin_address {
        return Err(ContractError::Unauthorized {});
    }

    let template = TierlistTemplate {
        id,
        title,
        items,
        creator: existing_template.creator,
    };
    TIERLIST_TEMPLATES.save(deps.storage, id, &template)?;
    Ok(Response::new())
}

pub fn execute_save_tierlist(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    tierlist: Tierlist,
) -> Result<Response, ContractError> {
    let template = TIERLIST_TEMPLATES.load(deps.storage, tierlist.template_id)?;
    let id = tierlist.template_id;
    let valid = tierlist.clone().validate_against_template(template);
    if !valid {
        return Err(ContractError::InvalidTierlist {});
    }

    TIERLISTS.save(deps.storage, (info.sender.to_string(), id), &tierlist)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_binary(&CONFIG.load(deps.storage)?),
        QueryMsg::Template { id } => query_template(deps, id),
        QueryMsg::TierlistFromTemplate { id } => query_tierlist_from_template(deps, id),
        QueryMsg::Tierlist { address, id } => query_tierlist(deps, address, id),
        QueryMsg::TierlistsByAddress {
            address,
            start_after,
            limit,
        } => query_tierlists_by_address(deps, address, start_after, limit),
        QueryMsg::Templates { start_after, limit } => {
            query_tierlist_templates(deps, start_after, limit)
        }
    }
}

pub fn query_template(deps: Deps, id: u64) -> StdResult<Binary> {
    let template = TIERLIST_TEMPLATES.may_load(deps.storage, id)?;
    to_binary(&TemplateResponse { template })
}

pub fn query_tierlist_from_template(deps: Deps, id: u64) -> StdResult<Binary> {
    let template = TIERLIST_TEMPLATES.may_load(deps.storage, id)?;
    match template {
        None => to_binary(&TierlistResponse { tierlist: None }),
        Some(template) => to_binary(&TierlistResponse {
            tierlist: Some(Tierlist::from_template(template)),
        }),
    }
}

pub fn query_tierlist(deps: Deps, address: String, id: u64) -> StdResult<Binary> {
    deps.api.addr_validate(&address).unwrap(); // Validate address
    let tierlist = TIERLISTS.may_load(deps.storage, (address, id))?;
    match tierlist {
        None => to_binary(&TierlistResponse { tierlist: None }),
        Some(tierlist) => to_binary(&TierlistResponse {
            tierlist: Some(tierlist),
        }),
    }
}

pub fn query_tierlist_templates(
    deps: Deps,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    let min = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let tierlists: Vec<_> = TIERLIST_TEMPLATES
        .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
        .take(limit as usize)
        .collect::<Result<Vec<(u64, TierlistTemplate)>, _>>()?;
    to_binary(&tierlists)
}

pub fn query_tierlists_by_address(
    deps: Deps,
    address: String,
    start_after: Option<u64>,
    limit: Option<u32>,
) -> StdResult<Binary> {
    deps.api.addr_validate(&address).unwrap(); // Validate address
    let min = start_after.map(Bound::exclusive);
    let limit = limit.unwrap_or(DEFAULT_LIMIT);
    let tierlists: Vec<_> = TIERLISTS
        .prefix(address)
        .range(deps.storage, min, None, cosmwasm_std::Order::Ascending)
        .take(limit as usize)
        .collect::<Result<Vec<(u64, Tierlist)>, _>>()?;
    to_binary(&tierlists)
}

#[cfg(test)]
mod tests {
    use crate::contract::{execute, instantiate, query};
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg, TemplateResponse, TierlistResponse};
    use crate::state::{Config, Tierlist, TierlistItem, TierlistTemplate};
    use cosmwasm_std::from_binary;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    pub const ADDR1: &str = "addr1";
    pub const ADDR2: &str = "addr2";

    #[test]
    fn test_instantiate() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let info = mock_info(ADDR1, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info,
            InstantiateMsg {
                admin_address: ADDR1.to_string(),
            },
        )
        .unwrap();

        let bin = query(deps.as_ref(), env, QueryMsg::Config {}).unwrap();
        let config: Config = from_binary(&bin).unwrap();
        assert_eq!(
            config,
            Config {
                admin_address: ADDR1.to_string()
            }
        );
    }

    #[test]
    fn test_create_template() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let info = mock_info(ADDR1, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin_address: ADDR1.to_string(),
            },
        )
        .unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 1".to_string(),
            items: vec![
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
            ],
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = QueryMsg::Template { id: 0 };
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let template: TemplateResponse = from_binary(&bin).unwrap();
        assert_eq!(
            template.template,
            Some(TierlistTemplate {
                id: 0,
                title: "Tierlist 1".to_string(),
                items: vec![
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
                ],
                creator: ADDR1.to_string()
            })
        );
    }

    #[test]
    fn test_edit_template() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let info = mock_info(ADDR1, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin_address: ADDR1.to_string(),
            },
        )
        .unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 1".to_string(),
            items: vec![
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
            ],
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::EditTemplate {
            id: 0,
            title: "NewTitle".to_string(),
            items: vec![
                TierlistItem {
                    name: "A".to_string(),
                    image_url: None,
                },
                TierlistItem {
                    name: "C".to_string(),
                    image_url: None,
                },
            ],
        };
        // Try and edit as non admin non owner
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(ADDR2, &[]),
            msg.clone(),
        )
        .unwrap_err();

        // Valid edit
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        // Query the template
        let msg = QueryMsg::Template { id: 0 };
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let template: TemplateResponse = from_binary(&bin).unwrap();
        assert_eq!(
            template.template,
            Some(TierlistTemplate {
                id: 0,
                title: "NewTitle".to_string(),
                items: vec![
                    TierlistItem {
                        name: "A".to_string(),
                        image_url: None,
                    },
                    TierlistItem {
                        name: "C".to_string(),
                        image_url: None,
                    },
                ],
                creator: ADDR1.to_string()
            })
        )
    }

    #[test]
    fn test_delete_template() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let info = mock_info(ADDR1, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin_address: ADDR1.to_string(),
            },
        )
        .unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 1".to_string(),
            items: vec![
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
            ],
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::DeleteTemplate { id: 0 };
        // Try to delete as non admin or creator
        execute(
            deps.as_mut(),
            env.clone(),
            mock_info(ADDR2, &[]),
            msg.clone(),
        )
        .unwrap_err();
        // Valid delete
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = QueryMsg::Template { id: 0 };
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let res: TemplateResponse = from_binary(&bin).unwrap();
        assert_eq!(res.template, None);
    }

    #[test]
    fn test_save_tierlist() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let info = mock_info(ADDR1, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin_address: ADDR1.to_string(),
            },
        )
        .unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 1".to_string(),
            items: vec![
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
            ],
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Valid tierlist
        let tierlist = Tierlist {
            template_id: 0,
            items_to_tiers: vec![
                (
                    TierlistItem {
                        name: "A".to_string(),
                        image_url: None,
                    },
                    "S".to_string(),
                ),
                (
                    TierlistItem {
                        name: "B".to_string(),
                        image_url: None,
                    },
                    "A".to_string(),
                ),
                (
                    TierlistItem {
                        name: "C".to_string(),
                        image_url: None,
                    },
                    "B".to_string(),
                ),
            ],
        };
        let msg = ExecuteMsg::SaveTierlist {
            tierlist: tierlist.clone(),
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        // Query for it
        let msg = QueryMsg::Tierlist {
            address: ADDR1.to_string(),
            id: 0,
        };
        let bin = query(deps.as_ref(), env.clone(), msg).unwrap();
        let res: TierlistResponse = from_binary(&bin).unwrap();
        assert_eq!(res.tierlist, Some(tierlist));

        // Query for nonexistent
        let msg = QueryMsg::Tierlist {
            address: ADDR2.to_string(),
            id: 0,
        };
        let bin = query(deps.as_ref(), env.clone(), msg).unwrap();
        let res: TierlistResponse = from_binary(&bin).unwrap();
        assert_eq!(res.tierlist, None);

        // Invalid tierlist, missing options
        let tierlist = Tierlist {
            template_id: 0,
            items_to_tiers: vec![
                (
                    TierlistItem {
                        name: "A".to_string(),
                        image_url: None,
                    },
                    "S".to_string(),
                ),
                (
                    TierlistItem {
                        name: "B".to_string(),
                        image_url: None,
                    },
                    "A".to_string(),
                ),
            ],
        };
        let msg = ExecuteMsg::SaveTierlist { tierlist };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap_err();

        // Invalid tierlist, additional options
        let tierlist = Tierlist {
            template_id: 0,
            items_to_tiers: vec![
                (
                    TierlistItem {
                        name: "A".to_string(),
                        image_url: None,
                    },
                    "S".to_string(),
                ),
                (
                    TierlistItem {
                        name: "B".to_string(),
                        image_url: None,
                    },
                    "A".to_string(),
                ),
                (
                    TierlistItem {
                        name: "C".to_string(),
                        image_url: None,
                    },
                    "A".to_string(),
                ),
                (
                    TierlistItem {
                        name: "D".to_string(),
                        image_url: None,
                    },
                    "A".to_string(),
                ),
            ],
        };
        let msg = ExecuteMsg::SaveTierlist { tierlist };
        execute(deps.as_mut(), env, info, msg).unwrap_err();
    }

    #[test]
    fn test_query_tierlists() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let info = mock_info(ADDR1, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin_address: ADDR1.to_string(),
            },
        )
        .unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 1".to_string(),
            items: vec![
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
            ],
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 2".to_string(),
            items: vec![
                TierlistItem {
                    name: "D".to_string(),
                    image_url: None,
                },
                TierlistItem {
                    name: "E".to_string(),
                    image_url: None,
                },
                TierlistItem {
                    name: "F".to_string(),
                    image_url: None,
                },
            ],
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let tierlist_1 = Tierlist::from_template(TierlistTemplate {
            id: 0,
            title: "Tierlist 1".to_string(),
            items: vec![
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
            ],
            creator: ADDR1.to_string(),
        });

        let tierlist_2 = Tierlist::from_template(TierlistTemplate {
            id: 1,
            title: "Tierlist 2".to_string(),
            items: vec![
                TierlistItem {
                    name: "D".to_string(),
                    image_url: None,
                },
                TierlistItem {
                    name: "E".to_string(),
                    image_url: None,
                },
                TierlistItem {
                    name: "F".to_string(),
                    image_url: None,
                },
            ],
            creator: ADDR1.to_string(),
        });

        let msg = ExecuteMsg::SaveTierlist {
            tierlist: tierlist_1,
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();
        let msg = ExecuteMsg::SaveTierlist {
            tierlist: tierlist_2,
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = QueryMsg::TierlistsByAddress {
            address: ADDR1.to_string(),
            start_after: None,
            limit: None,
        };
        let bin = query(deps.as_ref(), env.clone(), msg).unwrap();
        let res: Vec<(u64, Tierlist)> = from_binary(&bin).unwrap();
        assert_eq!(res.len(), 2);

        let msg = QueryMsg::TierlistsByAddress {
            address: ADDR2.to_string(),
            start_after: None,
            limit: None,
        };
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let res: Vec<(u64, Tierlist)> = from_binary(&bin).unwrap();
        assert_eq!(res.len(), 0);
    }

    #[test]
    fn test_query_templates() {
        let env = mock_env();
        let mut deps = mock_dependencies();
        let info = mock_info(ADDR1, &[]);
        instantiate(
            deps.as_mut(),
            env.clone(),
            info.clone(),
            InstantiateMsg {
                admin_address: ADDR1.to_string(),
            },
        )
        .unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 1".to_string(),
            items: vec![
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
            ],
        };
        execute(deps.as_mut(), env.clone(), info.clone(), msg).unwrap();

        let msg = ExecuteMsg::CreateTemplate {
            title: "Tierlist 2".to_string(),
            items: vec![
                TierlistItem {
                    name: "D".to_string(),
                    image_url: None,
                },
                TierlistItem {
                    name: "E".to_string(),
                    image_url: None,
                },
                TierlistItem {
                    name: "F".to_string(),
                    image_url: None,
                },
            ],
        };
        execute(deps.as_mut(), env.clone(), info, msg).unwrap();

        let msg = QueryMsg::Templates {
            start_after: None,
            limit: None,
        };
        let bin = query(deps.as_ref(), env, msg).unwrap();
        let res: Vec<(u64, TierlistTemplate)> = from_binary(&bin).unwrap();
        assert_eq!(
            res,
            vec![
                (
                    0,
                    TierlistTemplate {
                        id: 0,
                        title: "Tierlist 1".to_string(),
                        items: vec![
                            TierlistItem {
                                name: "A".to_string(),
                                image_url: None
                            },
                            TierlistItem {
                                name: "B".to_string(),
                                image_url: None
                            },
                            TierlistItem {
                                name: "C".to_string(),
                                image_url: None
                            }
                        ],
                        creator: ADDR1.to_string()
                    }
                ),
                (
                    1,
                    TierlistTemplate {
                        id: 1,
                        title: "Tierlist 2".to_string(),
                        items: vec![
                            TierlistItem {
                                name: "D".to_string(),
                                image_url: None
                            },
                            TierlistItem {
                                name: "E".to_string(),
                                image_url: None
                            },
                            TierlistItem {
                                name: "F".to_string(),
                                image_url: None
                            }
                        ],
                        creator: ADDR1.to_string()
                    }
                ),
            ]
        );
    }
}
