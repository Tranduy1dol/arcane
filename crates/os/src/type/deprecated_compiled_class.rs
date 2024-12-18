use std::cell::OnceCell;
use std::sync::Arc;
use crate::r#type::error::ContractClassError;
use crate::r#type::hash::GenericClassHash;

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

impl GenericDeprecatedCompiledClass {
    pub fn get_starknet_api_contract_class(&self) -> Result<&StarknetApiDeprecatedClass, ContractClassError> {
        self.starknet_api_contract_class
            .get_or_try_init(|| self.build_starknet_api_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }

    pub fn to_starknet_api_contract_class(self) -> Result<StarknetApiDeprecatedClass, ContractClassError> {
        let cairo_lang_class = self.get_starknet_api_contract_class()?;
        Ok(cairo_lang_class.clone())
    }
}