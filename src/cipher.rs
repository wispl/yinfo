use std::{borrow::Cow, collections::HashMap};

use url::{
    form_urlencoded::{parse, Serializer},
    Url,
};

use once_cell::sync::Lazy;
use regex::{escape, Regex};

use rquickjs::Ctx;

use crate::{errors::Error, structs::VideoFormat, utils::between};

/// Operations used inside the player js code to decipher the stream url. The operations
/// are javascript code all doing a specific function, such as swapping or reversing.
#[derive(Debug)]
enum Operation {
    Swap(usize),
    Reverse(),
    Slice(usize),
    Splice(usize),
}

impl Operation {
    /// Creates a new operation with the given definition and parameter. The definition is a slice
    /// of the javascript code and the parameter is usually an integer.
    ///
    /// # Errors
    ///
    /// An error is returned if no operations can be found.
    pub fn new(def: &str, param: &str) -> Result<Self, Error> {
        // TODO: might be possible to use non-regex method and use string patterns instead
        static REVERSE: Lazy<Regex> =
            Lazy::new(|| Regex::new(r"(?:return )?a\.reverse\(\)").unwrap());
        static SLICE: Lazy<Regex> = Lazy::new(|| Regex::new(r"return a\.slice\(b\)").unwrap());
        static SPLICE: Lazy<Regex> = Lazy::new(|| Regex::new(r"a\.splice\(0,b\)").unwrap());
        static SWAP: Lazy<Regex> = Lazy::new(|| {
            Regex::new(r"var c=a\[0\];a\[0\]=a\[b%a\.length\];a\[b(?:%a.length|)\]=c(?:;return a)?")
                .unwrap()
        });

        // TODO: probably not a good idea to unconditionally unwrap to 0
        let param = param.parse::<usize>().unwrap_or(0);
        if REVERSE.is_match(def) {
            Ok(Operation::Reverse())
        } else if SLICE.is_match(def) {
            Ok(Operation::Slice(param))
        } else if SPLICE.is_match(def) {
            Ok(Operation::Splice(param))
        } else if SWAP.is_match(def) {
            Ok(Operation::Swap(param))
        } else {
            Err(Error::Cipher(format!("invalid operation '{def}'")))
        }
    }
}

// TODO: might want an enum for this
/// Extracted information from a player js required for deciphering a [`VideoFormat`].
///
/// Each player js code has dedicated code for solving streams. When we make a request to this
/// url, we get javascript code which has to be extracted. Three things are required:
/// 1. a set of operations used for deciphering the signature of the stream url
/// 2. a set of operations used for deciphering the ncode of the stream url
/// 3. the timestamp of the player js
///
/// The timestamp is required for making requests to ensure the correct player js is in used
/// for the request.
///
/// The operations for the signature are mandatory for deciphering the stream but the ncode is
/// not required but results in the download being throttled. The signature operations are relatively
/// short and can be translated natively, but the ncode operations are quite long which is why
/// quickjs is used to execute it.
pub struct Cipher {
    operations: Option<Vec<Operation>>,
    nfunc: Option<String>,
    timestamp: Option<String>,
}

impl Cipher {
    /// Creates a cipher solution for the given url after parsing the code.
    #[must_use]
    pub fn new(player_js: &str) -> Self {
        Cipher {
            operations: extract_operations(player_js),
            nfunc: extract_nfunc(player_js),
            timestamp: extract_timestamp(player_js),
        }
    }

    /// Returns the timestamp associated with this player js
    #[must_use]
    pub fn timestamp(&self) -> Option<&str> {
        self.timestamp.as_deref()
    }

    /// Apply the cipher solution to the given video format and returns a deciphered url.
    ///
    /// # Errors
    ///
    /// An error is returned if any data is missing in the video format, such as the signature or
    /// the url. Other errors include failing to decipher the signature or failing to execute js.
    pub fn apply(&self, context: &Ctx, format: &VideoFormat) -> Result<String, Error> {
        type QueryMap<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;
        // contains s, sp, and url
        let signature_map = format
            .signature_cipher
            .as_ref()
            .map(|x| parse(x.as_bytes()).collect::<QueryMap<'_>>());

        let (url, sp, s) = if let Some(mut map) = signature_map {
            // TODO: check if url is guaranteed to exist in signature
            (map.remove("url"), map.remove("sp"), map.remove("s"))
        } else {
            (format.url.as_deref().map(Cow::Borrowed), None, None)
        };
        let url = url.ok_or(Error::Cipher("No url found in video format".to_owned()))?;

        let mut url = Url::parse(&url)?;
        let mut queries: QueryMap<'_> = url.query_pairs().collect();

        if let Some(n) = queries.get("n") {
            let result = self.apply_nfunc(context, n)?;
            queries.insert(Cow::Borrowed("n"), Cow::Owned(result));
        }

        if let Some(s) = s {
            let sp = sp.unwrap_or(Cow::Borrowed("signature"));
            let result = self.apply_operations(s.as_ref())?;
            queries.insert(sp.clone(), Cow::Owned(result));
        }

        let queries = Serializer::new(String::new())
            .extend_pairs(queries.iter())
            .finish();
        url.set_query(Some(&queries));
        Ok(url.into())
    }

    fn apply_operations(&self, signature: &str) -> Result<String, Error> {
        let operations = self
            .operations
            .as_ref()
            .ok_or(Error::Cipher("failed to extract operations!".to_owned()))?;

        let mut chars: Vec<char> = signature.chars().collect();
        for op in operations {
            match op {
                Operation::Swap(x) => chars.swap(0, x % signature.len()),
                Operation::Reverse() => chars.reverse(),
                Operation::Splice(x) | Operation::Slice(x) => {
                    chars.drain(0..*x);
                }
            }
        }
        Ok(chars.into_iter().collect())
    }

    fn apply_nfunc(&self, ctx: &Ctx, nparam: &str) -> Result<String, Error> {
        let nfunc = self
            .nfunc
            .as_ref()
            .ok_or(Error::Cipher("failed to extract n function!".to_owned()))?;

        let func = format!(r#"let n={nfunc};n("{nparam}")"#);
        match ctx.eval::<String, String>(func) {
            Ok(x) => {
                if x.starts_with("enhanced_except") {
                    return Err(Error::JSEnhancedExcept);
                }
                Ok(x)
            }
            Err(_) => Err(Error::JSExecution(ctx.catch().get().unwrap())),
        }
    }
}

/// Find the name of the main function, which contains all signature operations.
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

/// Extract all operations used in the main function.
fn extract_operations(js: &str) -> Option<Vec<Operation>> {
    const MAIN_DEF: &str =
        r#"=function\([[:alpha:]]\)\{a=a\.split\(""\);(.*);return a\.join\(""\)}"#;
    const FUNC_DEF: &str = r":function\(a(?:,[[:alpha:]])*\)\{(.*?)\}";

    // Find the definition of the main function.
    let main = find_main(js)?;
    let pattern = escape(main) + MAIN_DEF;
    let pattern = Regex::new(&pattern).unwrap();
    let captures = pattern.captures(js)?;

    // Now get the name and parameter of each operation inside.
    // The operations look like this: Fo.Bo(3);Ho.Do(6) and so on.
    // Note the name here is actually after the period.
    let body = &captures[1];
    let iter = body
        .split(';')
        .map(|s| (between(s, ".", "("), between(s, ",", ")")));

    // Map the function names to their definitions.
    let names = iter
        .clone()
        .map(|(n, _)| n)
        .collect::<Vec<&str>>()
        .join("|");
    let pattern = format!("({names}){FUNC_DEF}");
    let pattern = Regex::new(&pattern).unwrap();
    let defs: HashMap<&str, &str> = pattern
        .captures_iter(js)
        .map(|c| (c.get(1).unwrap().as_str(), c.get(2).unwrap().as_str()))
        .collect();

    // Convert each operation to the rust implementation.
    iter.map(|(n, a)| Operation::new(defs.get(n).unwrap(), a))
        .collect::<Result<Vec<Operation>, Error>>()
        .ok()
}

/// Extract the entire nfunc, this always seems to have some form of enhanced except at the end.
fn extract_nfunc(js: &str) -> Option<String> {
    static NFUNC: &str = r#"(?xs)
        function\((?P<args>[^)]*)\)\s*
        (?P<code>
         \{
            var\s*[[:word:]]+\s*=\s*                            # word splitting
            (?:
                [[:word:]]+.split\(""\)
                |
                String\.prototype\.split\.call\([[:word:]]+,""\)
            ),
            .*                                                  # match whatever in between
            return\s*"enhanced_except[^}]+}                     # enhanced_except
            [^}]+                                               # match whatever until the end
         };
        )
    "#;
    static NFUNC_RE: Lazy<Regex> = Lazy::new(|| Regex::new(NFUNC).unwrap());

    let captures = NFUNC_RE.captures(js)?;
    if let (Some(args), Some(code)) = (captures.name("args"), captures.name("code")) {
        Some(format!("function({}){}", args.as_str(), code.as_str()))
    } else {
        None
    }
}

fn extract_timestamp(js: &str) -> Option<String> {
    static TIMESTAMP: Lazy<Regex> =
        Lazy::new(|| Regex::new(r"(?:signatureTimestamp|sts):(\d+)").unwrap());
    let captures = TIMESTAMP.captures(js)?;
    Some(captures[1].to_owned())
}
