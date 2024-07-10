use yinfo::{
    clients::{ClientConfig, ClientType},
    innertube::Innertube,
};

use std::fs;
use std::fs::File;
use std::io::Write;

#[tokio::main]
async fn main() {
    let reqwest = reqwest::Client::new();
    // let client = clients::get_ytcfg(reqwest.clone()).await.unwrap();
    let client = ClientConfig::new(ClientType::Web);
    // let client = clients::ClientConfig::new(clients::ClientType::Android);
    let innertune = Innertune::new(reqwest.clone(), client).await;

    // let now = Instant::now();
    // let json = fs::read_to_string("player.json").unwrap();
    // let video_details = serde_json::from_str::<structs::PlayerResponse>(&json).unwrap();
    // match url {
    //     Ok(x) => println!("{}", x),
    //     Err(why) => println!("{}", why),
    // }

    let sabaton = innertune.player("RhmHSAClG1c").await.unwrap();
    let mut file = File::create("player.json").unwrap();
    file.write_all(serde_json::to_string(&sabaton).unwrap().as_bytes()).unwrap();
    let test = innertune.decipher_format(&sabaton.best_audio()).await;
    match test {
        Ok(x) => println!("{}", x),
        Err(why) => println!("{}", why),
    }

    let i = innertune.player("RhmHSAClG1c").await.unwrap();
    let i = innertune.decipher_format(&i.best_audio()).await;
    match i {
        Ok(x) => println!("{}", x),
        Err(why) => println!("{}", why),
    }

    // let search = innertune.search("to hell and back").await.unwrap();
    // println!("{:?}", search);
    // let mut file = File::create("search.json").unwrap();
    // file.write_all(search.as_bytes()).unwrap();

    // let json = fs::read_to_string("player.json").unwrap();
    // let resuls = serde_json::from_str::<query::WebSearch>(&json).unwrap();
    // let results = search.queries();
    // println!("{:#?}", results);
}
