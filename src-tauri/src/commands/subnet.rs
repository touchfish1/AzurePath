use crate::core;
use crate::types::subnet::{SubnetResult, SubnetSplitResult};

#[tauri::command]
pub fn calculate_subnet(address: String, cidr: u8) -> Result<SubnetResult, String> {
    core::subnet::calculate_subnet(&address, cidr)
}

#[tauri::command]
pub fn split_subnet(network: String, target_prefix: u8) -> Result<SubnetSplitResult, String> {
    core::subnet::split_subnet(&network, target_prefix)
}
