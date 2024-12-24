use blockifier::execution::call_info::CallInfo;
use blockifier::transaction::objects::TransactionExecutionInfo;
use cairo_vm::Felt252;
use starknet_api::core::ContractAddress;
use starknet_api::state::StorageKey;
use starknet_core::types::{
    ExecuteInvocation, FunctionInvocation, TransactionTrace, TransactionTraceWithHash,
};
use std::collections::{HashMap, HashSet};

pub(crate) fn get_subcalled_contracts_from_tx_traces(
    traces: &[TransactionTraceWithHash],
) -> (HashSet<Felt252>, HashSet<Felt252>) {
    let mut contracts_subcalled: HashSet<Felt252> = HashSet::new();
    let mut classes_subcalled: HashSet<Felt252> = HashSet::new();
    for trace in traces {
        match &trace.trace_root {
            TransactionTrace::Invoke(invoke_trace) => {
                if let Some(inv) = &invoke_trace.validate_invocation {
                    process_function_invocations(
                        inv,
                        &mut contracts_subcalled,
                        &mut classes_subcalled,
                    );
                }
                if let ExecuteInvocation::Success(inv) = &invoke_trace.execute_invocation {
                    process_function_invocations(
                        inv,
                        &mut contracts_subcalled,
                        &mut classes_subcalled,
                    );
                }
                if let Some(inv) = &invoke_trace.fee_transfer_invocation {
                    process_function_invocations(
                        inv,
                        &mut contracts_subcalled,
                        &mut classes_subcalled,
                    );
                }
            }
            TransactionTrace::Declare(declare_trace) => {
                if let Some(inv) = &declare_trace.validate_invocation {
                    process_function_invocations(
                        inv,
                        &mut contracts_subcalled,
                        &mut classes_subcalled,
                    );
                }
                if let Some(inv) = &declare_trace.fee_transfer_invocation {
                    process_function_invocations(
                        inv,
                        &mut contracts_subcalled,
                        &mut classes_subcalled,
                    );
                }
            }
            TransactionTrace::L1Handler(l1handler_trace) => {
                process_function_invocations(
                    &l1handler_trace.function_invocation,
                    &mut contracts_subcalled,
                    &mut classes_subcalled,
                );
            }

            TransactionTrace::DeployAccount(deploy_trace) => {
                if let Some(inv) = &deploy_trace.validate_invocation {
                    process_function_invocations(
                        inv,
                        &mut contracts_subcalled,
                        &mut classes_subcalled,
                    );
                }
                if let Some(inv) = &deploy_trace.fee_transfer_invocation {
                    process_function_invocations(
                        inv,
                        &mut contracts_subcalled,
                        &mut classes_subcalled,
                    );
                }
                process_function_invocations(
                    &deploy_trace.constructor_invocation,
                    &mut contracts_subcalled,
                    &mut classes_subcalled,
                );
            }
        }
    }
    (contracts_subcalled, classes_subcalled)
}

fn process_function_invocations(
    inv: &FunctionInvocation,
    contracts: &mut HashSet<Felt252>,
    classes: &mut HashSet<Felt252>,
) {
    contracts.insert(Felt252::from(inv.contract_address));
    classes.insert(Felt252::from(inv.class_hash));
    for call in &inv.calls {
        process_function_invocations(call, contracts, classes);
    }
}

pub(crate) fn get_all_accessed_keys(
    tx_execution_infos: &[TransactionExecutionInfo],
) -> HashMap<ContractAddress, HashSet<StorageKey>> {
    let mut accessed_keys_by_address: HashMap<ContractAddress, HashSet<StorageKey>> =
        HashMap::new();

    for tx_execution_info in tx_execution_infos {
        let accessed_keys_in_tx = get_accessed_keys_in_tx(tx_execution_info);
        for (contract_address, storage_keys) in accessed_keys_in_tx {
            accessed_keys_by_address
                .entry(contract_address)
                .or_default()
                .extend(storage_keys);
        }
    }

    accessed_keys_by_address
}

fn get_accessed_keys_in_tx(
    tx_execution_info: &TransactionExecutionInfo,
) -> HashMap<ContractAddress, HashSet<StorageKey>> {
    let mut accessed_keys_by_address: HashMap<ContractAddress, HashSet<StorageKey>> =
        HashMap::new();

    for call_info in [
        &tx_execution_info.validate_call_info,
        &tx_execution_info.execute_call_info,
        &tx_execution_info.fee_transfer_call_info,
    ]
    .into_iter()
    .flatten()
    {
        let call_storage_keys = get_accessed_storage_keys(call_info);
        for (contract_address, storage_keys) in call_storage_keys {
            accessed_keys_by_address
                .entry(contract_address)
                .or_default()
                .extend(storage_keys);
        }
    }

    accessed_keys_by_address
}

fn get_accessed_storage_keys(
    call_info: &CallInfo,
) -> HashMap<ContractAddress, HashSet<StorageKey>> {
    let mut accessed_keys_by_address: HashMap<ContractAddress, HashSet<StorageKey>> =
        HashMap::new();

    let contract_address = &call_info.call.storage_address;
    accessed_keys_by_address
        .entry(*contract_address)
        .or_default()
        .extend(call_info.accessed_storage_keys.iter().copied());

    for inner_call in &call_info.inner_calls {
        let inner_call_storage_keys = get_accessed_storage_keys(inner_call);
        for (contract_address, storage_keys) in inner_call_storage_keys {
            accessed_keys_by_address
                .entry(contract_address)
                .or_default()
                .extend(storage_keys);
        }
    }

    accessed_keys_by_address
}
