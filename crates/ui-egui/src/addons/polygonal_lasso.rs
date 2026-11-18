use pluma_tools::addons::polygonal_lasso;
use serde_json::json;

use crate::menus::Item;

pub fn menu() -> Vec<Item> {
    vec![Item::Cmd("Polygonal Lasso Tool", "tool.select", json!({"tool": polygonal_lasso::ID}))]
}
