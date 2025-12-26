use alloy::primitives::{Address, U256, keccak256};
use alloy::sol_types::SolCall;
use evm_knowledge::{
    environment_deployment::{deploy_lock_contract, spin_up_anvil_instance},
    fetch_values,
    contract_bindings::gate_lock::GateLock
};
use revm::{Database, DatabaseRef, Evm, primitives::TransactTo, db::CacheDB};

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
    let mut cache_db = CacheDB::new(db);
    
    let slot_contract_value_mapping_bytes = U256::from(2).to_be_bytes::<32>();
    let mut ids = Vec::new();
    let mut slot = U256::ZERO;
    
    loop {
        ids.push(slot);
        let key_bytes = slot.to_be_bytes::<32>();
        let hash = keccak256([key_bytes, slot_contract_value_mapping_bytes].concat());
        let storage_slot = U256::from_be_slice(hash.as_slice());
        let values = match cache_db.storage(contract_address, storage_slot) {
            Ok(val) => val,
            Err(_) => {
                return Err(eyre::eyre!("Storage read failed"));
            }
        };
        if values == U256::ZERO {
            ids.pop();
            break;
        }
        
        let bit_for_unlocked = U256::from(1) << 224;
        let unlocked_values = values | bit_for_unlocked;
        match cache_db.insert_account_storage(contract_address, storage_slot, unlocked_values) {
            Ok(_) => {},
            Err(_) => {
                return Err(eyre::eyre!("Storage write failed"));
            }
        };
        
        let mask_for_first_64_bits = U256::from((1u128 << 64) - 1);
        let first_value = values & mask_for_first_64_bits;
        
        let values_shifted_right_64 = values >> U256::from(64);
        let mask_for_next_160_bits = (U256::from(1) << 160) - U256::from(1);
        let second_value = values_shifted_right_64 & mask_for_next_160_bits;
        
        let is_even = first_value % U256::from(2) == U256::ZERO;
        if is_even {
            slot = first_value;
        } else {
            slot = second_value;
        }
    }
    
    let call_data = GateLock::isSolvedCall { ids: ids }.abi_encode();
    
    // https://github.com/bluealloy/revm/issues/1803
    let mut evm = Evm::builder()
        .with_db(&mut cache_db)
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
    
    let output = match tx_res.result.output() {
        Some(out) => out,
        None => {
            return Err(eyre::eyre!("No output from transaction"));
        }
    };

    let is_solved = match GateLock::isSolvedCall::abi_decode_returns(&output, true) {
        Ok(res) => res,
        Err(e) => {
            return Err(eyre::eyre!("Failed to decode return value: {:?}", e));
        }
    };

    println!("Is solved: {}", is_solved.res);
    Ok(is_solved.res)
}
