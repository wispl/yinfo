use yinfo::innertube::{Innertube, Config};

#[tokio::main]
async fn main() {
    let innertube = Innertube::new(Config::default()).unwrap();
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
