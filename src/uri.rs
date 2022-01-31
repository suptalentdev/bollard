#[cfg(windows)]
use hex::FromHex;
use hyper::Uri as HyperUri;
use url::Url;

use std::borrow::Cow;
use std::ffi::OsStr;

use crate::docker::{ClientType, ClientVersion};
use crate::errors::Error;

#[derive(Debug)]
pub struct Uri<'a> {
    encoded: Cow<'a, str>,
}

impl<'a> Into<HyperUri> for Uri<'a> {
    fn into(self) -> HyperUri {
        self.encoded.as_ref().parse().unwrap()
    }
}

impl<'a> Uri<'a> {
    pub(crate) fn parse<O>(
        socket: &'a str,
        client_type: &ClientType,
        path: &'a str,
        query: Option<O>,
        client_version: &ClientVersion,
    ) -> Result<Self, Error>
    where
        O: serde::ser::Serialize,
    {
        let host_str = format!(
            "{}://{}/v{}.{}{}",
            Uri::socket_scheme(client_type),
            Uri::socket_host(socket, client_type),
            client_version.major_version,
            client_version.minor_version,
            path
        );
        let mut url = Url::parse(host_str.as_ref()).unwrap();
        url = url.join(path).unwrap();

        if let Some(pairs) = query {
            let qs = serde_urlencoded::to_string(pairs)
                .map_err(|e| crate::errors::ErrorKind::URLEncodedError { err: e })?;
            url.set_query(Some(&qs));
        }

        debug!(
            "Parsing uri: {}, client_type: {:?}, socket: {}",
            url.as_str(),
            client_type,
            socket
        );
        Ok(Uri {
            encoded: Cow::Owned(url.as_str().to_owned()),
        })
    }

    fn socket_host<P>(socket: P, client_type: &ClientType) -> String
    where
        P: AsRef<OsStr>,
    {
        match client_type {
            ClientType::Http => socket.as_ref().to_string_lossy().into_owned(),
            #[cfg(any(feature = "ssl", feature = "tls"))]
            ClientType::SSL => socket.as_ref().to_string_lossy().into_owned(),
            #[cfg(unix)]
            ClientType::Unix => hex::encode(socket.as_ref().to_string_lossy().as_bytes()),
            #[cfg(windows)]
            ClientType::NamedPipe => hex::encode(socket.as_ref().to_string_lossy().as_bytes()),
        }
    }

    fn socket_scheme(client_type: &ClientType) -> &'a str {
        match client_type {
            ClientType::Http => "http",
            #[cfg(any(feature = "ssl", feature = "tls"))]
            ClientType::SSL => "https",
            #[cfg(unix)]
            ClientType::Unix => "unix",
            #[cfg(windows)]
            ClientType::NamedPipe => "net.pipe",
        }
    }

    #[cfg(windows)]
    fn socket_path(uri: &HyperUri) -> Option<String> {
        uri.host()
            .iter()
            .filter_map(|host| {
                Vec::from_hex(host)
                    .ok()
                    .map(|raw| String::from_utf8_lossy(&raw).into_owned())
            })
            .next()
    }

    #[cfg(windows)]
    pub(crate) fn socket_path_dest(dest: &HyperUri, client_type: &ClientType) -> Option<String> {
        format!(
            "{}://{}",
            Uri::socket_scheme(client_type),
            dest.host().unwrap_or("UNKNOWN_HOST")
        )
        .parse()
        .ok()
        .and_then(|uri| Self::socket_path(&uri))
    }
}
