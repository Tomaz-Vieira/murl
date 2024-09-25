pub use camino;

use std::{collections::{BTreeMap, HashMap}, fmt::Display, ops::Deref, str::FromStr};
use camino::Utf8PathBuf;

const FRAGMENT: &percent_encoding::AsciiSet = &percent_encoding::CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');
const QUERY: &percent_encoding::AsciiSet =    &percent_encoding::CONTROLS.add(b' ').add(b'"').add(b'#').add(b'<').add(b'>').add(b'=');

pub enum Protocol{
    Ws,
    Wss,
    Http,
    Https,
}
impl Display for Protocol{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            Self::Ws => write!(f, "ws"),
            Self::Wss => write!(f, "wss"),
            Self::Http => write!(f, "http"),
            Self::Https => write!(f, "https"),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum HostnameLabelError{
    #[error("Value is empty")]
    Empty(String),
    #[error("Value contains invalid char")]
    ContainsInvalidChar(String),
    #[error("Value's first char is not alphanumeric")]
    FirstCharNotAlphabetic(String),
}

pub struct Label(String);

impl Deref for Label{
    type Target = str;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Display for Label{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let self_str: &str = self.as_ref();
        write!(f, "{self_str}")
    }
}

impl TryFrom<String> for Label{
    type Error = HostnameLabelError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        let Some(first_char) = value.chars().next() else{
            return Err(HostnameLabelError::Empty(value))
        };
        if !first_char.is_alphabetic(){
            return Err(HostnameLabelError::FirstCharNotAlphabetic(value))
        }
        if value.chars().find(|c| !c.is_alphanumeric()).is_some(){
            return Err(HostnameLabelError::ContainsInvalidChar(value))
        }
        Ok(Self(value))
    }
}

pub struct Host{
    pub name: Label,
    pub domains: Vec<Label>,
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

// pub struct Query

pub struct Url{
    pub protocol: Protocol,
    pub host: Host,
    pub port: Option<u16>,
    pub path: Utf8PathBuf,
    pub query: BTreeMap<String, String>,
    pub fragment: Option<String>,
}

impl Display for Url{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self{protocol, host, path, ..} = self;
        write!(f, "{protocol}://{host}")?;
        if let Some(port) = &self.port{
            write!(f, ":{port}")?;
        }
        write!(f, "{path}")?;
        if self.query.len() > 0 {
            write!(f, "?")?;
            for (idx, (k, v)) in self.query.iter().enumerate(){
                let separator = if idx > 0 { "&" } else {""};
                let k = percent_encoding::utf8_percent_encode(k, QUERY);
                let v = percent_encoding::utf8_percent_encode(v, QUERY);
                write!(f, "{separator}{k}={v}")?;
            }
        }
        if let Some(fragment) = &self.fragment{
            let fragment = percent_encoding::utf8_percent_encode(fragment, FRAGMENT);
            write!(f, "#{fragment}")?;
        }
        Ok(())
    }
}

impl Url{
    pub fn into_parent(mut self) -> Self{
        self.path.pop();
        self
    }
}

#[test]
fn test_display(){
    let url = Url{
        protocol: Protocol::Http,
        host: Host {
            name: "localhost".to_owned().try_into().unwrap(), 
            domains:  vec![
                "some".to_owned().try_into().unwrap(),
                "domain".to_owned().try_into().unwrap(),
                "com".to_owned().try_into().unwrap(),
            ]},
        port: None,
        path: Utf8PathBuf::from_str("/some/path").unwrap(),
        query: BTreeMap::from([
            ("param1".into(), "value1".into()),
            ("param2".into(), "value2".into()),
        ]),
        fragment: Some("my fragment".into()),
    };
    let url_str = url.to_string();
    assert_eq!(url_str, "http://localhost.some.domain.com/some/path?param1=value1&param2=value2#my%20fragment")
    
}
