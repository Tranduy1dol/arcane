use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use starknet_api::contract_address;
use starknet_api::core::{ChainId, ContractAddress};

const MAX_STEPS_PER_TX: u32 = 4_000_000;
pub const DEFAULT_FEE_TOKEN_ADDR: &str = "482bc27fc5627bf974a72b65c43aa8a0464a70aab91ad8379b56a4f17a84c3";
pub const DEFAULT_DEPRECATED_FEE_TOKEN_ADDR: &str = "482bc27fc5627bf974a72b65c43aa8a0464a70aab91ad8379b56a4f17a84c3";
pub const STORED_BLOCK_HASH_BUFFER: u64 = 10;

#[serde_as]
#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct StarknetOsConfig {
    #[serde_as(as = "ChainIdNum")]
    pub chain_id: ChainId,
    pub fee_token_address: ContractAddress,
    pub deprecated_fee_token_address: ContractAddress,
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct StarknetGeneralConfig {
    pub starknet_os_config: StarknetOsConfig,
    pub gas_price_bounds: GasPriceBounds,
    pub invoke_tx_max_n_steps: u32,
    pub validate_max_n_steps: u32,
    pub default_eth_price_in_fri: u128,
    pub sequencer_address: ContractAddress,
    pub enforce_l1_handler_fee: bool,
    #[serde(default = "default_use_kzg_da")]
    pub use_kzg_da: bool,
}

const fn default_use_kzg_da() -> bool {
    true
}

#[derive(Debug, Serialize, Clone, Deserialize, PartialEq)]
pub struct GasPriceBounds {
    pub min_wei_l1_gas_price: u128,
    pub min_fri_l1_gas_price: u128,
    pub max_fri_l1_gas_price: u128,
    pub min_wei_l1_data_gas_price: u128,
    pub min_fri_l1_data_gas_price: u128,
    pub max_fri_l1_data_gas_price: u128,
}

impl Default for StarknetGeneralConfig {
    fn default() -> Self {
        Self {
            starknet_os_config: StarknetOsConfig {
                chain_id: ChainId::Sepolia,
                fee_token_address: contract_address!(DEFAULT_FEE_TOKEN_ADDR),
                deprecated_fee_token_address: contract_address!(DEFAULT_DEPRECATED_FEE_TOKEN_ADDR),
            },
            gas_price_bounds: GasPriceBounds {
                max_fri_l1_data_gas_price: 10000000000,
                max_fri_l1_gas_price: 100000000000000,
                min_fri_l1_data_gas_price: 10,
                min_fri_l1_gas_price: 100000000000,
                min_wei_l1_data_gas_price: 100000,
                min_wei_l1_gas_price: 10000000000,
            },
            invoke_tx_max_n_steps: MAX_STEPS_PER_TX,
            validate_max_n_steps: MAX_STEPS_PER_TX,
            default_eth_price_in_fri: 1_000_000_000_000_000_000_000,
            sequencer_address: contract_address!(SEQUENCER_ADDR_0_13_2),
            enforce_l1_handler_fee: true,
            use_kzg_da: false,
        }
    }
}