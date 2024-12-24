use std::cell::OnceCell;
use std::sync::Arc;
use serde::{Deserialize, Serialize, Serializer};
use serde::ser::Error;
use serde_with::serde_as;
use starknet_core::types::EntryPointsByType;
use starknet_types_core::felt::Felt;
use crate::casm_contract_class::{CairoLangCasmClass, GenericCasmContractClass};
use crate::error::ContractClassError;
use crate::hash::GenericClassHash;

pub type CairoLangSierraContractClass = cairo_lang_starknet_classes::contract_class::ContractClass;
pub type StarknetCoreSierraContractClass = starknet_core::types::FlattenedSierraClass;

#[derive(Debug, Clone)]
pub struct GenericSierraContractClass {
    cairo_lang_contract_class: OnceCell<Arc<CairoLangSierraContractClass>>,
    starknet_core_contract_class: OnceCell<Arc<StarknetCoreSierraContractClass>>,
    serialized_class: OnceCell<Vec<u8>>,
    class_hash: OnceCell<GenericClassHash>,
}

#[serde_as]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlattenedSierraClassWithAbi {
    pub sierra_program: Vec<Felt>,
    pub contract_class_version: String,
    pub entry_points_by_type: EntryPointsByType,
    pub abi: Option<cairo_lang_starknet_classes::abi::Contract>,
}

impl From<StarknetCoreSierraContractClass> for GenericSierraContractClass {
    fn from(starknet_core_class: StarknetCoreSierraContractClass) -> Self {
        Self {
            cairo_lang_contract_class: Default::default(),
            starknet_core_contract_class: OnceCell::from(Arc::new(starknet_core_class)),
            serialized_class: Default::default(),
            class_hash: Default::default(),
        }
    }
}

impl Serialize for GenericSierraContractClass {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if let Some(cairo_lang_class) = self.cairo_lang_contract_class.get() {
            cairo_lang_class.serialize(serializer)
        } else if let Some(starknet_core_class) = self.starknet_core_contract_class.get() {
            let class_with_abi = FlattenedSierraClassWithAbi::try_from(starknet_core_class.as_ref())
                .map_err(|e| S::Error::custom(e.to_string()))?;
            class_with_abi.serialize(serializer)
        } else {
            Err(S::Error::custom("No possible serialization"))
        }
    }
}

impl GenericSierraContractClass {
    pub fn from_bytes(serialized_class: Vec<u8>) -> Self {
        Self {
            cairo_lang_contract_class: Default::default(),
            starknet_core_contract_class: Default::default(),
            serialized_class: OnceCell::from(serialized_class),
            class_hash: OnceCell::new(),
        }
    }

    fn build_cairo_lang_class(&self) -> Result<CairoLangSierraContractClass, ContractClassError> {
        self.get_serialized_contract_class().and_then(|res| {
            let contract_class = serde_json::from_slice(res)?;
            Ok(contract_class)
        })
    }

    pub fn get_serialized_contract_class(&self) -> Result<&Vec<u8>, ContractClassError> {
        self.serialized_class.get_or_try_init(|| serde_json::to_vec(self)).map_err(Into::into)
    }

    fn build_starknet_core_class(&self) -> Result<StarknetCoreSierraContractClass, ContractClassError> {
        let serialized_class = self.get_serialized_contract_class()?;
        let sierra_class: starknet_core::types::contract::SierraClass =
            serde_json::from_slice(serialized_class).map_err(ContractClassError::SerdeError)?;

        sierra_class.flatten().map_err(|e| ContractClassError::SerdeError(serde_json::Error::custom(e)))
    }
    pub fn get_cairo_lang_contract_class(&self) -> Result<&CairoLangSierraContractClass, ContractClassError> {
        self.cairo_lang_contract_class
            .get_or_try_init(|| self.build_cairo_lang_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }

    pub fn get_starknet_core_contract_class(&self) -> Result<&StarknetCoreSierraContractClass, ContractClassError> {
        self.starknet_core_contract_class
            .get_or_try_init(|| self.build_starknet_core_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }

    pub fn to_cairo_lang_contract_class(self) -> Result<CairoLangSierraContractClass, ContractClassError> {
        let cairo_lang_class = self.get_cairo_lang_contract_class()?;
        Ok(cairo_lang_class.clone())
    }

    pub fn to_starknet_core_contract_class(self) -> Result<StarknetCoreSierraContractClass, ContractClassError> {
        let blockifier_class = self.get_starknet_core_contract_class()?;
        Ok(blockifier_class.clone())
    }

    fn compute_class_hash(&self) -> Result<GenericClassHash, ContractClassError> {
        let starknet_core_contract_class = self.get_starknet_core_contract_class()?;
        let class_hash = starknet_core_contract_class.class_hash();
        Ok(GenericClassHash::new(class_hash.into()))
    }

    pub fn class_hash(&self) -> Result<GenericClassHash, ContractClassError> {
        self.class_hash.get_or_try_init(|| self.compute_class_hash()).copied()
    }

    pub fn compile(&self) -> Result<GenericCasmContractClass, ContractClassError> {
        let cairo_lang_class = self.get_cairo_lang_contract_class()?.clone();
        let add_pythonic_hints = false;
        let max_bytecode_size = 180000;
        let casm_contract_class =
            CairoLangCasmClass::from_contract_class(cairo_lang_class, add_pythonic_hints, max_bytecode_size)?;

        Ok(GenericCasmContractClass::from(casm_contract_class))
    }
}