use fd_client::Client;

fn main() {
    let client = Client::new("http://127.0.0.1:8085");

    let info = client.get_info().unwrap();
    println!("info: {}", info);

    let registry = client.get_registry().unwrap();
    println!("registry entries: {}", registry.entries.len());
}
