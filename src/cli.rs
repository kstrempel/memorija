//! Command-line interface of the binary (spec: `cli`).

use std::net::{IpAddr, Ipv4Addr};

use clap::error::ErrorKind;
use clap::{Args, CommandFactory, Parser, Subcommand};

/// Default port for the HTTP transport.
pub const DEFAULT_HTTP_PORT: u16 = 8080;

/// Default bind address for the HTTP transport.
pub const DEFAULT_HTTP_HOST: IpAddr = IpAddr::V4(Ipv4Addr::LOCALHOST);

/// Version line reported by `--version`: the crate version plus the UTC
/// build date captured by `build.rs` (design D2).
pub const VERSION_LINE: &str =
    concat!(env!("CARGO_PKG_VERSION"), " (built ", env!("BUILD_DATE"), ")");

/// Version line with the tool name, shown under the `help` usage.
pub const AFTER_HELP: &str = concat!("memorija ", env!("CARGO_PKG_VERSION"), " (built ", env!("BUILD_DATE"), ")");

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

impl Cli {
    /// The serve arguments, applying the bare-invocation default: no
    /// subcommand behaves as `serve --stdio`.
    pub fn serve_args(&self) -> ServeArgs {
        self.command
            .as_ref()
            .map(|command| match command {
                Command::Serve(args) => args.clone(),
            })
            .unwrap_or_default()
    }
}

#[derive(Debug, Clone, Subcommand)]
pub enum Command {
    /// Start the MCP server
    Serve(ServeArgs),
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

/// Parse the process arguments into the serve arguments, exiting with a clap
/// usage error on invalid input (including the `--stdio`/`--http` conflict).
pub fn parse() -> ServeArgs {
    let args = Cli::parse().serve_args();
    if let Err(message) = args.validate() {
        Cli::command()
            .error(ErrorKind::ArgumentConflict, message)
            .exit();
    }
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn parse(args: &[&str]) -> Result<ServeArgs, clap::Error> {
        Cli::try_parse_from(std::iter::once("memorija").chain(args.iter().copied()))
            .map(|cli| cli.serve_args())
    }

    #[test]
    fn version_flag_reports_name_version_and_build_date() {
        let err = parse(&["--version"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::DisplayVersion);
        let text = err.to_string();
        assert!(text.starts_with("memorija "), "starts with the tool name: {text}");
        assert!(text.contains(env!("CARGO_PKG_VERSION")), "names the version: {text}");
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
        assert!(text.contains(env!("CARGO_PKG_VERSION")), "names the version: {text}");
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
        assert_eq!(DEFAULT_HTTP_HOST, std::net::IpAddr::V4(std::net::Ipv4Addr::LOCALHOST));
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
        assert!(err.to_string().contains("not-an-address"), "names the value: {err}");
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
}
