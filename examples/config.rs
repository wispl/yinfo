use yinfo::{clients::ClientConfig, clients::ClientType, Config, Innertube};

#[tokio::main]
async fn main() {
    let config = Config {
        configs: vec![ClientConfig::new(ClientType::Web)],
        retry_limit: 2,
        ..Default::default()
    };
    let innertube = Innertube::new(config).unwrap();
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
