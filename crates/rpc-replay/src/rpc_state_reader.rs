use starknet::core::types::BlockId;
use starknet::providers::jsonrpc::HttpTransport;
use starknet::providers::JsonRpcClient;

pub struct AsyncRpcStateReader {
    rpc_client: JsonRpcClient<HttpTransport>,
    block_id: BlockId,
}

impl AsyncRpcStateReader {
    pub fn new(rpc_client: JsonRpcClient<HttpTransport>, block_id: BlockId) -> Self {
        Self { rpc_client, block_id }
    }
}