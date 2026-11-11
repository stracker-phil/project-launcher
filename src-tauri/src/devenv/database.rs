//! A project's MySQL or MariaDB database and how DataBee opens it (ADR 0011). `database` in the project file wins
//! (a `mysql://` URL, or `false` to hide one); otherwise the DDEV or wp-env setup names it. The password travels
//! in an env var, never as an argument or to the page.

use std::fs;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;
use ts_rs::TS;
use url::Url;

use super::{Connection, Server, wpenv};
use crate::run::shell::quote;
use crate::run::system::Job;

/// DataBee reads the password from here, so it never shows in the process list.
pub const PASSWORD_ENV: &str = "DATABEE_PASSWORD";
const DDEV_DEFAULT_TYPE: &str = "mariadb";

#[derive(Serialize, TS, Debug, Clone, Copy, PartialEq, Eq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "kebab-case")]
pub enum DatabaseSource {
    Ddev,
    WpEnv,
    Config,
}

/// What the detail shows: where the database comes from, its engine, `user@host/db` for a URL (never the
/// password) and why it can't be opened.
#[derive(Serialize, TS, Debug, Clone, PartialEq)]
#[ts(export, export_to = "src/api/types.gen.ts")]
#[serde(rename_all = "camelCase")]
pub struct DatabaseInfo {
    pub source: DatabaseSource,
    pub engine: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// DataBee's arguments and the password for `PASSWORD_ENV`.
#[derive(Debug, Clone, PartialEq)]
pub struct Launch {
    pub args: Vec<String>,
    pub password: String,
}

impl Launch {
    /// The `databee` command line, with the password in the env rather than an argument.
    pub fn job(&self) -> Job {
        let mut command = std::iter::once("databee".to_string())
            .chain(self.args.iter().map(|a| quote(a)))
            .collect::<Vec<_>>()
            .join(" ");
        let mut env = Vec::new();
        if !self.password.is_empty() {
            command.push_str(&format!(" --password-env {PASSWORD_ENV}"));
            env.push((PASSWORD_ENV.to_string(), self.password.clone()));
        }
        Job {
            command,
            cwd: None,
            env,
        }
    }
}

fn engine_name(kind: &str) -> Option<&'static str> {
    match kind {
        "mariadb" => Some("MariaDB"),
        "mysql" => Some("MySQL"),
        _ => None,
    }
}

/// `config.yaml`, then the `config.*.yaml` overrides in the order DDEV applies them.
fn ddev_config_files(dir: &Path) -> Vec<String> {
    let folder = dir.join(".ddev");
    let Ok(entries) = fs::read_dir(&folder) else {
        return Vec::new();
    };
    let mut overrides: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| {
            name.starts_with("config.")
                && name != "config.yaml"
                && (name.ends_with(".yaml") || name.ends_with(".yml"))
        })
        .collect();
    overrides.sort();
    std::iter::once("config.yaml".to_string())
        .chain(overrides)
        .filter_map(|name| fs::read_to_string(folder.join(name)).ok())
        .collect()
}

/// The lines after a top-level `key:`: indented ones, or a list's `- ` items at the key's own indent.
fn block<'a>(yaml: &'a str, key: &str) -> Vec<&'a str> {
    let mut lines = yaml
        .lines()
        .skip_while(|line| line.trim_end() != format!("{key}:"));
    lines.next();
    lines
        .take_while(|line| line.starts_with([' ', '\t', '-']) || line.trim().is_empty())
        .collect()
}

/// `omit_containers: [db, …]`, or the same as a block list.
fn omits_db(yaml: &str) -> bool {
    let inline = yaml
        .lines()
        .find_map(|line| line.strip_prefix("omit_containers:"))
        .unwrap_or_default();
    let words = inline
        .chars()
        .chain(block(yaml, "omit_containers").join(" ").chars())
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                ' '
            }
        })
        .collect::<String>();
    words.split_whitespace().any(|word| word == "db")
}

/// A `key: value` line of a block, without quotes.
fn field(block: &[&str], key: &str) -> Option<String> {
    block.iter().find_map(|line| {
        let value = line.trim().strip_prefix(&format!("{key}:"))?;
        let value = value.trim().trim_matches(|c| c == '"' || c == '\'');
        (!value.is_empty()).then(|| value.to_string())
    })
}

fn ddev_database(dir: &Path) -> Option<DatabaseInfo> {
    if !dir.join(".ddev/config.yaml").is_file() {
        return None;
    }
    let mut kind = DDEV_DEFAULT_TYPE.to_string();
    let mut version = None;
    for yaml in ddev_config_files(dir) {
        if omits_db(&yaml) {
            return None;
        }
        let database = block(&yaml, "database");
        kind = field(&database, "type").unwrap_or(kind);
        version = field(&database, "version").or(version);
    }
    // DataBee speaks MySQL only; a Postgres project has nothing to open.
    let engine = engine_name(&kind)?;
    Some(DatabaseInfo {
        source: DatabaseSource::Ddev,
        engine: match version {
            Some(version) => format!("{engine} {version}"),
            None => engine.into(),
        },
        detail: None,
        error: None,
    })
}

fn wp_env_database(dir: &Path) -> Option<DatabaseInfo> {
    // wp-env runs the official MariaDB image.
    wpenv::configured(dir).then(|| DatabaseInfo {
        source: DatabaseSource::WpEnv,
        engine: "MariaDB".into(),
        detail: None,
        error: None,
    })
}

/// A `mysql://` or `mariadb://` URL.
fn parse_url(value: &str) -> Option<Url> {
    let url = Url::parse(value.trim()).ok()?;
    matches!(url.scheme(), "mysql" | "mariadb").then_some(url)
}

/// A URL part with its `%XX` escapes decoded (a `+` stays a `+`).
fn decode(text: &str) -> String {
    percent_encoding::percent_decode_str(text)
        .decode_utf8_lossy()
        .into_owned()
}

fn socket_of(url: &Url) -> Option<String> {
    url.query_pairs()
        .find(|(k, _)| k == "socket")
        .map(|(_, v)| v.into_owned())
}

/// `user@host:port/db`, or the socket's file name for the host; never the password.
fn describe_url(url: &Url) -> String {
    let user = Some(decode(url.username()))
        .filter(|u| !u.is_empty())
        .unwrap_or("root".into());
    let socket = url
        .query_pairs()
        .find(|(k, _)| k == "socket")
        .map(|(_, v)| v.into_owned());
    let host = match (&socket, url.host_str()) {
        (Some(socket), _) => socket.rsplit('/').next().unwrap_or(socket).to_string(),
        (None, Some(host)) => match url.port() {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        },
        (None, None) => "127.0.0.1".into(),
    };
    let database = decode(url.path().trim_start_matches('/'));
    if database.is_empty() {
        format!("{user}@{host}")
    } else {
        format!("{user}@{host}/{database}")
    }
}

/// The database the file's `database` names.
fn configured_url(text: &str) -> DatabaseInfo {
    match parse_url(text) {
        Some(url) => DatabaseInfo {
            source: DatabaseSource::Config,
            engine: if url.scheme() == "mariadb" {
                "MariaDB"
            } else {
                "MySQL"
            }
            .into(),
            detail: Some(describe_url(&url)),
            error: None,
        },
        None => DatabaseInfo {
            source: DatabaseSource::Config,
            engine: "MySQL".into(),
            detail: None,
            error: Some("“database” must be a mysql:// URL.".into()),
        },
    }
}

/// The project's database: the file's `database` (`false` hides one, a URL names it), else what its DDEV or
/// wp-env setup runs, the configured server's first.
pub fn detect(
    dir: &Path,
    configured: Option<&Value>,
    server: Option<Server>,
) -> Option<DatabaseInfo> {
    match configured {
        Some(Value::Bool(false)) => return None,
        Some(Value::String(text)) if !text.trim().is_empty() => return Some(configured_url(text)),
        _ => {}
    }
    let detectors: [fn(&Path) -> Option<DatabaseInfo>; 2] = if server == Some(Server::WpEnv) {
        [wp_env_database, ddev_database]
    } else {
        [ddev_database, wp_env_database]
    };
    detectors.into_iter().find_map(|detect| detect(dir))
}

/// DataBee's arguments for a database a dev server runs.
pub fn server_launch(connection: &Connection, name: &str) -> Launch {
    Launch {
        args: vec![
            "-H".into(),
            connection.host.clone(),
            "-P".into(),
            connection.port.to_string(),
            "-u".into(),
            connection.user.clone(),
            "-d".into(),
            connection.database.clone(),
            "--name".into(),
            name.into(),
        ],
        password: connection.password.clone(),
    }
}

/// A socket path with `~/` and `${dir}` filled in.
fn socket_path(socket: &str, dir: &Path, home: Option<&Path>) -> String {
    let expanded = match (socket.strip_prefix("~/"), home) {
        (Some(rest), Some(home)) => home.join(rest).to_string_lossy().into_owned(),
        _ => socket.to_string(),
    };
    expanded.replace("${dir}", &dir.to_string_lossy())
}

/// DataBee's arguments for the file's URL: the URL without its password, a socket as its own option (`~` and
/// `${dir}` filled in).
pub fn config_launch(
    value: &str,
    name: &str,
    dir: &Path,
    home: Option<&Path>,
) -> Result<Launch, String> {
    let mut url =
        parse_url(value).ok_or("“database” in .project-launcher.json must be a mysql:// URL.")?;
    let password = decode(url.password().unwrap_or_default());
    // Only a URL that can't have a password refuses; a mysql:// URL can.
    let _ = url.set_password(None);
    let socket = socket_of(&url);
    let rest: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| k != "socket")
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if rest.is_empty() {
        url.set_query(None);
    } else {
        url.query_pairs_mut().clear().extend_pairs(rest);
    }
    let text = url.to_string();
    let text = text
        .strip_prefix("mariadb:")
        .map(|r| format!("mysql:{r}"))
        .unwrap_or(text);
    let mut args = vec![text];
    if let Some(socket) = socket {
        args.extend(["--socket".into(), socket_path(&socket, dir, home)]);
    }
    args.extend(["--name".into(), name.into()]);
    Ok(Launch { args, password })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tempfile::TempDir;

    const NOT_A_URL: &str = "“database” must be a mysql:// URL.";
    const NOT_A_URL_IN_FILE: &str = "“database” in .project-launcher.json must be a mysql:// URL.";

    fn project(files: &[(&str, &str)]) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (path, text) in files {
            let file = dir.path().join(path);
            fs::create_dir_all(file.parent().unwrap()).unwrap();
            fs::write(file, text).unwrap();
        }
        dir
    }

    fn info(source: DatabaseSource, engine: &str) -> DatabaseInfo {
        DatabaseInfo {
            source,
            engine: engine.into(),
            detail: None,
            error: None,
        }
    }

    fn ddev_engine(files: &[(&str, &str)]) -> Option<String> {
        let dir = project(files);
        detect(dir.path(), None, None).map(|info| {
            assert_eq!(info.source, DatabaseSource::Ddev);
            info.engine
        })
    }

    #[test]
    fn false_hides_the_database_even_when_a_setup_has_one() {
        let dir = project(&[
            (".ddev/config.yaml", "name: demo-site\n"),
            (".wp-env.json", "{}"),
        ]);

        assert_eq!(
            detect(dir.path(), Some(&json!(false)), Some(Server::Ddev)),
            None
        );
    }

    #[test]
    fn a_url_names_the_database_without_its_password() {
        for (url, engine, detail) in [
            (
                "mysql://admin:s3cret@db.example.com:3307/shop",
                "MySQL",
                "admin@db.example.com:3307/shop",
            ),
            (
                "mariadb://admin:s3cret@db.example.com/shop",
                "MariaDB",
                "admin@db.example.com/shop",
            ),
            (
                "  mysql://admin@db.example.com/shop  ",
                "MySQL",
                "admin@db.example.com/shop",
            ),
            (
                "mysql://db.example.com:3306/shop",
                "MySQL",
                "root@db.example.com:3306/shop",
            ),
            (
                "mysql://my%40user@db.example.com/shop",
                "MySQL",
                "my@user@db.example.com/shop",
            ),
            (
                "mysql://admin@db.example.com",
                "MySQL",
                "admin@db.example.com",
            ),
            (
                "mysql://app@localhost/shop?socket=/tmp/demo/mysql.sock",
                "MySQL",
                "app@mysql.sock/shop",
            ),
            (
                "mysql://app@localhost/shop?socket=mysql.sock",
                "MySQL",
                "app@mysql.sock/shop",
            ),
        ] {
            let dir = project(&[]);

            assert_eq!(
                detect(dir.path(), Some(&json!(url)), None),
                Some(DatabaseInfo {
                    source: DatabaseSource::Config,
                    engine: engine.into(),
                    detail: Some(detail.into()),
                    error: None,
                }),
                "{url}"
            );
        }
    }

    #[test]
    fn a_url_wins_over_the_setup_in_the_folder() {
        let dir = project(&[(".ddev/config.yaml", "name: demo-site\n")]);

        let detected = detect(
            dir.path(),
            Some(&json!("mysql://admin@db.example.com/shop")),
            Some(Server::Ddev),
        );

        assert_eq!(detected.map(|d| d.source), Some(DatabaseSource::Config));
    }

    #[test]
    fn a_string_that_is_not_a_mysql_url_is_an_error() {
        for text in [
            "localhost/shop",
            "postgres://db.example.com/shop",
            "https://example.com",
            "nonsense",
        ] {
            let dir = project(&[]);

            assert_eq!(
                detect(dir.path(), Some(&json!(text)), None),
                Some(DatabaseInfo {
                    source: DatabaseSource::Config,
                    engine: "MySQL".into(),
                    detail: None,
                    error: Some(NOT_A_URL.into()),
                }),
                "{text}"
            );
        }
    }

    #[test]
    fn a_blank_or_non_string_database_falls_back_to_the_setup() {
        let dir = project(&[(".ddev/config.yaml", "name: demo-site\n")]);

        for configured in [
            json!(""),
            json!("   "),
            json!(true),
            json!(3),
            json!(null),
            json!({}),
        ] {
            assert_eq!(
                detect(dir.path(), Some(&configured), None),
                Some(info(DatabaseSource::Ddev, "MariaDB")),
                "{configured}"
            );
        }
    }

    #[test]
    fn a_ddev_project_defaults_to_mariadb() {
        let dir = project(&[(".ddev/config.yaml", "name: demo-site\ntype: php\n")]);

        assert_eq!(
            detect(dir.path(), None, None),
            Some(info(DatabaseSource::Ddev, "MariaDB"))
        );
    }

    #[test]
    fn ddev_reports_the_databases_type_and_version() {
        for (yaml, expected) in [
            (
                "name: demo-site\ndatabase:\n  type: mysql\n  version: \"8.0\"\n",
                "MySQL 8.0",
            ),
            (
                "name: demo-site\ndatabase:\n  type: mariadb\n  version: '10.11'\n",
                "MariaDB 10.11",
            ),
            (
                "name: demo-site\ndatabase:\n  version: 10.6\n",
                "MariaDB 10.6",
            ),
            ("name: demo-site\ndatabase:\n  type: mysql\n", "MySQL"),
            (
                "database:\n    type: mysql\n    version: 5.7\nwebserver_type: nginx-fpm\n",
                "MySQL 5.7",
            ),
            ("type: postgres\ndatabase:\n  type: mysql\n", "MySQL"),
        ] {
            assert_eq!(
                ddev_engine(&[(".ddev/config.yaml", yaml)]).as_deref(),
                Some(expected),
                "{yaml:?}"
            );
        }
    }

    #[test]
    fn ddev_config_overrides_apply_in_sorted_order() {
        let engine = ddev_engine(&[
            (
                ".ddev/config.yaml",
                "database:\n  type: mysql\n  version: 5.7\n",
            ),
            (".ddev/config.b.yaml", "database:\n  version: 8.4\n"),
            (".ddev/config.a.yaml", "database:\n  version: 8.0\n"),
        ]);

        assert_eq!(engine.as_deref(), Some("MySQL 8.4"));
    }

    #[test]
    fn a_ddev_override_can_change_the_type() {
        let engine = ddev_engine(&[
            (
                ".ddev/config.yaml",
                "database:\n  type: mariadb\n  version: 10.11\n",
            ),
            (".ddev/config.local.yml", "database:\n  type: mysql\n"),
        ]);

        assert_eq!(engine.as_deref(), Some("MySQL 10.11"));
    }

    #[test]
    fn other_files_in_the_ddev_folder_are_not_overrides() {
        let engine = ddev_engine(&[
            (".ddev/config.yaml", "database:\n  type: mariadb\n"),
            (
                ".ddev/docker-compose.override.yaml",
                "database:\n  type: mysql\n",
            ),
            (".ddev/config.notes.txt", "database:\n  type: mysql\n"),
        ]);

        assert_eq!(engine.as_deref(), Some("MariaDB"));
    }

    #[test]
    fn a_ddev_project_that_omits_the_db_container_has_no_database() {
        for omitted in [
            "omit_containers: [db]\n",
            "omit_containers: [db, dba]\n",
            "omit_containers: [ddev-router, db]\n",
            "omit_containers:\n  - dba\n  - db\n",
            "omit_containers: [\"db\"]\n",
        ] {
            let yaml = format!("name: demo-site\n{omitted}");

            assert_eq!(
                ddev_engine(&[(".ddev/config.yaml", yaml.as_str())]),
                None,
                "{omitted:?}"
            );
        }
    }

    #[test]
    fn omitting_other_containers_keeps_the_database() {
        for omitted in [
            "omit_containers: [dba]\n",
            "omit_containers: [ddev-router, ddev-ssh-agent]\n",
            "omit_containers:\n  - dba\n",
            "omit_containers: []\n",
        ] {
            let yaml = format!("name: demo-site\n{omitted}");

            assert_eq!(
                ddev_engine(&[(".ddev/config.yaml", yaml.as_str())]).as_deref(),
                Some("MariaDB"),
                "{omitted:?}"
            );
        }
    }

    #[test]
    fn an_override_can_omit_the_db_container() {
        let engine = ddev_engine(&[
            (".ddev/config.yaml", "name: demo-site\n"),
            (".ddev/config.nodb.yaml", "omit_containers: [db]\n"),
        ]);

        assert_eq!(engine, None);
    }

    #[test]
    fn a_postgres_ddev_project_has_no_database_to_open() {
        let dir = project(&[(
            ".ddev/config.yaml",
            "database:\n  type: postgres\n  version: 16\n",
        )]);

        assert_eq!(detect(dir.path(), None, Some(Server::Ddev)), None);
    }

    #[test]
    fn a_folder_without_ddev_config_has_no_ddev_database() {
        let dir = project(&[(".ddev/config.local.yaml", "name: demo-site\n")]);

        assert_eq!(detect(dir.path(), None, Some(Server::Ddev)), None);
    }

    #[test]
    fn a_wp_env_folder_has_a_mariadb_database() {
        for file in [".wp-env.json", ".wp-env.override.json"] {
            let dir = project(&[(file, "{}")]);

            assert_eq!(
                detect(dir.path(), None, None),
                Some(info(DatabaseSource::WpEnv, "MariaDB")),
                "{file}"
            );
        }
    }

    #[test]
    fn a_folder_with_no_setup_has_no_database() {
        let dir = project(&[("package.json", "{}")]);

        assert_eq!(detect(dir.path(), None, None), None);
    }

    #[test]
    fn the_configured_server_is_asked_first() {
        let dir = project(&[
            (".ddev/config.yaml", "name: demo-site\n"),
            (".wp-env.json", "{}"),
        ]);

        for (server, source) in [
            (None, DatabaseSource::Ddev),
            (Some(Server::Ddev), DatabaseSource::Ddev),
            (Some(Server::WpEnv), DatabaseSource::WpEnv),
        ] {
            assert_eq!(
                detect(dir.path(), None, server).map(|d| d.source),
                Some(source),
                "{server:?}"
            );
        }
    }

    #[test]
    fn the_other_setup_is_used_when_the_configured_server_has_none() {
        let ddev_only = project(&[(".ddev/config.yaml", "name: demo-site\n")]);
        let wp_env_only = project(&[(".wp-env.json", "{}")]);

        assert_eq!(
            detect(ddev_only.path(), None, Some(Server::WpEnv)).map(|d| d.source),
            Some(DatabaseSource::Ddev)
        );
        assert_eq!(
            detect(wp_env_only.path(), None, Some(Server::Ddev)).map(|d| d.source),
            Some(DatabaseSource::WpEnv)
        );
    }

    #[test]
    fn a_servers_launch_names_its_connection_and_keeps_the_password_apart() {
        let connection = Connection {
            host: "127.0.0.1".into(),
            port: 32768,
            user: "demo".into(),
            password: "secret".into(),
            database: "shop".into(),
        };

        assert_eq!(
            server_launch(&connection, "Demo Site (DDEV)"),
            Launch {
                args: [
                    "-H",
                    "127.0.0.1",
                    "-P",
                    "32768",
                    "-u",
                    "demo",
                    "-d",
                    "shop",
                    "--name",
                    "Demo Site (DDEV)"
                ]
                .map(String::from)
                .to_vec(),
                password: "secret".into(),
            }
        );
    }

    fn config_launch_of(url: &str) -> Launch {
        config_launch(
            url,
            "Demo Site",
            Path::new("/work/demo-site"),
            Some(Path::new("/home/demo")),
        )
        .unwrap()
    }

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_url_launch_removes_the_password_and_returns_it_decoded() {
        assert_eq!(
            config_launch_of("mysql://admin:p%40ss%20w0rd@db.example.com:3307/shop"),
            Launch {
                args: args(&[
                    "mysql://admin@db.example.com:3307/shop",
                    "--name",
                    "Demo Site"
                ]),
                password: "p@ss w0rd".into(),
            }
        );
    }

    #[test]
    fn a_url_launch_without_a_password_has_an_empty_one() {
        assert_eq!(
            config_launch_of("mysql://admin@db.example.com/shop"),
            Launch {
                args: args(&["mysql://admin@db.example.com/shop", "--name", "Demo Site"]),
                password: String::new(),
            }
        );
    }

    #[test]
    fn a_mariadb_url_is_handed_over_as_mysql() {
        assert_eq!(
            config_launch_of("mariadb://admin:pw@db.example.com/shop").args,
            args(&["mysql://admin@db.example.com/shop", "--name", "Demo Site"])
        );
    }

    #[test]
    fn a_socket_becomes_its_own_option_before_the_name() {
        for (socket, expected) in [
            ("/tmp/demo/mysql.sock", "/tmp/demo/mysql.sock"),
            ("~/run/mysql.sock", "/home/demo/run/mysql.sock"),
            (
                "${dir}/.ddev/mysql.sock",
                "/work/demo-site/.ddev/mysql.sock",
            ),
        ] {
            assert_eq!(
                config_launch_of(&format!("mysql://app:pw@localhost/shop?socket={socket}")),
                Launch {
                    args: args(&[
                        "mysql://app@localhost/shop",
                        "--socket",
                        expected,
                        "--name",
                        "Demo Site"
                    ]),
                    password: "pw".into(),
                },
                "{socket}"
            );
        }
    }

    #[test]
    fn a_home_socket_stays_as_written_without_a_home_folder() {
        let launch = config_launch(
            "mysql://app@localhost/shop?socket=~/run/mysql.sock",
            "Demo Site",
            Path::new("/work/demo-site"),
            None,
        )
        .unwrap();

        assert_eq!(
            launch.args,
            args(&[
                "mysql://app@localhost/shop",
                "--socket",
                "~/run/mysql.sock",
                "--name",
                "Demo Site"
            ])
        );
    }

    #[test]
    fn other_query_parameters_stay_on_the_url() {
        assert_eq!(
            config_launch_of("mysql://app@localhost/shop?charset=utf8&socket=/tmp/demo/mysql.sock&ssl-mode=DISABLED").args,
            args(&[
                "mysql://app@localhost/shop?charset=utf8&ssl-mode=DISABLED",
                "--socket",
                "/tmp/demo/mysql.sock",
                "--name",
                "Demo Site",
            ])
        );
    }

    #[test]
    fn a_launch_from_something_that_is_not_a_mysql_url_fails() {
        for text in [
            "",
            "nonsense",
            "postgres://db.example.com/shop",
            "https://example.com",
        ] {
            assert_eq!(
                config_launch(text, "Demo Site", Path::new("/work/demo-site"), None),
                Err(NOT_A_URL_IN_FILE.to_string()),
                "{text:?}"
            );
        }
    }

    #[test]
    fn a_database_info_serializes_without_its_empty_fields() {
        assert_eq!(
            serde_json::to_value(info(DatabaseSource::WpEnv, "MariaDB")).unwrap(),
            json!({ "source": "wp-env", "engine": "MariaDB" })
        );
        assert_eq!(
            serde_json::to_value(DatabaseInfo {
                source: DatabaseSource::Config,
                engine: "MySQL".into(),
                detail: Some("admin@db.example.com/shop".into()),
                error: Some("broken".into()),
            })
            .unwrap(),
            json!({
                "source": "config",
                "engine": "MySQL",
                "detail": "admin@db.example.com/shop",
                "error": "broken",
            })
        );
    }
}
