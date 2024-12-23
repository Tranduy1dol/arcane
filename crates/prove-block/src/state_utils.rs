use std::collections::{HashMap, HashSet};
use cairo_vm::Felt252;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::{JsonRpcClient, Provider, ProviderError};
use starknet_core::types::{BlockId, Felt, MaybePendingStateUpdate, StarknetError, StateDiff, TransactionTraceWithHash};
use arcane_os_type::class_hash_utils::ContractClassComponentHashes;
use arcane_os_type::compiled_class::GenericCompiledClass;
use arcane_os_type::sierra_contract_class::GenericSierraContractClass;
use arcane_os_type::casm_contract_class::GenericCasmContractClass;
use arcane_os_type::deprecated_compiled_class::GenericDeprecatedCompiledClass;
use rpc_client::client::RpcClient;
use crate::ProveBlockError;
use crate::utils::get_subcalled_contracts_from_tx_traces;

#[derive(Clone)]
pub struct FormattedStateUpdate {
    // TODO: Use more descriptive types
    pub class_hash_to_compiled_class_hash: HashMap<Felt252, Felt252>,
    pub compiled_classes: HashMap<Felt252, GenericCasmContractClass>,
    pub deprecated_compiled_classes: HashMap<Felt252, GenericDeprecatedCompiledClass>,
    pub declared_class_hash_component_hashes: HashMap<Felt252, ContractClassComponentHashes>,
}

pub(crate) async fn get_formatted_state_update(
    provider: &RpcClient,
    block_id: BlockId,
    previous_block_id: BlockId,
) -> Result<(FormattedStateUpdate, Vec<TransactionTraceWithHash>), ProveBlockError> {
    let state_update = match provider.get_state_update(block_id).await? {
        MaybePendingStateUpdate::Update(update) => update,
        MaybePendingStateUpdate::PendingUpdate(_) => {
            panic!("Block is still pending!")
        }
    };
    let state_diff = state_update.state_diff;
    let traces = provider.trace_block_transactions(block_id).await?;

    let (accessed_addresses, accessed_classes) = get_subcalled_contracts_from_tx_traces(&traces);

    let declared_classes: HashSet<_> = state_diff.declared_classes.iter().map(|declared_item| Felt252::from(declared_item.class_hash)).collect();

    let mut class_hash_to_compiled_class_hash: HashMap<Felt252, Felt252> = HashMap::new();
    let (compiled_contract_classes, deprecated_compiled_contract_classes, declared_class_hash_component_hashes) =
    build_compiled_class_and_maybe_update_class_hash_to_compiled_class_hash(
        provider,
        previous_block_id,
        block_id,
        &accessed_addresses,
        &declared_classes,
        &accessed_classes,
        &mut class_hash_to_compiled_class_hash
    ).await?;

    format_declared_classes(&state_diff, &mut class_hash_to_compiled_class_hash);

    Ok((
        FormattedStateUpdate {
            class_hash_to_compiled_class_hash,
            compiled_classes: compiled_contract_classes,
            deprecated_compiled_classes: deprecated_compiled_contract_classes,
            declared_class_hash_component_hashes,
        },
        traces,
    ))
}

async fn build_compiled_class_and_maybe_update_class_hash_to_compiled_class_hash(
    provider: &JsonRpcClient<HttpTransport>,
    previous_block_id: BlockId,
    block_id: BlockId,
    accessed_addresses: &HashSet<Felt252>,
    declared_classes: &HashSet<Felt252>,
    accessed_classes: &HashSet<Felt252>,
    class_hash_to_compiled_class_hash: &mut HashMap<Felt252, Felt252>,
) -> Result<
    (
        HashMap<Felt252, GenericCasmContractClass>,
        HashMap<Felt252, GenericDeprecatedCompiledClass>,
        HashMap<Felt252, ContractClassComponentHashes>,
    ),
    ProveBlockError,
> {
    let mut compiled_contract_classes: HashMap<Felt252, GenericCasmContractClass> = HashMap::new();
    let mut deprecated_compiled_contract_classes: HashMap<Felt252, GenericDeprecatedCompiledClass> = HashMap::new();

    for contract_address in accessed_addresses {
        // In case there is a class change, we need to get the compiled class for
        // the block to prove and for the previous block as they may differ.
        // Note that we must also consider the case where the contract was deployed in the current
        // block, so we can ignore "ContractNotFound" failures.
        if let Err(e) = add_compiled_class_from_contract_to_os_input(
            provider,
            *contract_address,
            previous_block_id,
            class_hash_to_compiled_class_hash,
            &mut compiled_contract_classes,
            &mut deprecated_compiled_contract_classes,
        )
            .await
        {
            match e {
                ProveBlockError::RpcError(ProviderError::StarknetError(StarknetError::ContractNotFound)) => {
                }
                _ => return Err(e),
            }
        }

        add_compiled_class_from_contract_to_os_input(
            provider,
            *contract_address,
            block_id,
            class_hash_to_compiled_class_hash,
            &mut compiled_contract_classes,
            &mut deprecated_compiled_contract_classes,
        )
            .await?;
    }

    for class_hash in accessed_classes {
        let contract_class = provider.starknet_rpc().get_class(block_id, class_hash).await?;
        add_compiled_class_to_os_input(
            *class_hash,
            contract_class,
            class_hash_to_compiled_class_hash,
            &mut compiled_contract_classes,
            &mut deprecated_compiled_contract_classes,
        )?;
    }

    let mut declared_class_hash_to_component_hashes = HashMap::new();
    for class_hash in declared_classes {
        let contract_class = provider.starknet_rpc().get_class(block_id, class_hash).await?;
        if let starknet::core::types::ContractClass::Sierra(flattened_sierra_class) = &contract_class {
            let component_hashes = ContractClassComponentHashes::from(flattened_sierra_class.clone());
            declared_class_hash_to_component_hashes.insert(*class_hash, component_hashes);
        }
    }

    Ok((compiled_contract_classes, deprecated_compiled_contract_classes, declared_class_hash_to_component_hashes))
}

async fn add_compiled_class_from_contract_to_os_input(
    provider: &JsonRpcClient<HttpTransport>,
    contract_address: Felt252,
    block_id: BlockId,
    class_hash_to_compiled_class_hash: &mut HashMap<Felt252, Felt252>,
    compiled_contract_classes: &mut HashMap<Felt252, GenericCasmContractClass>,
    deprecated_compiled_contract_classes: &mut HashMap<Felt252, GenericDeprecatedCompiledClass>,
) -> Result<(), ProveBlockError> {
    let class_hash = provider.get_class_hash_at(block_id, contract_address).await?;
    let contract_class = provider.get_class(block_id, class_hash).await?;

    add_compiled_class_to_os_input(
        class_hash,
        contract_class,
        class_hash_to_compiled_class_hash,
        compiled_contract_classes,
        deprecated_compiled_contract_classes,
    )
}

fn add_compiled_class_to_os_input(
    class_hash: Felt,
    contract_class: starknet::core::types::ContractClass,
    class_hash_to_compiled_class_hash: &mut HashMap<Felt252, Felt252>,
    compiled_contract_classes: &mut HashMap<Felt252, GenericCasmContractClass>,
    deprecated_compiled_contract_classes: &mut HashMap<Felt252, GenericDeprecatedCompiledClass>,
) -> Result<(), ProveBlockError> {
    if class_hash_to_compiled_class_hash.contains_key(&Felt252::from(class_hash)) {
        return Ok(());
    }

    let compiled_class = compile_contract_class(contract_class)?;
    let compiled_class_hash = compiled_class.class_hash()?;

    // Remove deprecated classes from HashMap
    if matches!(&compiled_class, GenericCompiledClass::Cairo0(_)) {
        log::warn!("Skipping deprecated class for ch_to_cch: 0x{:x}", class_hash);
    } else {
        class_hash_to_compiled_class_hash.insert(Felt252::from(class_hash), compiled_class_hash.into());
    }

    match compiled_class {
        GenericCompiledClass::Cairo0(deprecated_cc) => {
            deprecated_compiled_contract_classes.insert(Felt252::from(class_hash), deprecated_cc);
        }
        GenericCompiledClass::Cairo1(casm_cc) => {
            compiled_contract_classes.insert(compiled_class_hash.into(), casm_cc);
        }
    }

    Ok(())
}

fn compile_contract_class(
    contract_class: starknet::core::types::ContractClass,
) -> Result<GenericCompiledClass, ProveBlockError> {
    let compiled_class = match contract_class {
        starknet::core::types::ContractClass::Sierra(flattened_sierra_cc) => {
            let sierra_class = GenericSierraContractClass::from(flattened_sierra_cc);
            let compiled_class = sierra_class.compile()?;
            GenericCompiledClass::Cairo1(compiled_class)
        }
        starknet::core::types::ContractClass::Legacy(legacy_cc) => {
            let compiled_class = GenericDeprecatedCompiledClass::try_from(legacy_cc)?;
            GenericCompiledClass::Cairo0(compiled_class)
        }
    };

    Ok(compiled_class)
}

fn format_declared_classes(state_diff: &StateDiff, class_hash_to_compiled_class_hash: &mut HashMap<Felt252, Felt252>) {
    // The comment below explicits that the value should be 0 for new classes:
    // From execute_transactions.cairo
    // Note that prev_value=0 enforces that a class may be declared only once.
    // dict_update{dict_ptr=contract_class_changes}(
    //     key=[class_hash_ptr], prev_value=0, new_value=compiled_class_hash
    // );

    // class_hash_to_compiled_class_hash is already populated. However, for classes
    // that are defined in state_diff.declared_classes, we need to set the
    // compiled_class_hashes to zero as it was explained above
    for class in state_diff.declared_classes.iter() {
        class_hash_to_compiled_class_hash.insert(Felt252::from(class.class_hash), Felt252::ZERO);
    }
}