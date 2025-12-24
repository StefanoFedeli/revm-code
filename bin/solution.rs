use alloy::primitives::{Address, U256};
use alloy::sol_types::SolCall;
use evm_knowledge::{
    environment_deployment::{deploy_lock_contract, spin_up_anvil_instance},
    fetch_values,
    contract_bindings::gate_lock::GateLock
};
use revm::{DatabaseRef, Evm, primitives::TransactTo, db::WrapDatabaseRef};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let controls = spin_up_anvil_instance().await?;
    let payload = fetch_values();

    let deploy_address = deploy_lock_contract(&controls, payload).await?;

    assert!(solve(deploy_address, controls).await?);
    Ok(())
}

// your solution goes here.
async fn solve<DB: DatabaseRef>(contract_address: Address, db: DB) -> eyre::Result<bool> 
{
    let mut wrapped_db = WrapDatabaseRef(db);
    
    let call_data = GateLock::isSolvedCall { ids: vec![U256::ZERO] }.abi_encode();
    
    let mut evm = Evm::builder()
        .with_db(&mut wrapped_db)
        .modify_tx_env(|tx| {
            tx.caller = Address::ZERO;
            tx.transact_to = TransactTo::Call(contract_address);
            tx.data = call_data.into();
        })
        .build();
    
    let tx_res = match evm.transact() {
        Ok(res) => res,
        Err(_) => {
            return Err(eyre::eyre!("EVM transaction failed!"));
        }
    };

    let output = tx_res.result.output().unwrap_or_default();
    let is_solved = match GateLock::isSolvedCall::abi_decode_returns(&output, true) {
        Ok(res) => res,
        Err(_) => {
            return Err(eyre::eyre!("Failed to decode return value"));
        }
    };

    Ok(is_solved.res)
}
