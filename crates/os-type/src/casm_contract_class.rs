use std::cell::OnceCell;
use std::sync::Arc;
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