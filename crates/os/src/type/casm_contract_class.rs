use crate::r#type::error::ContractClassError;
use crate::r#type::hash::GenericClassHash;
use std::cell::OnceCell;
use std::sync::Arc;

pub type BlockifierCasmClass = blockifier::execution::contract_class::ContractClassV1;
pub type CairoLangCasmClass = cairo_lang_starknet_classes::casm_contract_class::CasmContractClass;

#[derive(Debug, Clone)]
pub struct GenericCasmContractClass {
    blockifier_contract_class: OnceCell<Arc<BlockifierCasmClass>>,
    cairo_lang_contract_class: OnceCell<Arc<CairoLangCasmClass>>,
    serialized_class: OnceCell<Arc<Vec<u8>>>,
    class_hash: OnceCell<GenericClassHash>,
}

impl GenericCasmContractClass {
    pub fn get_cairo_lang_contract_class(&self) -> Result<&CairoLangCasmClass, ContractClassError> {
        self.cairo_lang_contract_class
            .get_or_try_init(|| self.build_cairo_lang_class().map(Arc::new))
            .map(|boxed| boxed.as_ref())
    }
    pub fn to_cairo_lang_contract_class(self) -> Result<CairoLangCasmClass, ContractClassError> {
        let cairo_lang_class = self.get_cairo_lang_contract_class()?;
        Ok(cairo_lang_class.clone())
    }
}
