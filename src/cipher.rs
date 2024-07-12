use std::{
    collections::HashMap,
    borrow::Cow,
};

use url::{
    Url,
    form_urlencoded::{parse, Serializer},
};

use once_cell::sync::Lazy;
use regex::{Regex, escape};

use rquickjs::{async_with, AsyncContext};

use crate::{
    structs::VideoFormat,
    errors::Error,
    utils::between,
};

static REVERSE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?:return )?a\.reverse\(\)").unwrap());
static SLICE: Lazy<Regex> = Lazy::new(|| Regex::new(r"return a\.slice\(b\)").unwrap());
static SPLICE: Lazy<Regex> = Lazy::new(|| Regex::new(r"a\.splice\(0,b\)").unwrap());
static SWAP: Lazy<Regex> = Lazy::new(||
    Regex::new(r"var c=a\[0\];a\[0\]=a\[b%a\.length\];a\[b(?:%a.length|)\]=c(?:;return a)?").unwrap()
);

#[derive(Debug)]
pub enum Operation {
    Swap(i32),
    Reverse(),
    Slice(i32),
    Splice(i32),
}

impl Operation {
    pub fn new(def: &str, param: &str) -> Self {
        let param = param.parse::<i32>().unwrap_or(0);
        if REVERSE.is_match(def) {
            Operation::Reverse()
        } else if SLICE.is_match(def) {
            Operation::Slice(param)
        } else if SPLICE.is_match(def) {
            Operation::Splice(param)
        } else if SWAP.is_match(def) {
            Operation::Swap(param)
        } else {
            // TODO: handle error
            Operation::Reverse()
        }
    }
}

pub struct Cipher {
    operations: Option<Vec<Operation>>,
    nfunc: Option<String>,
    timestamp: Option<String>,
}

impl Cipher {
    pub fn new(player_js: &str) -> Self {
        Cipher {
            operations: extract_operations(player_js),
            nfunc: extract_nfunc(player_js),
            timestamp: extract_timestamp(player_js),
        }
    }

    pub fn timestamp(&self) -> Option<&str> {
        self.timestamp.as_deref()
    }

    pub async fn apply(
        &self,
        context: &AsyncContext,
        format: &VideoFormat
    ) -> Result<String, Error> {
        type QueryMap<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;
        // contains s, sp, and url
        let signature_map = format.signature_cipher.as_ref()
            .map(|x| parse(x.as_bytes()).collect::<QueryMap<'_>>());

        if let Some(map) = signature_map {
            let url = map.get("url")
                .ok_or(Error::MissingField("url parameter", "VideoFormat.signature"))?;
            let mut url = Url::parse(url)?;
            let mut queries: QueryMap<'_> = url.query_pairs().collect();

            if let Some(s) = map.get("s") {
                let sp = map.get("sp").unwrap_or(&Cow::Borrowed("signature"));
                let result = self.apply_operations(s)?;
                queries.insert(sp.clone(), Cow::Owned(result));
            }

            if let Some(n) = queries.get("n") {
                let result = self.apply_nfunc(context, n).await?;
                queries.insert(Cow::Borrowed("n"), Cow::Owned(result));
            }
            let queries = Serializer::new(String::new())
                .extend_pairs(queries.iter())
                .finish();
            url.set_query(Some(&queries));
            Ok(url.into())
        } else {
            format.url.clone().ok_or(Error::MissingField("url", "VideoFormat"))
        }
    }

    pub fn apply_operations(&self, signature: &str) -> Result<String, Error> {
        let operations = self.operations.as_ref()
            .ok_or(Error::Cipher("failed to extract operations!".to_owned()))?;

        let mut chars: Vec<char> = signature.chars().collect();
        for op in operations {
            match op {
                Operation::Swap(x) => {
                    let pos = *x as usize % signature.len();
                    chars.swap(0, pos);
                }
                Operation::Reverse() => chars.reverse(),
                Operation::Splice(x) | Operation::Slice(x) => {
                    let end = *x as usize;
                    chars.drain(0..end);
                }
            }
        }
        Ok(chars.into_iter().collect())
    }

    // failing to apply nfunc is not an error, the video is still playable, just throttled,
    // when that is the case, None is returned.
    async fn apply_nfunc(&self, ctx: &AsyncContext, nparam: &str) -> Result<String, Error> {
        let nfunc = self.nfunc.as_ref()
            .ok_or(Error::Cipher("failed to extract n function!".to_owned()))?;

        let func = format!(r#"let n={nfunc};n("{nparam}")"#);
        async_with!(ctx => |ctx| {
            match ctx.eval::<String, String>(func) {
                Ok(x) => {
                    if x.starts_with("enhanced_except") {
                        return Err(Error::JSEnhancedExcept)
                    }
                    Ok(x)
                }
                Err(_) => Err(Error::JSExecution(ctx.catch().get().unwrap()))
            }
        }).await
    }
}

fn find_main(js: &str) -> Option<&str> {
    static CANDIDATES: &[&str; 6] = &[
        r"\b[cs]\s*&&\s*[adf]\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*([a-zA-Z0-9$]+)\(",
        r"\b[[:alnum:]]+\s*&&\s*[[:alnum:]]+\.set\([^,]+\s*,\s*encodeURIComponent\s*\(\s*([a-zA-Z0-9$]+)\(",
        r"\bm=([a-zA-Z0-9$]{2,})\(decodeURIComponent\(h\.s\)\)",
        r"\bc&&\(c=([a-zA-Z0-9$]{2,})\(decodeURIComponent\(c\)\)",
        r#"(?:\b|[^a-zA-Z0-9$])([a-zA-Z0-9$]{2,})\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)(?:;[a-zA-Z0-9$]{2}\.[a-zA-Z0-9$]{2}\(a,\d+\))?"#,
        r#"([a-zA-Z0-9$]+)\s*=\s*function\(\s*a\s*\)\s*\{\s*a\s*=\s*a\.split\(\s*""\s*\)"#,
    ];
    static MAIN: Lazy<Regex> = Lazy::new(|| Regex::new(&CANDIDATES.join("|")).unwrap());

    if let Some(captures) = MAIN.captures(js) {
        for i in 1..=CANDIDATES.len() {
            if let Some(val) = captures.get(i) {
                return Some(val.as_str());
            }
        }
    }
    None
}

fn extract_operations(js: &str) -> Option<Vec<Operation>> {
    const FUNC_BODY: &str = r#"=function\([[:alpha:]]\)\{a=a\.split\(""\);(.*);return a\.join\(""\)}"#;
    const FUNC_DEF: &str = r":function\(a(?:,[[:alpha:]])*\)\{(.*?)\}";

    let main = find_main(js)?;
    let pattern = escape(main) + FUNC_BODY;
    let pattern = Regex::new(&pattern).unwrap();
    let captures = pattern.captures(js)?;

    // get name and parameters of functions used inside the body
    let body = &captures[1];
    let iter = body.split(';').map(|s| (
            between(s, ".", "("),
            between(s, ",", ")")
    ));

    // find definitions of each function used
    let names = iter.clone().map(|(n, _)| n).collect::<Vec<&str>>().join("|");
    // TODO: searches are duplicated
    let pattern = Regex::new(&("(".to_owned() + &names + ")" + FUNC_DEF)).unwrap();
    let defs: HashMap<&str, &str> = pattern.captures_iter(js)
        .map(|c| (c.get(1).unwrap().as_str(), c.get(2).unwrap().as_str()))
        .collect();

    // convert to operations which are done in rust
    Some(iter.map(|(n, a)| Operation::new(defs.get(n).unwrap(), a)).collect())
}

fn find_nfunc(js: &str) -> Option<&str> {
    static NFUNC: Lazy<Regex> = Lazy::new(||
        // Regex::new(r#"\.get\("n"\)\)&&\(b=(?P<nfunc>[a-zA-Z0-9$]+)(?:\[(?P<idx>\d+)\])?\([[:alnum:]]\)"#).unwrap()
        Regex::new(r#"(?x)(?:\.get\("n"\)\)&&\(b=|b=String\.fromCharCode\(110\),c=a\.get\(b\)\)&&\(c=)
            (?P<nfunc>[a-zA-Z0-9$]+)(?:\[(?P<idx>\d+)\])?\([a-zA-Z0-9]\)"#).unwrap()
    );

    let captures = NFUNC.captures(js)?;
    let nfunc = captures.name("nfunc").unwrap().as_str();
    // the real value is actually inside an array
    if let Some(idx) = captures.name("idx") {
        // find the array definition
        let pattern = "var ".to_owned() + &escape(nfunc) + r"\s*=\s*\[(.+?)\]\s*[,;]";
        let pattern = Regex::new(&pattern).unwrap();
        // find the indexed value
        if let Some(cap) = pattern.captures(js) {
            let idx = idx.as_str().parse::<usize>().unwrap();
            let word = cap.get(1).unwrap().as_str().split(',').nth(idx).unwrap();
            return Some(word.trim());
        }
        None
    } else {
        Some(nfunc)
    }
}

pub fn extract_nfunc(js: &str) -> Option<String> {
    let name = find_nfunc(js)?;
    let pattern = format!(
        r#"(?xs)
        (?:
            function\s+{0}|
            [{{;,]\s*{0}\s*=\s*function|
            (?:var|const|let)\s+{0}\s*=\s*function
        )\s*
        \((?P<args>[^)]*)\)\s*
        (?P<code>\{{.*return\s*"enhanced_except.*?}}.+?}};)"#,
        escape(name)
    );
    let pattern = Regex::new(&pattern).unwrap();
    let captures = pattern.captures(js)?;

    if let (Some(args), Some(code)) = (captures.name("args"),captures.name("code")) {
        Some(format!("function({}){}", args.as_str(), code.as_str()))
    } else {
        None
    }
}

fn extract_timestamp(js: &str) -> Option<String> {
    static TIMESTAMP: Lazy<Regex> = Lazy::new(||
        Regex::new(r"(?:signatureTimestamp|sts):(\d+)").unwrap()
    );
    let captures = TIMESTAMP.captures(js)?;
    Some(captures[1].to_owned())
}

