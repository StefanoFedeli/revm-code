//https://doc.rust-lang.org/cargo/reference/build-scripts.html

use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=contracts/GateLock.json");

    let out_path = Path::new("src").join("generated_contracts.rs");
    let content = r#"#[rustfmt::skip]
pub mod gate_lock {
    alloy::sol!(
        #[allow(missing_docs)]
        #[sol(rpc, abi)]
        #[derive(Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        GateLock,
        "contracts/GateLock.json"
    );
}
"#;
    fs::write(out_path, content).expect("Unable to write generated_contracts.rs");
}