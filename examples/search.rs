use yinfo::{
    clients::{ClientConfig, ClientType},
    innertube::Innertube,
};

use std::time::Instant;

#[tokio::main]
async fn main() {
    let reqwest = reqwest::Client::new();
    let client = ClientConfig::new(ClientType::Web);
    let innertube = Innertube::new(reqwest.clone(), client).unwrap();

    let before = Instant::now();
    let search = innertube.search("sabaton to hell and back").await.unwrap();
    println!("Elapsed time: {:.2?}", before.elapsed());

    println!("{:#?}", search);
}
