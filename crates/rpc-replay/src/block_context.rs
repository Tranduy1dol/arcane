use crate::utils::{felt_to_u128, FeltConversionError};
use blockifier::blockifier::block::{BlockInfo, GasPrices};
use blockifier::bouncer::BouncerConfig;
use blockifier::context::{BlockContext, ChainInfo, FeeTokenAddresses};
use blockifier::versioned_constants::{StarknetVersion, VersionedConstants};
use starknet::core::types::{BlockWithTxs, L1DataAvailabilityMode};
use starknet_api::block::{BlockNumber, BlockTimestamp};
use starknet_api::contract_address;
use starknet_api::core::ChainId;
use starknet_types_core::felt::Felt;
use std::num::NonZeroU128;

fn felt_to_gas_price(price: &Felt) -> Result<NonZeroU128, FeltConversionError> {
    if *price == Felt::ZERO {
        return Ok(NonZeroU128::MIN);
    }
    let gas_price = felt_to_u128(price)?;
    NonZeroU128::new(gas_price).ok_or(FeltConversionError::CustomError(
        "Gas price cannot be zero".to_string(),
    ))
}

pub fn build_block_context(
    chain_id: String,
    block: &BlockWithTxs,
) -> Result<BlockContext, FeltConversionError> {
    let sequencer_address_hex = block.sequencer_address.to_hex_string();
    let sequencer_address = contract_address!(sequencer_address_hex.as_str());
    let use_kzg_da = match block.l1_da_mode {
        L1DataAvailabilityMode::Blob => true,
        L1DataAvailabilityMode::Calldata => false,
    };

    let block_info = BlockInfo {
        block_number: BlockNumber(block.block_number),
        block_timestamp: BlockTimestamp(block.timestamp),
        sequencer_address,
        gas_prices: GasPrices {
            eth_l1_gas_price: felt_to_gas_price(&block.l1_gas_price.price_in_wei)?,
            strk_l1_gas_price: felt_to_gas_price(&block.l1_gas_price.price_in_fri)?,
            eth_l1_data_gas_price: felt_to_gas_price(&block.l1_data_gas_price.price_in_wei)?,
            strk_l1_data_gas_price: felt_to_gas_price(&block.l1_data_gas_price.price_in_fri)?,
        },
        use_kzg_da,
    };

    let chain_info = ChainInfo {
        chain_id: ChainId::Other(chain_id),
        fee_token_addresses: FeeTokenAddresses {
            strk_fee_token_address: contract_address!(
                "0x04718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d"
            ),
            eth_fee_token_address: contract_address!(
                "0x049d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"
            ),
        },
    };

    let versioned_constants = VersionedConstants::get(StarknetVersion::V0_13_1);
    let bouncer_config = BouncerConfig::max();

    Ok(BlockContext::new(
        block_info,
        chain_info,
        versioned_constants.clone(),
        bouncer_config,
    ))
}
