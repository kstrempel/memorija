//! Command-line interface of the binary (spec: `cli`).

use std::net::{IpAddr, Ipv4Addr};

use clap::error::ErrorKind;
use clap::{Args, CommandFactory, Parser, Subcommand};

/// Default port for the HTTP transport.
pub const DEFAULT_HTTP_PORT: u16 = 8080;

/// Default port for the web viewer (one above the MCP HTTP transport, so
/// both can run at once).
pub const DEFAULT_WEB_PORT: u16 = 8081;

/// Default bind address for the HTTP transport.
pub const DEFAULT_HTTP_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Version line reported by `--version`: the crate version plus the UTC
/// build date captured by `build.rs` (design D2).
pub const VERSION_LINE: &str = concat!(
    env!("CARGO_PKG_VERSION"),
    " (built ",
    env!("BUILD_DATE"),
    ")"
);

/// Version line with the tool name, shown under the `help` usage.
pub const AFTER_HELP: &str = concat!(
    "memorija ",
    env!("CARGO_PKG_VERSION"),
    " (built ",
    env!("BUILD_DATE"),
    ")"
);

#[derive(Debug, Parser)]
#[command(
    name = "memorija",
    about = "Persistent memory files for the user and their work across sessions",
    version = VERSION_LINE,
    after_help = AFTER_HELP
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Start the MCP server
    Serve(ServeArgs),
    /// Start the local, read-only web viewer
    Web(WebArgs),
    /// List every memory file with its line count
    List,
    /// Print one memory file, or one of its recorded versions
    Show(ShowArgs),
    /// Print a file's version timeline (version, time, cause)
    Versions {
        /// The memory file to inspect
        file: String,
    },
    /// Print the global change log, newest first
    Changes {
        /// Timespan: <n>h, <n>d, <n>w, <n>m (a rolling 30-day window),
        /// YYYY-MM-DD, START..END, or all [default: 7d]
        #[arg(value_parser = timespan)]
        timespan: Option<crate::inspect::Timespan>,
    },
}

/// The clap value parser for the `changes` timespan: grammar errors surface
/// as usage errors naming the value.
fn timespan(spec: &str) -> Result<crate::inspect::Timespan, String> {
    crate::inspect::parse_timespan(spec)
}

#[derive(Debug, Clone, Default, Args)]
pub struct ShowArgs {
    /// The memory file to print
    pub file: String,

    /// Print the content recorded at this version instead of the live file
    pub version: Option<u64>,

    /// Print raw content even when stdout is a terminal
    #[arg(long)]
    pub raw: bool,

    /// Render markdown even when stdout is piped
    #[arg(long)]
    pub render: bool,
}

#[derive(Debug, Clone, Default, Args)]
pub struct ServeArgs {
    /// Serve over Streamable HTTP instead of stdio
    #[arg(long)]
    pub http: bool,

    /// Serve over stdio (the default)
    #[arg(long)]
    pub stdio: bool,

    /// Port the HTTP server listens on (requires --http)
    #[arg(long, requires = "http", value_parser = clap::value_parser!(u16).range(1..))]
    pub port: Option<u16>,

    /// Address the HTTP server binds to (requires --http; default 127.0.0.1)
    #[arg(long, requires = "http")]
    pub host: Option<IpAddr>,
}

/// Which transport to serve MCP over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Transport {
    Stdio,
    Http { host: IpAddr, port: u16 },
}

/// Arguments of the `web` subcommand (spec: `web-ui`): a local, read-only
/// viewer binding to the loopback by default, on port 8081 so the MCP HTTP
/// transport (8080) can run beside it.
#[derive(Debug, Clone, Args)]
pub struct WebArgs {
    /// Port the web server listens on
    #[arg(long, default_value_t = DEFAULT_WEB_PORT, value_parser = clap::value_parser!(u16).range(1..))]
    pub port: u16,

    /// Address the web server binds to (default 127.0.0.1)
    #[arg(long, default_value_t = DEFAULT_HTTP_HOST)]
    pub host: IpAddr,

    /// Do not open the main page in the system browser
    #[arg(long)]
    pub no_open: bool,
}

impl ServeArgs {
    /// Enforce the mutual exclusion of `--stdio` and `--http` that cannot be
    /// declared via `conflicts_with`: clap drops `requires` requirements that
    /// clash with a conflict, which would silently accept
    /// `--stdio --port ...`.
    pub fn validate(&self) -> Result<(), String> {
        if self.http && self.stdio {
            return Err("--stdio and --http cannot be combined; choose one transport".to_string());
        }
        Ok(())
    }

    pub fn transport(&self) -> Transport {
        if self.http {
            Transport::Http {
                host: self.host.unwrap_or(DEFAULT_HTTP_HOST),
                port: self.port.unwrap_or(DEFAULT_HTTP_PORT),
            }
        } else {
            Transport::Stdio
        }
    }
}

/// Parse the process arguments into the resolved subcommand, applying the
/// bare-invocation default (`serve --stdio`) and the serve-argument
/// validation. Invalid input exits with a clap usage error.
pub fn parse_command() -> Command {
    let command = match Cli::try_parse() {
        Ok(cli) => cli.command.unwrap_or(Command::Serve(ServeArgs::default())),
        Err(error) => error.exit(),
    };
    if let Command::Serve(args) = &command {
        if let Err(message) = args.validate() {
            Cli::command()
                .error(ErrorKind::ArgumentConflict, message)
                .exit();
        }
    }
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn parse(args: &[&str]) -> Result<ServeArgs, clap::Error> {
        Cli::try_parse_from(std::iter::once("memorija").chain(args.iter().copied())).map(|cli| {
            match cli.command {
                Some(Command::Serve(args)) => args,
                _ => ServeArgs::default(),
            }
        })
    }

    fn parse_command(args: &[&str]) -> Result<Command, clap::Error> {
        Cli::try_parse_from(std::iter::once("memorija").chain(args.iter().copied()))
            .map(|cli| cli.command.unwrap_or(Command::Serve(ServeArgs::default())))
    }

    #[test]
    fn version_flag_reports_name_version_and_build_date() {
        let err = parse(&["--version"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);
        let text = err.to_string();
        assert!(
            text.starts_with("memorija "),
            "starts with the tool name: {text}"
        );
        assert!(
            text.contains(env!("CARGO_PKG_VERSION")),
            "names the version: {text}"
        );
        let date = built_date(&text).expect("build date: {text}");
        assert_eq!(date.len(), 10, "YYYY-MM-DD form");
        let bytes = date.as_bytes();
        assert!(
            bytes[..4].iter().all(u8::is_ascii_digit)
                && bytes[4] == b'-'
                && bytes[5..7].iter().all(u8::is_ascii_digit)
                && bytes[7] == b'-'
                && bytes[8..10].iter().all(u8::is_ascii_digit),
            "YYYY-MM-DD form: {date}"
        );
    }

    /// The `YYYY-MM-DD` build date printed after `(built `, if present.
    fn built_date(text: &str) -> Option<&str> {
        let start = text.find("(built ")? + "(built ".len();
        Some(&text[start..start + 10])
    }

    #[test]
    fn help_prints_version_and_build_date() {
        let err = parse(&["help"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        let text = err.to_string();
        assert!(
            text.contains("Persistent memory files"),
            "states the tool purpose: {text}"
        );
        assert!(
            text.contains(env!("CARGO_PKG_VERSION")),
            "names the version: {text}"
        );
        assert!(built_date(&text).is_some(), "contains a build date: {text}");
        assert!(text.contains("serve"), "lists serve: {text}");
    }

    #[test]
    fn bare_invocation_defaults_to_serve_stdio() {
        let args = parse(&[]).unwrap();
        assert_eq!(args.transport(), Transport::Stdio);
    }

    #[test]
    fn serve_without_flags_defaults_to_stdio() {
        let args = parse(&["serve"]).unwrap();
        assert_eq!(args.transport(), Transport::Stdio);
    }

    #[test]
    fn stdio_flag_maps_to_stdio() {
        let args = parse(&["serve", "--stdio"]).unwrap();
        assert_eq!(args.transport(), Transport::Stdio);
    }

    #[test]
    fn http_maps_to_default_port() {
        let args = parse(&["serve", "--http"]).unwrap();
        assert_eq!(
            args.transport(),
            Transport::Http {
                host: DEFAULT_HTTP_HOST,
                port: 8080
            }
        );
        assert_eq!(
            DEFAULT_HTTP_HOST,
            std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)
        );
    }

    #[test]
    fn http_with_port_maps_to_that_port() {
        let args = parse(&["serve", "--http", "--port", "9000"]).unwrap();
        assert_eq!(
            args.transport(),
            Transport::Http {
                host: DEFAULT_HTTP_HOST,
                port: 9000
            }
        );
    }

    #[test]
    fn http_with_host_maps_to_that_host() {
        let args = parse(&["serve", "--http", "--host", "0.0.0.0"]).unwrap();
        assert_eq!(
            args.transport(),
            Transport::Http {
                host: std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED),
                port: 8080
            }
        );
    }

    #[test]
    fn http_with_host_and_port_maps_to_both() {
        let args = parse(&["serve", "--http", "--host", "::1", "--port", "9000"]).unwrap();
        assert_eq!(
            args.transport(),
            Transport::Http {
                host: std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST),
                port: 9000
            }
        );
    }

    #[test]
    fn host_without_http_is_rejected() {
        let err = parse(&["serve", "--host", "0.0.0.0"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
        assert!(err.to_string().contains("--http"), "mentions --http: {err}");
    }

    #[test]
    fn host_with_stdio_is_rejected() {
        let err = parse(&["serve", "--stdio", "--host", "0.0.0.0"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
        assert!(err.to_string().contains("--http"), "mentions --http: {err}");
    }

    #[test]
    fn invalid_host_is_rejected() {
        let err = parse(&["serve", "--http", "--host", "not-an-address"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
        assert!(
            err.to_string().contains("not-an-address"),
            "names the value: {err}"
        );
    }

    #[test]
    fn stdio_and_http_conflict() {
        let args = parse(&["serve", "--stdio", "--http"]).unwrap();
        let err = args.validate().unwrap_err();
        assert!(err.contains("--stdio"), "names --stdio: {err}");
        assert!(err.contains("--http"), "names --http: {err}");
    }

    #[test]
    fn port_without_http_is_rejected() {
        let err = parse(&["serve", "--port", "9000"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
        assert!(err.to_string().contains("--http"), "mentions --http: {err}");
    }

    #[test]
    fn port_with_stdio_is_rejected() {
        let err = parse(&["serve", "--stdio", "--port", "9000"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
        assert!(err.to_string().contains("--http"), "mentions --http: {err}");
    }

    #[test]
    fn out_of_range_port_is_rejected() {
        let err = parse(&["serve", "--http", "--port", "70000"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
        assert!(err.to_string().contains("70000"), "names the port: {err}");
    }

    #[test]
    fn help_subcommand_requests_help() {
        let err = parse(&["help"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        assert!(err.to_string().contains("serve"), "lists serve: {err}");
    }

    #[test]
    fn help_serve_prints_serve_usage() {
        let err = parse(&["help", "serve"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        let text = err.to_string();
        for flag in ["--stdio", "--http", "--host", "--port"] {
            assert!(text.contains(flag), "mentions {flag}: {text}");
        }
    }

    #[test]
    fn help_unknown_topic_is_rejected() {
        let err = parse(&["help", "flush"]).unwrap_err();
        assert_ne!(err.kind(), ErrorKind::DisplayHelp);
        assert!(err.to_string().contains("flush"), "names flush: {err}");
    }

    #[test]
    fn unknown_subcommand_is_rejected() {
        let err = parse(&["flush"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidSubcommand);
        assert!(err.to_string().contains("flush"), "names flush: {err}");
    }

    #[test]
    fn web_parses_with_defaults() {
        match parse_command(&["web"]).unwrap() {
            Command::Web(args) => {
                assert_eq!(args.port, DEFAULT_WEB_PORT, "default port 8081");
                assert_eq!(args.host, DEFAULT_HTTP_HOST, "default host 127.0.0.1");
                assert!(!args.no_open);
            }
            other => panic!("expected web: {other:?}"),
        }
    }

    #[test]
    fn web_with_port_maps_to_that_port() {
        match parse_command(&["web", "--port", "9090"]).unwrap() {
            Command::Web(args) => assert_eq!(args.port, 9090),
            other => panic!("expected web: {other:?}"),
        }
    }

    #[test]
    fn web_with_host_maps_to_that_host() {
        match parse_command(&["web", "--host", "0.0.0.0"]).unwrap() {
            Command::Web(args) => {
                assert_eq!(
                    args.host,
                    std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)
                )
            }
            other => panic!("expected web: {other:?}"),
        }
    }

    #[test]
    fn web_with_host_and_port_maps_to_both() {
        match parse_command(&["web", "--host", "::1", "--port", "9090"]).unwrap() {
            Command::Web(args) => {
                assert_eq!(
                    args.host,
                    std::net::IpAddr::V6(std::net::Ipv6Addr::LOCALHOST)
                );
                assert_eq!(args.port, 9090);
            }
            other => panic!("expected web: {other:?}"),
        }
    }

    #[test]
    fn web_no_open_flag_is_set() {
        match parse_command(&["web", "--no-open"]).unwrap() {
            Command::Web(args) => assert!(args.no_open),
            other => panic!("expected web: {other:?}"),
        }
    }

    #[test]
    fn web_out_of_range_port_is_rejected() {
        let err = parse_command(&["web", "--port", "70000"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
        assert!(err.to_string().contains("70000"), "names the port: {err}");
    }

    #[test]
    fn web_invalid_host_is_rejected() {
        let err = parse_command(&["web", "--host", "not-an-address"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
        assert!(
            err.to_string().contains("not-an-address"),
            "names the value: {err}"
        );
    }

    #[test]
    fn help_web_prints_web_usage() {
        let err = parse_command(&["help", "web"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayHelp);
        let text = err.to_string();
        for flag in ["--host", "--port", "--no-open"] {
            assert!(text.contains(flag), "mentions {flag}: {text}");
        }
    }

    #[test]
    fn list_parses_as_a_subcommand() {
        assert!(matches!(parse_command(&["list"]), Ok(Command::List)));
    }

    #[test]
    fn help_lists_the_inspection_commands() {
        let err = parse_command(&["--help"]).unwrap_err();
        let text = err.to_string();
        assert!(text.contains("list"), "lists list: {text}");
    }

    #[test]
    fn bare_invocation_still_resolves_to_serve() {
        assert!(matches!(
            parse_command(&[]),
            Ok(Command::Serve(ServeArgs { .. }))
        ));
    }

    #[test]
    fn show_parses_file_version_and_flags() {
        match parse_command(&["show", "notes.md"]).unwrap() {
            Command::Show(args) => {
                assert_eq!(args.file, "notes.md");
                assert_eq!(args.version, None);
                assert!(!args.raw && !args.render);
            }
            other => panic!("expected show: {other:?}"),
        }

        match parse_command(&["show", "notes.md", "2", "--raw"]).unwrap() {
            Command::Show(args) => {
                assert_eq!(args.version, Some(2));
                assert!(args.raw);
            }
            other => panic!("expected show: {other:?}"),
        }

        match parse_command(&["show", "notes.md", "--render"]).unwrap() {
            Command::Show(args) => assert!(args.render),
            other => panic!("expected show: {other:?}"),
        }
    }

    #[test]
    fn show_accepts_raw_and_render_together_raw_wins() {
        // No clap conflict is declared (design D2): both flags parse, and the
        // raw-wins decision is made by `inspect::force_raw`.
        match parse_command(&["show", "notes.md", "--raw", "--render"]).unwrap() {
            Command::Show(args) => assert!(args.raw && args.render),
            other => panic!("expected show: {other:?}"),
        }
    }

    #[test]
    fn versions_parses_the_file_argument() {
        match parse_command(&["versions", "notes.md"]).unwrap() {
            Command::Versions { file } => assert_eq!(file, "notes.md"),
            other => panic!("expected versions: {other:?}"),
        }
        let err = parse_command(&["versions"]).unwrap_err();
        assert_eq!(
            err.kind(),
            ErrorKind::MissingRequiredArgument,
            "a file is required: {err}"
        );
    }

    #[test]
    fn changes_parses_all_timespan_forms() {
        match parse_command(&["changes"]).unwrap() {
            Command::Changes { timespan } => assert_eq!(timespan, None),
            other => panic!("expected changes: {other:?}"),
        }
        for (spec, expected) in [
            ("24h", crate::inspect::Timespan::Window(24 * 3_600)),
            (
                "2026-11-01",
                crate::inspect::parse_timespan("2026-11-01").unwrap(),
            ),
            ("all", crate::inspect::Timespan::All),
        ] {
            match parse_command(&["changes", spec]).unwrap() {
                Command::Changes {
                    timespan: Some(parsed),
                } => assert_eq!(parsed, expected, "{spec}"),
                other => panic!("expected changes: {other:?}"),
            }
        }
    }

    #[test]
    fn unparseable_timespan_is_a_usage_error() {
        let err = parse_command(&["changes", "not-a-timespan"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
        assert!(
            err.to_string().contains("not-a-timespan"),
            "names the value: {err}"
        );
        let err = parse_command(&["changes", "2026-10-31..2026-10-01"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ValueValidation);
        assert!(
            err.to_string().contains("after its end"),
            "names the problem: {err}"
        );
    }
}
