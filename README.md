 # Murl
 Non-stringly-typed URLs.

 Urls are often used as strings, but what they really are is a serialized
 structure with fields such as `scheme`, `host`, `path` and `query_params`.
 In fact, `murl` URLs do _not_ contain their string representation inside them.

 This crate provides the `Url` struct, which should be the preferred
 way to create and modify URLs instead of fallibly parsing and/or concatenating
 strings, without ever exposing the user to things like percent-encoding issues.
 <br>
 ## Examples

 ### Infallibly creating a URL
 ```rust
use std::str::FromStr;
use std::collections::BTreeMap;
use murl::{Url, Scheme, Host, Label};
use camino::Utf8PathBuf;
// non-fallibly creating the url
 let mut url = Url{
     scheme: Scheme::Https,
     host: Host { // the hostname is "example.com"
         name: Label::from_str("example").unwrap(),
         domains:  vec![
             Label::from_str("com").unwrap(),
         ]
     },
     port: Some(443),
     path: Utf8PathBuf::from("/some/path"),
     query: BTreeMap::from([ // query params are just strings. Escaping is done automatically
         ("key with spaces".into(), "val&with&ampersands".into()),
         ("key=with=equals".into(), "val#with#hashtag".into()),
     ]),
     fragment: None,
 };
 assert_eq!(
     url.to_string(),
     "https://example.com:443/some/path?key%20with%20spaces=val%26with%26ampersands&key%3Dwith%3Dequals=val%23with%23hashtag"
 );
```

 ### Parsing a URL from a string
 If you get a string from a user or an external process, you can fallibly parse it via `FromStr`:

 ```rust
 use std::str::FromStr;
 use std::collections::BTreeMap;
 use murl::{Url, Scheme, Host, Label};
 use camino::Utf8PathBuf;

 let parsed_url = Url::from_str("http://example.com/some/path?a=123").unwrap();
 let expected = Url{
     scheme: Scheme::Http,
     host: Host{
         name: Label::from_str("example").unwrap(),
         domains: vec![
             Label::from_str("com").unwrap()
         ],
     },
     port: None,
     path: Utf8PathBuf::from("/some/path"),
     query: BTreeMap::from([
         ("a".to_owned(), "123".to_owned())
     ]),
     fragment: None,
 };
 assert_eq!(parsed_url, expected);
 ```