use flate2::read::GzDecoder;
use starknet_core::types::contract::legacy::{
    LegacyContractClass, LegacyEntrypointOffset, RawLegacyAbiEntry, RawLegacyConstructor,
    RawLegacyEntryPoint, RawLegacyEntryPoints, RawLegacyEvent, RawLegacyFunction,
    RawLegacyL1Handler, RawLegacyMember, RawLegacyStruct,
};
use starknet_core::types::{
    CompressedLegacyContractClass, LegacyContractAbiEntry, LegacyContractEntryPoint,
    LegacyEntryPointsByType, LegacyFunctionAbiEntry, LegacyFunctionAbiType, LegacyStructMember,
};
use std::io::Read;

#[derive(thiserror::Error, Debug)]
pub enum LegacyContractDecompressionError {
    #[error("Failed to deserialize the program as JSON: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Failed to decompress the program: {0}")]
    Decompression(#[from] std::io::Error),
}

pub(crate) fn decompress_starknet_core_contract_class(
    compressed_legacy_class: CompressedLegacyContractClass,
) -> Result<LegacyContractClass, LegacyContractDecompressionError> {
    let mut program_str = String::new();
    let mut decoder = GzDecoder::new(compressed_legacy_class.program.as_slice());
    decoder.read_to_string(&mut program_str)?;

    let program: starknet_core::types::contract::legacy::LegacyProgram =
        serde_json::from_str(&program_str)?;
    let abi = compressed_legacy_class
        .abi
        .unwrap_or_default()
        .into_iter()
        .map(raw_legacy_abi_entry_from_legacy_contract_abi_entry)
        .collect();

    Ok(LegacyContractClass {
        abi,
        entry_points_by_type: raw_legacy_entrypoints_from_legacy_entrypoints(
            compressed_legacy_class.entry_points_by_type,
        ),
        program,
    })
}

fn raw_legacy_abi_entry_from_legacy_contract_abi_entry(
    legacy_contract_abi_entry: LegacyContractAbiEntry,
) -> RawLegacyAbiEntry {
    match legacy_contract_abi_entry {
        LegacyContractAbiEntry::Function(entry) => {
            raw_abi_entry_from_legacy_function_abi_entry(entry)
        }
        LegacyContractAbiEntry::Struct(entry) => RawLegacyAbiEntry::Struct(RawLegacyStruct {
            members: entry
                .members
                .into_iter()
                .map(raw_legacy_member_from_legacy_struct_member)
                .collect(),
            name: entry.name,
            size: entry.size,
        }),
        LegacyContractAbiEntry::Event(entry) => RawLegacyAbiEntry::Event(RawLegacyEvent {
            data: entry.data,
            keys: entry.keys,
            name: entry.name,
        }),
    }
}

fn raw_abi_entry_from_legacy_function_abi_entry(
    entry: LegacyFunctionAbiEntry,
) -> RawLegacyAbiEntry {
    match entry.r#type {
        LegacyFunctionAbiType::Function => RawLegacyAbiEntry::Function(RawLegacyFunction {
            inputs: entry.inputs,
            name: entry.name,
            outputs: entry.outputs,
            state_mutability: entry.state_mutability,
        }),
        LegacyFunctionAbiType::Constructor => {
            RawLegacyAbiEntry::Constructor(RawLegacyConstructor {
                inputs: entry.inputs,
                name: entry.name,
                outputs: entry.outputs,
            })
        }
        LegacyFunctionAbiType::L1Handler => RawLegacyAbiEntry::L1Handler(RawLegacyL1Handler {
            inputs: entry.inputs,
            name: entry.name,
            outputs: entry.outputs,
        }),
    }
}

fn raw_legacy_member_from_legacy_struct_member(member: LegacyStructMember) -> RawLegacyMember {
    RawLegacyMember {
        name: member.name,
        offset: member.offset,
        r#type: member.r#type,
    }
}

fn raw_legacy_entrypoints_from_legacy_entrypoints(
    legacy_entry_points_by_type: LegacyEntryPointsByType,
) -> RawLegacyEntryPoints {
    RawLegacyEntryPoints {
        constructor: legacy_entry_points_by_type
            .constructor
            .into_iter()
            .map(raw_legacy_entrypoint_from_legacy_entrypoint)
            .collect(),
        external: legacy_entry_points_by_type
            .external
            .into_iter()
            .map(raw_legacy_entrypoint_from_legacy_entrypoint)
            .collect(),
        l1_handler: legacy_entry_points_by_type
            .l1_handler
            .into_iter()
            .map(raw_legacy_entrypoint_from_legacy_entrypoint)
            .collect(),
    }
}

fn raw_legacy_entrypoint_from_legacy_entrypoint(
    legacy_entry_point: LegacyContractEntryPoint,
) -> RawLegacyEntryPoint {
    RawLegacyEntryPoint {
        offset: LegacyEntrypointOffset::U64AsInt(legacy_entry_point.offset),
        selector: legacy_entry_point.selector,
    }
}
