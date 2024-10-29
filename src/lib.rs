#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]

pub use camino;
use percent_encoding::percent_decode_str;

use std::{collections::BTreeMap, fmt::Display, str::FromStr};
use camino::Utf8PathBuf;

const ESCAPE_SET: &percent_encoding::AsciiSet =    &percent_encoding::CONTROLS
    .add(b' ')
    .add(b'"').add(b'`')
    .add(b'<').add(b'>')
    .add(b'?').add(b'#').add(b'=').add(b'&')
    .add(b'{').add(b'}')
    .add(b'%');

#[derive(PartialEq, Eq, Copy, Clone, strum::Display, strum::AsRefStr, strum::VariantArray, Debug)]
pub enum Scheme{
    //Note: the order is important for parsing because ws is a prefix of wss
    #[strum(serialize="wss")]
    Wss,
    #[strum(serialize="ws")]
    Ws,
    #[strum(serialize="https")]
    Https,
    #[strum(serialize="http")]
    Http,
}

#[derive(thiserror::Error, Debug)]
#[error("Could not parse scheme")]
pub struct SchemeParsingError;

impl Scheme{
    pub fn parse(input: &str) -> Result<(Self, &str), SchemeParsingError>{
        use strum::VariantArray;
        for variant in Scheme::VARIANTS{
            if let Some(rest) = input.strip_prefix(variant.as_ref()){
                return Ok((*variant, rest))
            }
        }
        Err(SchemeParsingError)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LabelError{
    #[error("Value is empty")]
    Empty,
    #[error("Value contains invalid char")]
    ContainsInvalidChar,
    #[error("Value's first char is not alphanumeric")]
    FirstCharNotAlphabetic,
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// Each of the period-separated components of a hostname.
///
/// E.g. `example` and `com` in `example.com`
pub struct Label(String);

impl Display for Label{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl Label{
    /// Parses `input` until a `Label` is found, and returns the remaining input if successful.
    pub fn parse(input: &str) -> Result<(Self, &str), LabelError>{
        let (input, rest) = {
            match input.find(|c| "/.:".contains(c)){
                Some(separator_idx) => input.split_at(separator_idx),
                None => (input, "") //FIXME  : empty "" ? It should be the empty end of input
            }
        };
        let label = Self::from_str(input)?;
        Ok((label, rest))
    }
}

impl Label{
    fn char_is_allowed(c: char) -> bool{
        return c.is_alphabetic() || "_-".contains(c);
    }
}

impl FromStr for Label{
    type Err = LabelError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let first_char = value.chars().next().ok_or(LabelError::Empty)?;
        if !first_char.is_alphabetic(){
            return Err(LabelError::FirstCharNotAlphabetic)
        }
        for c in value.chars(){
            if Self::char_is_allowed(c){
                continue
            }
            return Err(LabelError::ContainsInvalidChar)
        }
        Ok(Self(value.to_owned()))
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
/// A host name like `example.com`
pub struct Host{
    // The required, leftmost `Label` of the hostname, like `vm1` in `vm1.example.com`
    pub name: Label,
    // Optional domains where the host is, like `example.com` in `vm1.example.com`
    pub domains: Vec<Label>,
}

#[derive(thiserror::Error, Debug)]
pub enum HostError{
    #[error(transparent)]
    LabelError(#[from] LabelError),
    #[error("No labels")]
    NoLabels,
}

impl Host{
    pub fn parse(input: &str) -> Result<(Self, &str), HostError>{
        let (input, rest) = match input.find(|c: char| "/:".contains(c)){
            Some(slash_idx) => input.split_at(slash_idx),
            None => (input, "")
        };

        let mut labels: Vec<Label> = input.split('.')
            .map(|raw_label| Label::from_str(raw_label))
            .collect::<Result<_, _>>()?;
        if labels.len() == 0{
            return Err(HostError::NoLabels)
        }
        let name = labels.remove(0);
        Ok((
            Host{name, domains: labels},
            rest,
        ))
    }
}

impl Display for Host{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        for domain in &self.domains{
            write!(f, ".{domain}")?;
        }
        Ok(())
    }
}

#[derive(thiserror::Error, Debug)]
pub enum UrlParsingError{
    #[error(transparent)]
    SchemeParsingError(#[from] SchemeParsingError),
    #[error("Missing separator")]
    MissingSeparator,
    #[error(transparent)]
    HostError(#[from] HostError),
    #[error("Garbled port")]
    GarbledPort,
    #[error("Missing path")]
    MissingPath,
    #[error("Path not absolute")]
    PathNotAbsolute,
    #[error("Can't percent-decode")]
    CantDecode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A structured, non-string-based URL
pub struct Url{
    /// The URL scheme, like `http` in `http://example.com/`
    pub scheme: Scheme,
    /// The URL hostname, like `example.com` in `http://example.com/`
    pub host: Host,
    /// A port number, like `80` in `http://example.com:80/`
    pub port: Option<u16>,
    /// The URL path, like `/` in `http://example.com/`
    pub path: Utf8PathBuf,
    /// The query parameters, like `a=123&b=456` in `http://example.com/?a=123&b=456`
    pub query: BTreeMap<String, String>,
    /// The URL fragment, like `paragraph_1` in `http://example.com/#paragraph_1`
    pub fragment: Option<String>,
}

impl FromStr for Url{
    type Err = UrlParsingError;
    fn from_str(input: &str) -> Result<Self, UrlParsingError>{
        let (scheme, input) = Scheme::parse(input)?;
        let input = input.strip_prefix("://").ok_or(UrlParsingError::MissingSeparator)?;
        let (host, input) = Host::parse(input)?;

        let (port, input) = match input.strip_prefix(":"){
            None => (None, input),
            Some(input) => {
                let split_idx = input.find(|c: char| !c.is_numeric()).ok_or(UrlParsingError::MissingPath)?;
                let (port_raw, input) = input.split_at(split_idx);
                let port = u16::from_str(port_raw).map_err(|_| UrlParsingError::GarbledPort)?;
                (Some(port), input)
            }
        };

        let (raw_path, raw_query, raw_fragment) = match input.find(|c: char| c == '?' || c == '#'){
            None => (input, "", ""),
            Some(separator_idx) => {
                let (raw_path, input) = input.split_at(separator_idx);
                match input.strip_prefix('#'){
                    Some(raw_fragment) => (raw_path, "", raw_fragment),
                    None => {
                        let input = input.strip_prefix('?').unwrap();
                        match input.split_once('#'){
                            Some((raw_query, raw_fragment)) => (raw_path, raw_query, raw_fragment),
                            None => (raw_path, input, ""),
                        }
                    },
                }
            },
        };

        let decoded_path = percent_encoding::percent_decode(raw_path.as_bytes())
            .decode_utf8()
            .map_err(|_| UrlParsingError::CantDecode)?;

        if raw_path.is_empty(){
            return Err(UrlParsingError::MissingPath)
        }

        let path = camino::Utf8PathBuf::from(&decoded_path);
        if !path.is_absolute(){
            return Err(UrlParsingError::PathNotAbsolute)
        }

        let mut query = BTreeMap::<String, String>::new();
        for raw_pair in raw_query.split("&"){
            let (raw_key, raw_val) = match raw_pair.split_once('='){
                None => (raw_pair, ""),
                Some((key, val)) => (key, val),
            };
            let decoded_key = percent_encoding::percent_decode_str(raw_key).decode_utf8().map_err(|_| UrlParsingError::CantDecode)?;
            let decoded_val = percent_encoding::percent_decode_str(raw_val).decode_utf8().map_err(|_| UrlParsingError::CantDecode)?;
            query.insert(decoded_key.to_string(), decoded_val.to_string());
        }

        let fragment = percent_decode_str(raw_fragment).decode_utf8().map_err(|_| UrlParsingError::CantDecode)?;

        Ok(Url{
            scheme,
            host,
            port,
            path,
            query,
            fragment: if fragment.is_empty() { None } else { Some(fragment.to_string()) },
        })
    }
}

impl Display for Url{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self{scheme, host, path, ..} = self;
        write!(f, "{scheme}://{host}")?;
        if let Some(port) = &self.port{
            write!(f, ":{port}")?;
        }
        if !path.is_absolute(){
            write!(f, "/")?;
        }
        let path_str: String = percent_encoding::percent_encode(path.as_str().as_bytes(), ESCAPE_SET).collect();
        write!(f, "{path_str}")?;
        if self.query.len() > 0 {
            write!(f, "?")?;
            for (idx, (k, v)) in self.query.iter().enumerate(){
                let separator = if idx > 0 { "&" } else {""};
                let k = percent_encoding::utf8_percent_encode(k, ESCAPE_SET);
                let v = percent_encoding::utf8_percent_encode(v, ESCAPE_SET);
                write!(f, "{separator}{k}={v}")?;
            }
        }
        if let Some(fragment) = &self.fragment{
            let fragment = percent_encoding::utf8_percent_encode(fragment, ESCAPE_SET);
            write!(f, "#{fragment}")?;
        }
        Ok(())
    }
}

impl Url{
    /// Consumes the URL and returns a URL whose path is the parent of the original one
    pub fn into_parent(mut self) -> Self{
        self.path.pop();
        self
    }
}


#[test]
fn test_parsing(){
    let mut url = Url{
        scheme: Scheme::Https,
        host: Host {
            name: Label::from_str("some_host").unwrap(),
            domains:  vec![
                Label::from_str("a").unwrap(),
                Label::from_str("b").unwrap(),
                Label::from_str("c").unwrap(),
            ]
        },
        port: Some(123),
        path: Utf8PathBuf::from_str("/some/path/path_question_mark?path_question_mark").unwrap(),
        query: BTreeMap::from([
            ("space space".into(), "ampersand&ampersand".into()),
            ("equals=equals".into(), "hashtag#hashtag".into()),
        ]),
        fragment: Some("inner_fragment".into()),
    };

    let url_param = Url{
        scheme: Scheme::Https,
        host: Host {
            name: Label::from_str("param_host").unwrap(),
            domains:  vec![
            ]
        },
        port: Some(123),
        path: Utf8PathBuf::from_str("/some/path/param_question_mark?param_question_mark").unwrap(),
        query: BTreeMap::from([
            ("space space".into(), "ampersand&ampersand".into()),
            ("equals=equals".into(), "hashtag#hashtag".into()),
        ]),
        fragment: Some("inner_fragment".into()),
    };

    url.query.insert("some_url".to_owned(), url_param.to_string());

    let raw: String = url.to_string();
    let parsed = Url::from_str(&raw).unwrap();

    println!("orig: {url}");
    println!("pars: {parsed}");

    assert_eq!(url, parsed);


    let parsed_url_param = Url::from_str(parsed.query.get("some_url").unwrap()).unwrap();
    assert_eq!(url_param, parsed_url_param);
}

