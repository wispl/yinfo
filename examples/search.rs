use yinfo::innertube::{Innertube, Config};

#[tokio::main]
async fn main() {
    let innertube = Innertube::new(Config::default()).unwrap();
    let search = innertube.search("sabaton to hell and back").await.unwrap();
    println!("{:#?}", search);
}
