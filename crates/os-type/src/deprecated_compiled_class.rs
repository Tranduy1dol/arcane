use std::cell::OnceCell;
use std::sync::Arc;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use starknet_gateway_types::class_hash::compute_class_hash;
use crate::arcane_core_addons::{decompress_starknet_core_contract_class, LegacyContractDecompressionError};
use crate::error::{ContractClassError, ConversionError};
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

impl GenericDeprecatedCompiledClass {
    pub fn from_bytes(serialized_class: Vec<u8>) -> Self {
        Self {
            blockifier_contract_class: Default::default(),
            starknet_api_contract_class: Default::default(),
            starknet_core_contract_class: Default::default(),
            serialized_class: OnceCell::from(serialized_class),
            class_hash: OnceCell::new(),
        }
    }

    fn build_starknet_api_class(&self) -> Result<StarknetApiDeprecatedClass, ContractClassError> {
        if let Some(serialized_class) = self.serialized_class.get() {
            let contract_class = serde_json::from_slice(serialized_class)?;
            return Ok(contract_class);
        }

        Err(ContractClassError::ConversionError(ConversionError::StarknetClassMissing))
    }

    fn build_blockifier_class(&self) -> Result<BlockifierDeprecatedClass, ContractClassError> {
        let serialized_class = self.serialized_class.get_or_try_init(|| serde_json::to_vec(self))?;

        let blockifier_class: BlockifierDeprecatedClass = serde_json::from_slice(serialized_class)?;
        Ok(blockifier_class)
    }

    pub fn get_starknet_api_contract_class(&self) -> Result<&StarknetApiDeprecatedClass, ContractClassError> {
        self.starknet_api_contract_class
            .get_or_try_init(|| self.build_starknet_api_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }

    pub fn get_blockifier_contract_class(&self) -> Result<&BlockifierDeprecatedClass, ContractClassError> {
        self.blockifier_contract_class
            .get_or_try_init(|| self.build_blockifier_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }

    pub fn get_serialized_contract_class(&self) -> Result<&Vec<u8>, ContractClassError> {
        self.serialized_class.get_or_try_init(|| serde_json::to_vec(self)).map_err(Into::into)
    }

    pub fn to_starknet_api_contract_class(self) -> Result<StarknetApiDeprecatedClass, ContractClassError> {
        let cairo_lang_class = self.get_starknet_api_contract_class()?;
        Ok(cairo_lang_class.clone())
    }

    pub fn to_blockifier_contract_class(self) -> Result<BlockifierDeprecatedClass, ContractClassError> {
        let blockifier_class = self.get_blockifier_contract_class()?;
        Ok(blockifier_class.clone())
    }

    fn compute_class_hash(&self) -> Result<GenericClassHash, ContractClassError> {
        let serialized_class = self.get_serialized_contract_class()?;
        /// TODO: Add Madara compute class hash
        let class_hash =
            compute_class_hash(serialized_class).map_err(|e| ContractClassError::HashError(e.to_string()))?;

        Ok(GenericClassHash::from_bytes_be(class_hash.hash().0.to_be_bytes()))
    }

    pub fn class_hash(&self) -> Result<GenericClassHash, ContractClassError> {
        self.class_hash.get_or_try_init(|| self.compute_class_hash()).copied()
    }
}

impl Serialize for GenericDeprecatedCompiledClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(starknet_api_class) = self.starknet_api_contract_class.get() {
            starknet_api_class.serialize(serializer)
        } else if let Some(starknet_core_class) = self.starknet_core_contract_class.get() {
            starknet_core_class.serialize(serializer)
        } else if self.serialized_class.get().is_some() {
            // It seems like there is no way to just pass the `serialized_class` field as the output
            // of `serialize()`, so we are forced to serialize an actual class instance.
            let starknet_api_class =
                self.get_starknet_api_contract_class().map_err(|e| serde::ser::Error::custom(e.to_string()))?;
            starknet_api_class.serialize(serializer)
        } else {
            Err(serde::ser::Error::custom("No conversion found"))
        }
    }
}

impl<'de> Deserialize<'de> for GenericDeprecatedCompiledClass {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let starknet_api_class = StarknetApiDeprecatedClass::deserialize(deserializer)?;
        Ok(Self::from(starknet_api_class))
    }
}

