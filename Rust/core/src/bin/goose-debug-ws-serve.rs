use std::{env, path::PathBuf};

use goose_core::{
    debug_ws_server::{DebugWsServerOptions, serve_debug_ws_once},
    report::write_json_report,
    tool_args::{args, default_path, path_value, value},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(2);
    }
}

fn run() -> goose_core::GooseResult<()> {
    let args = args();
    reject_legacy_debug_ws_token_arg(&args)?;
    let database_path = default_path(&args, "--db", "goose.sqlite")?;
    let session_id = value(&args, "--session-id")?
        .ok_or_else(|| goose_core::GooseError::message("--session-id is required"))?;
    let token = debug_ws_token_from_env(env::var)?;
    let bind_host = value(&args, "--bind-host")?.unwrap_or_else(|| "127.0.0.1".to_string());
    let port = parse_u16(
        value(&args, "--port")?.as_deref().unwrap_or("49152"),
        "--port",
    )?;
    let poll_interval_ms = parse_u64(
        value(&args, "--poll-interval-ms")?
            .as_deref()
            .unwrap_or("100"),
        "--poll-interval-ms",
    )?;
    let idle_timeout_ms = parse_u64(
        value(&args, "--idle-timeout-ms")?
            .as_deref()
            .unwrap_or("30000"),
        "--idle-timeout-ms",
    )?;
    let max_events = value(&args, "--max-events")?
        .as_deref()
        .map(|value| parse_usize(value, "--max-events"))
        .transpose()?;
    let output: Option<PathBuf> = path_value(&args, "--output")?;

    let report = serve_debug_ws_once(DebugWsServerOptions {
        database_path,
        session_id,
        bind_host,
        port,
        token,
        poll_interval_ms,
        idle_timeout_ms,
        max_events,
    })?;
    write_json_report(&report, output.as_deref())?;
    if report.pass {
        Ok(())
    } else {
        std::process::exit(1);
    }
}

fn debug_ws_token(token: Option<String>) -> goose_core::GooseResult<String> {
    token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| goose_core::GooseError::message("GOOSE_DEBUG_WS_TOKEN is required"))
}

fn debug_ws_token_from_env(
    read_env: impl FnOnce(&str) -> Result<String, env::VarError>,
) -> goose_core::GooseResult<String> {
    debug_ws_token(read_env("GOOSE_DEBUG_WS_TOKEN").ok())
}

fn reject_legacy_debug_ws_token_arg(args: &[String]) -> goose_core::GooseResult<()> {
    if args
        .iter()
        .any(|arg| arg == "--token" || arg.starts_with("--token="))
    {
        return Err(goose_core::GooseError::message(
            "--token is no longer accepted; set GOOSE_DEBUG_WS_TOKEN",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{debug_ws_token, debug_ws_token_from_env, reject_legacy_debug_ws_token_arg};

    #[test]
    fn rejects_a_missing_debug_websocket_token() {
        assert!(debug_ws_token(None).is_err());
    }

    #[test]
    fn rejects_an_empty_debug_websocket_token() {
        assert!(debug_ws_token(Some(String::new())).is_err());
        assert!(debug_ws_token(Some("  ".to_string())).is_err());
    }

    #[test]
    fn accepts_a_nonempty_debug_websocket_token() {
        assert_eq!(debug_ws_token(Some("local-session".to_string())).unwrap(), "local-session");
    }

    #[test]
    fn reads_the_token_from_the_named_environment_variable() {
        let token = debug_ws_token_from_env(|name| {
            assert_eq!(name, "GOOSE_DEBUG_WS_TOKEN");
            Ok("local-session".to_string())
        })
        .unwrap();
        assert_eq!(token, "local-session");
    }

    #[test]
    fn rejects_legacy_token_argument_forms() {
        for args in [
            vec!["--token".to_string(), "local-session".to_string()],
            vec!["--token=local-session".to_string()],
        ] {
            assert!(reject_legacy_debug_ws_token_arg(&args).is_err());
        }
    }

    #[test]
    fn allows_other_cli_arguments_without_a_legacy_token() {
        let args = vec!["--session-id".to_string(), "debug-session-1".to_string()];
        assert!(reject_legacy_debug_ws_token_arg(&args).is_ok());
    }
}

fn parse_u16(value: &str, name: &str) -> goose_core::GooseResult<u16> {
    value
        .parse::<u16>()
        .map_err(|error| goose_core::GooseError::message(format!("{name} must be a u16: {error}")))
}

fn parse_u64(value: &str, name: &str) -> goose_core::GooseResult<u64> {
    value
        .parse::<u64>()
        .map_err(|error| goose_core::GooseError::message(format!("{name} must be a u64: {error}")))
}

fn parse_usize(value: &str, name: &str) -> goose_core::GooseResult<usize> {
    value.parse::<usize>().map_err(|error| {
        goose_core::GooseError::message(format!("{name} must be a usize: {error}"))
    })
}
