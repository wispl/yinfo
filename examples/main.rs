use yinfo::{
    clients::{ClientConfig, ClientType},
    innertube::Innertube,
};

#[tokio::main]
async fn main() {
    let reqwest = reqwest::Client::new();
    let client = ClientConfig::new(ClientType::Web);
    let innertube = Innertube::new(reqwest.clone(), client).unwrap();

    let video = innertube.info("RhmHSAClG1c").await.unwrap();
    println!("{:#?}", video);
    let format = video.best_audio();
    if let Some(f) = format {
        let url = innertube.decipher_format(f).await;
        match url {
            Ok(x) => println!("{}", x),
            Err(why) => println!("{}", why),
        }
    } else {
        println!("No formats found");
    }
}
