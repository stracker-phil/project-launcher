//! Which links leave the app for the browser (ADR 0040): https to any host, http only to a local name.

use std::net::{Ipv4Addr, Ipv6Addr};

use url::{Host, Url};

const HOSTS_FILE: &str = "/etc/hosts";

/// Name endings reserved for, or by convention used on, local networks and dev servers.
const LOCAL_SUFFIXES: [&str; 7] = [
    ".localhost",
    ".local",
    ".test",
    ".internal",
    ".lan",
    ".home.arpa",
    ".ddev.site",
];

/// Whether `url` may open in the browser, with the names `/etc/hosts` maps counting as local.
pub fn is_web_url(url: &str) -> bool {
    let hosts = std::fs::read_to_string(HOSTS_FILE).unwrap_or_default();
    allowed(url, &host_names(&hosts))
}

/// Whether `url` may open in the browser; `local_names` are names mapped by hand, e.g. in `/etc/hosts`.
pub fn allowed(url: &str, local_names: &[String]) -> bool {
    let Ok(url) = Url::parse(url) else {
        return false;
    };
    match (url.scheme(), url.host()) {
        ("https", Some(_)) => true,
        ("http", Some(host)) => is_local(&host, local_names),
        _ => false,
    }
}

fn is_local(host: &Host<&str>, local_names: &[String]) -> bool {
    match host {
        Host::Ipv4(ip) => is_local_v4(ip),
        Host::Ipv6(ip) => is_local_v6(ip),
        Host::Domain(name) => {
            let name = name.trim_end_matches('.').to_ascii_lowercase();
            name == "localhost"
                || !name.contains('.')
                || LOCAL_SUFFIXES.iter().any(|suffix| name.ends_with(suffix))
                || local_names
                    .iter()
                    .any(|known| known.eq_ignore_ascii_case(&name))
        }
    }
}

fn is_local_v4(ip: &Ipv4Addr) -> bool {
    ip.is_loopback() || ip.is_private() || ip.is_link_local()
}

fn is_local_v6(ip: &Ipv6Addr) -> bool {
    // fc00::/7 holds the unique local addresses.
    ip.is_loopback() || (ip.segments()[0] & 0xfe00) == 0xfc00
}

/// The host names of a hosts file: every field after the address, comments left out.
pub fn host_names(hosts: &str) -> Vec<String> {
    hosts
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default())
        .flat_map(|line| line.split_whitespace().skip(1))
        .map(str::to_ascii_lowercase)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn https_to_any_host_is_allowed() {
        for url in [
            "https://example.com",
            "https://example.com/docs/page?q=1#top",
            "https://93.184.216.34/path",
            "https://localhost:8080",
        ] {
            assert!(allowed(url, &[]), "{url}");
        }
    }

    #[test]
    fn http_is_allowed_to_local_hosts_and_addresses() {
        for url in [
            "http://localhost",
            "http://localhost:8080",
            "http://127.0.0.1",
            "http://10.1.2.3",
            "http://192.168.0.5:3000",
            "http://172.16.4.4",
            "http://169.254.10.10",
            "http://[::1]",
            "http://[fd00::1]",
            "http://devbox",
            "http://shop.localhost",
            "http://printer.local",
            "http://app.test",
            "http://db.internal",
            "http://nas.lan",
            "http://router.home.arpa",
            "http://demo.ddev.site",
            "http://APP.TEST",
            "http://app.test.",
            "http://Demo.DDEV.Site/admin",
        ] {
            assert!(allowed(url, &[]), "{url}");
        }
    }

    #[test]
    fn http_is_allowed_to_a_name_mapped_by_hand() {
        let local = names(&["automation.acme"]);

        assert!(allowed("http://automation.acme", &local));
        assert!(allowed("http://Automation.Acme:9000/ui", &local));
    }

    #[test]
    fn http_is_refused_to_public_hosts() {
        let local = names(&["automation.acme"]);
        for url in [
            "http://example.com",
            "http://8.8.8.8",
            "http://acmetest.com",
            "http://nottest.com",
            "http://other.acme",
            "http://172.32.0.1",
        ] {
            assert!(!allowed(url, &local), "{url}");
        }
    }

    #[test]
    fn schemes_other_than_http_and_https_are_refused() {
        for url in [
            "file:///Applications/Calculator.app",
            "javascript:alert(1)",
            "ftp://example.com",
            "mailto:someone@example.com",
            "demoapp://open",
            "-a Calculator",
            "",
            "${url}/docs",
            "https://",
            "http://",
            "/relative/path",
            "example.com",
        ] {
            assert!(!allowed(url, &names(&["example.com"])), "{url}");
        }
    }

    #[test]
    fn host_names_skip_comments_and_blank_lines() {
        let hosts = "# Host database\n\n   \n127.0.0.1\tlocalhost\n# 10.0.0.1 hidden.acme\n";

        assert_eq!(host_names(hosts), ["localhost"]);
    }

    #[test]
    fn host_names_take_every_name_after_the_address_lowercased() {
        assert_eq!(
            host_names("127.0.0.1 Automation.Acme db.acme # local\n::1 Dev.Acme"),
            ["automation.acme", "db.acme", "dev.acme"]
        );
    }
}
