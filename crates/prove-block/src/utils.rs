use std::collections::HashSet;
use cairo_vm::Felt252;
use starknet_core::types::{ExecuteInvocation, FunctionInvocation, TransactionTrace, TransactionTraceWithHash};

pub(crate) fn get_subcalled_contracts_from_tx_traces(
    traces: &[TransactionTraceWithHash],
) -> (HashSet<Felt252>, HashSet<Felt252>) {
    let mut contracts_subcalled: HashSet<Felt252> = HashSet::new();
    let mut classes_subcalled: HashSet<Felt252> = HashSet::new();
    for trace in traces {
        match &trace.trace_root {
            TransactionTrace::Invoke(invoke_trace) => {
                if let Some(inv) = &invoke_trace.validate_invocation {
                    process_function_invocations(inv, &mut contracts_subcalled, &mut classes_subcalled);
                }
                if let ExecuteInvocation::Success(inv) = &invoke_trace.execute_invocation {
                    process_function_invocations(inv, &mut contracts_subcalled, &mut classes_subcalled);
                }
                if let Some(inv) = &invoke_trace.fee_transfer_invocation {
                    process_function_invocations(inv, &mut contracts_subcalled, &mut classes_subcalled);
                }
            }
            TransactionTrace::Declare(declare_trace) => {
                if let Some(inv) = &declare_trace.validate_invocation {
                    process_function_invocations(inv, &mut contracts_subcalled, &mut classes_subcalled);
                }
                if let Some(inv) = &declare_trace.fee_transfer_invocation {
                    process_function_invocations(inv, &mut contracts_subcalled, &mut classes_subcalled);
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
                    process_function_invocations(inv, &mut contracts_subcalled, &mut classes_subcalled);
                }
                if let Some(inv) = &deploy_trace.fee_transfer_invocation {
                    process_function_invocations(inv, &mut contracts_subcalled, &mut classes_subcalled);
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