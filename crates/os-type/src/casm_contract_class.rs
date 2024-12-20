use std::cell::OnceCell;
use std::sync::Arc;
use crate::error::{ContractClassError, ConversionError};
use crate::hash::GenericClassHash;

pub type CairoLangCasmClass = cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;
pub type BlockifierCasmClass = blockifier::execution::contract_class::ContractClassV1;

#[derive(Debug, Clone)]
pub struct GenericCasmContractClass {
    blockifier_contract_class: OnceCell<Arc<BlockifierCasmClass>>,
    cairo_lang_contract_class: OnceCell<Arc<CairoLangCasmClass>>,
    serialized_class: OnceCell<Arc<Vec<u8>>>,
    class_hash: OnceCell<GenericClassHash>,
}

impl From<CairoLangCasmClass> for GenericCasmContractClass {
    fn from(cairo_lang_class: CairoLangCasmClass) -> Self {
        Self {
            blockifier_contract_class: Default::default(),
            cairo_lang_contract_class: OnceCell::from(Arc::new(cairo_lang_class)),
            serialized_class: Default::default(),
            class_hash: Default::default(),
        }
    }
}

impl GenericCasmContractClass {
    pub fn from_bytes(serialized_class: Vec<u8>) -> Self {
        Self {
            blockifier_contract_class: OnceCell::new(),
            cairo_lang_contract_class: OnceCell::new(),
            serialized_class: OnceCell::from(Arc::new(serialized_class)),
            class_hash: OnceCell::new(),
        }
    }

    fn build_cairo_lang_class(&self) -> Result<CairoLangCasmClass, ContractClassError> {
        if let Some(serialized_class) = self.serialized_class.get() {
            let contract_class = serde_json::from_slice(serialized_class)?;
            return Ok(contract_class);
        }

        Err(ContractClassError::ConversionError(ConversionError::CairoLangClassMissing))
    }

    fn build_blockifier_class(&self) -> Result<BlockifierCasmClass, ContractClassError> {
        if let Some(cairo_lang_class) = self.cairo_lang_contract_class.get() {
            return blockifier_contract_class_from_cairo_lang_class(cairo_lang_class.as_ref().clone());
        }

        if let Some(serialized_class) = &self.serialized_class.get() {
            let cairo_lang_class = cairo_lang_contract_class_from_bytes(serialized_class)?;
            self.cairo_lang_contract_class
                .set(Arc::new(cairo_lang_class.clone()))
                .expect("cairo-lang class is already set");
            return blockifier_contract_class_from_cairo_lang_class(cairo_lang_class);
        }

        Err(ContractClassError::ConversionError(ConversionError::BlockifierClassMissing))
    }
    pub fn get_cairo_lang_contract_class(&self) -> Result<&CairoLangCasmClass, ContractClassError> {
        self.cairo_lang_contract_class
            .get_or_try_init(|| self.build_cairo_lang_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }

    pub fn get_blockifier_contract_class(&self) -> Result<&BlockifierCasmClass, ContractClassError> {
        self.blockifier_contract_class
            .get_or_try_init(|| self.build_blockifier_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }

    pub fn to_cairo_lang_contract_class(self) -> Result<CairoLangCasmClass, ContractClassError> {
        let cairo_lang_class = self.get_cairo_lang_contract_class()?;
        Ok(cairo_lang_class.clone())
    }

    pub fn to_blockifier_contract_class(self) -> Result<BlockifierCasmClass, ContractClassError> {
        let blockifier_class = self.get_blockifier_contract_class()?;
        Ok(blockifier_class.clone())
    }

    fn compute_class_hash(&self) -> Result<GenericClassHash, ContractClassError> {
        let compiled_class = self.get_cairo_lang_contract_class()?;
        let class_hash_felt = compiled_class.compiled_class_hash();

        Ok(GenericClassHash::from_bytes_be(class_hash_felt.to_bytes_be()))
    }

    pub fn class_hash(&self) -> Result<GenericClassHash, ContractClassError> {
        self.class_hash.get_or_try_init(|| self.compute_class_hash()).copied()
    }
}

fn blockifier_contract_class_from_cairo_lang_class(
    cairo_lang_class: CairoLangCasmClass,
) -> Result<BlockifierCasmClass, ContractClassError> {
    let blockifier_class: BlockifierCasmClass = cairo_lang_class
        .try_into()
        .map_err(|e| ContractClassError::ConversionError(ConversionError::BlockifierError(Box::new(e))))?;
    Ok(blockifier_class)
}

fn cairo_lang_contract_class_from_bytes(bytes: &[u8]) -> Result<CairoLangCasmClass, ContractClassError> {
    let contract_class = serde_json::from_slice(bytes)?;
    Ok(contract_class)
}