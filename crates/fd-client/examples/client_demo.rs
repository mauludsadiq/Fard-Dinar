use fd_client::{Client, Wallet};

fn main() {
    let client = Client::new("http://127.0.0.1:8085");

    let info = client.get_info().unwrap();
    println!("info: {}", info);

    let registry = client.get_registry().unwrap();
    println!("registry entries: {}", registry.entries.len());

    let wallet = Wallet::from_secret_hex("04fdb442adce1a5bd02c997e3262a0709c8ba0dce06562db18f7cfeafabf1dec").unwrap();
    println!("wallet pubkey: {}", wallet.public_key_hex());

    let tx = wallet.build_signed_transfer(
        "ed4928c628d1c2c6eae90338905995612959273a5c63f93636c14614ac8737d1",
        2000,
        0,
    );
    println!("signed transfer: {}", serde_json::to_string_pretty(&tx).unwrap());
}
