use std::cell::OnceCell;
use std::sync::Arc;
use crate::arcane_core_addons::{decompress_starknet_core_contract_class, LegacyContractDecompressionError};
use crate::hash::GenericClassHash;

pub type StarknetApiDeprecatedClass = starknet_api::deprecated_contract_class::ContractClass;
pub type StarknetCoreDeprecatedClass = starknet_core::types::contract::legacy::LegacyContractClass;
pub type CompressedStarknetCoreDeprecatedClass = starknet_core::types::CompressedLegacyContractClass;
pub type BlockifierDeprecatedClass = blockifier::execution::contract_class::ContractClassV0;

#[derive(Debug, Clone)]
pub struct GenericDeprecatedCompiledClass {
    blockifier_contract_class: OnceCell<Arc<BlockifierDeprecatedClass>>,
    starknet_api_contract_class: OnceCell<Arc<StarknetApiDeprecatedClass>>,
    starknet_core_contract_class: OnceCell<Arc<StarknetCoreDeprecatedClass>>,
    serialized_class: OnceCell<Vec<u8>>,
    class_hash: OnceCell<GenericClassHash>,
}

impl From<BlockifierDeprecatedClass> for GenericDeprecatedCompiledClass {
    fn from(blockifier_class: BlockifierDeprecatedClass) -> Self {
        Self {
            blockifier_contract_class: OnceCell::from(Arc::new(blockifier_class)),
            starknet_api_contract_class: Default::default(),
            starknet_core_contract_class: Default::default(),
            serialized_class: Default::default(),
            class_hash: Default::default(),
        }
    }
}

impl TryFrom<CompressedStarknetCoreDeprecatedClass> for GenericDeprecatedCompiledClass {
    type Error = LegacyContractDecompressionError;

    fn try_from(compressed_legacy_class: CompressedStarknetCoreDeprecatedClass) -> Result<Self, Self::Error> {
        let legacy_class = decompress_starknet_core_contract_class(compressed_legacy_class)?;
        Ok(Self::from(legacy_class))
    }
}