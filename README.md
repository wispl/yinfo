# yinfo

Minimal, fast enough, access to YouTube video metadata using the Innertube api.

* Async
* Video and search info
* Multiple clients

## Installing

This crate depends on [rquickjs](https://github.com/delskayn/rquickjs) and consequently QuickJS, which supports most but not all platforms.
Notably, for Windows, the `patch` utility is needed.

```toml
[dependencies.yinfo]
git = "https://github.com/wispl/yinfo.git"
```

## Quick Example

All network requests is done through the main Innertube instance, a quick
example of fetching video information.

```rust
let innertube = Innertube::new(Config::default()).unwrap();
let video_info = innertube.info("https://www.youtube.com/watch?v=5C_HPTJg5ek").await.unwrap();
```

## References and Credits

This project would not be possible without these:

* [yt-dlp](https://github.com/yt-dlp/yt-dlp)
* [lavalink](https://github.com/lavalink-devs/youtube-source)
* [pytube](https://github.com/pytube/pytube)

