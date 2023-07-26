use anyhow::{anyhow, bail, Context, Result};
use reqwest::IntoUrl;
use url::{Host, Url};

/// The base URL of a registry server.
// Note: The inner Url always has a scheme and host.
#[derive(Clone)]
pub struct RegistryUrl(Url);

impl RegistryUrl {
    /// Parses and validates the given URL into a [`RegistryUrl`].
    pub fn new(url: impl IntoUrl) -> Result<Self> {
        // Default to a HTTPS scheme if none is provided
        let mut url: Url = if !url.as_str().contains("://") {
            Url::parse(&format!("https://{url}", url = url.as_str()))
                .context("failed to parse registry server URL")?
        } else {
            url.into_url()
                .context("failed to parse registry server URL")?
        };

        match url.scheme() {
            "https" => {}
            "http" => {
                // Only allow HTTP connections to loopback
                match url
                    .host()
                    .ok_or_else(|| anyhow!("expected a host for URL `{url}`"))?
                {
                    Host::Domain(d) => {
                        if d != "localhost" {
                            bail!("an unsecured connection is not permitted to `{d}`");
                        }
                    }
                    Host::Ipv4(ip) => {
                        if !ip.is_loopback() {
                            bail!("an unsecured connection is not permitted to address `{ip}`");
                        }
                    }
                    Host::Ipv6(ip) => {
                        if !ip.is_loopback() {
                            bail!("an unsecured connection is not permitted to address `{ip}`");
                        }
                    }
                }
            }
            _ => bail!("expected a HTTPS scheme for URL `{url}`"),
        }

        // Normalize by appending a '/' if missing
        if !url.path().ends_with('/') {
            url.set_path(&(url.path().to_string() + "/"));
        }

        Ok(Self(url))
    }

    /// Returns a mostly-human-readable string that identifies the registry and
    /// contains only the characters `[0-9a-zA-Z-._]`. This string is
    /// appropriate to use with external systems that can't accept arbitrary
    /// URLs such as file system paths.
    pub fn safe_label(&self) -> String {
        // Host
        let mut label = match self.0.host().unwrap() {
            Host::Domain(domain) => domain.to_string(),
            Host::Ipv4(ip) => ip.to_string(),
            Host::Ipv6(ip) => format!("ipv6_{ip}").replace(':', "."),
        };
        // Port (if not the scheme default)
        if let Some(port) = self.0.port() {
            label += &format!("-{port}");
        }
        // Path (if not empty)
        let path = self.0.path().trim_matches('/');
        if !path.is_empty() {
            label += "_";
            // The path is already urlencoded; we just need to replace a few chars.
            for ch in path.chars() {
                match ch {
                    '/' => label += "_",
                    '%' => label += ".",
                    '*' => label += ".2A",
                    '.' => label += ".2E",
                    '_' => label += ".5F",
                    oth => label.push(oth),
                }
            }
        }
        label
    }

    pub(crate) fn into_url(self) -> Url {
        self.0
    }

    pub(crate) fn join(&self, path: &str) -> String {
        // Url::join can only fail if the base is relative or if the result is
        // very large (>4GB), neither of which should be possible in this lib.
        self.0.join(path).unwrap().to_string()
    }
}

impl std::str::FromStr for RegistryUrl {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl std::fmt::Display for RegistryUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Debug for RegistryUrl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("RegistryUrl")
            .field(&self.0.as_str())
            .finish()
    }
}

impl From<RegistryUrl> for Url {
    fn from(value: RegistryUrl) -> Self {
        value.into_url()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn must_parse(input: &str) -> RegistryUrl {
        RegistryUrl::new(input)
            .unwrap_or_else(|err| panic!("failed to parse valid input {input:?}: {err:?}"))
    }

    #[test]
    fn new_valid() {
        for (input, expected) in [
            ("bare-host", "https://bare-host/"),
            ("https://warg.io", "https://warg.io/"),
            ("https://warg.io/with/path", "https://warg.io/with/path/"),
            ("http://localhost", "http://localhost/"),
            ("http://127.0.0.1", "http://127.0.0.1/"),
            ("http://[::1]", "http://[::1]/"),
            ("http://localhost:8080", "http://localhost:8080/"),
            ("https://unchanged/", "https://unchanged/"),
        ] {
            assert_eq!(
                must_parse(input).to_string(),
                expected,
                "incorrect output for input {input:?}"
            )
        }
    }

    #[test]
    fn new_invalid() {
        for input in [
            "invalid:url",
            "bad://scheme",
            "http://insecure-domain",
            "http://6.6.6.6/insecure/ipv4",
            "http://[abcd::1234]/insecure/ipv6",
        ] {
            let res = RegistryUrl::new(input);
            assert!(
                res.is_err(),
                "input {input:?} should have failed; got {res:?}"
            );
        }
    }

    #[test]
    fn safe_label_works() {
        for (input, expected) in [
            ("warg.io", "warg.io"),
            ("http://localhost:80", "localhost"),
            ("example.com/with/path", "example.com_with_path"),
            ("port:1234", "port-1234"),
            ("port:1234/with/path", "port-1234_with_path"),
            ("https://1.2.3.4:1234/1234", "1.2.3.4-1234_1234"),
            ("https://[abcd::1234]:5678", "ipv6_abcd..1234-5678"),
            ("syms/splat*dot.lowdash_", "syms_splat.2Adot.2Elowdash.5F"),
            ("☃︎/☃︎", "xn--n3h_.E2.98.83.EF.B8.8E"), // punycode host + percent-encoded path
        ] {
            let url = must_parse(input);
            assert_eq!(url.safe_label(), expected);
        }
    }
}
