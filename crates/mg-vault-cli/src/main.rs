use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use mg_vault_client::{Client, ClientError, PROTOCOL_VERSION};
use mg_vault_core::{
    Error, IndexStatus, MarkdownIndex, SourceFingerprint, Vault, VaultRegistry, XdgPaths,
    export_snapshot,
};
use mg_vault_index::{Freshness, IndexMetadata, PersistentIndexStore, StoreError, database_path};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Debug, Parser)]
#[command(
    name = "mg-vault",
    version,
    about = "Local-first Markdown vault authority"
)]
struct Cli {
    /// Emit a stable versioned JSON envelope.
    #[arg(long, global = true)]
    json: bool,
    /// Never prompt for missing input (all current commands are non-interactive).
    #[arg(long, global = true)]
    no_input: bool,
    /// Disable ANSI color. `NO_COLOR` is also honored.
    #[arg(long, global = true)]
    no_color: bool,
    /// Use a registered vault instead of the selected vault.
    #[arg(long, global = true)]
    vault: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
    Note {
        #[command(subcommand)]
        command: NoteCommand,
    },
    Index {
        #[command(subcommand)]
        command: IndexCommand,
    },
    Search {
        query: String,
    },
    Service {
        #[command(subcommand)]
        command: ServiceCommand,
    },
    Interop {
        #[command(subcommand)]
        command: InteropCommand,
    },
}

#[derive(Debug, Subcommand)]
enum InteropCommand {
    Export,
}

#[derive(Debug, Subcommand)]
enum IndexCommand {
    Rebuild,
    Status,
}

#[derive(Debug, Subcommand)]
enum ServiceCommand {
    Status,
}

#[derive(Debug, Subcommand)]
enum VaultCommand {
    Register { name: String, path: PathBuf },
    List,
    Select { name: String },
}

#[derive(Debug, Subcommand)]
enum NoteCommand {
    Create {
        path: PathBuf,
        #[arg(long)]
        body: String,
    },
    Read {
        path: PathBuf,
    },
    Write {
        path: PathBuf,
        #[arg(long)]
        body: String,
        #[arg(long)]
        expected: SourceFingerprint,
    },
    Trash {
        path: PathBuf,
    },
    Restore {
        id: String,
    },
}

#[derive(Serialize)]
struct Envelope<T> {
    version: u8,
    ok: bool,
    data: T,
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error(transparent)]
    Core(#[from] Error),
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Client(#[from] ClientError),
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(output) => {
            print_success(&cli, output);
            ExitCode::SUCCESS
        }
        Err(error) => {
            print_error(&cli, &error);
            ExitCode::from(1)
        }
    }
}

struct Output {
    human: String,
    data: Value,
    raw_human: bool,
}

fn run(cli: &Cli) -> Result<Output, CliError> {
    let paths = XdgPaths::from_env()?;
    let mut registry = VaultRegistry::load(&paths.registry_file())?;
    match &cli.command {
        Command::Vault { command } => Ok(run_vault(command, &mut registry)?),
        Command::Note { command } => {
            let root = registry.resolve(cli.vault.as_deref())?;
            Ok(run_note(command, &Vault::open(&root)?)?)
        }
        Command::Index { command } => {
            let root = registry.resolve(cli.vault.as_deref())?;
            let vault = Vault::open(&root)?;
            run_index(
                command,
                &vault,
                &database_path(&paths.cache_dir().join("indexes"), &vault),
            )
        }
        Command::Search { query } => {
            let root = registry.resolve(cli.vault.as_deref())?;
            let vault = Vault::open(&root)?;
            Ok(run_search(
                query,
                &vault,
                &database_path(&paths.cache_dir().join("indexes"), &vault),
            ))
        }
        Command::Service { command } => match command {
            ServiceCommand::Status => run_service_status(&paths),
        },
        Command::Interop { command } => {
            let root = registry.resolve(cli.vault.as_deref())?;
            let vault = Vault::open(&root)?;
            match command {
                InteropCommand::Export => {
                    let data = export_snapshot(&vault)?;
                    Ok(Output {
                        human: serde_json::to_string_pretty(&data).map_err(Error::from)?,
                        data,
                        raw_human: true,
                    })
                }
            }
        }
    }
}

fn run_service_status(paths: &XdgPaths) -> Result<Output, CliError> {
    let process = std::process::id();
    let mut client = Client::connect(
        &paths.service_socket(),
        &[PROTOCOL_VERSION],
        format!("cli-{process}-hello"),
    )?;
    let status = client.service_status(format!("cli-{process}-status"))?;
    let data = json!({
        "service": status.state,
        "service_version": client.service_version(),
        "protocol_version": client.protocol_version(),
        "registered_vaults": status.registered_vaults,
        "transport": "unix",
    });
    Ok(Output {
        human: format!(
            "service: {} version={} protocol={} registered_vaults={}",
            status.state,
            client.service_version(),
            client.protocol_version(),
            status.registered_vaults
        ),
        data,
        raw_human: false,
    })
}

fn run_index(
    command: &IndexCommand,
    vault: &Vault,
    database: &std::path::Path,
) -> Result<Output, CliError> {
    let persistent = match command {
        IndexCommand::Rebuild => {
            match PersistentIndexStore::open(database).and_then(|mut store| store.rebuild(vault)) {
                Ok(metadata) => Ok(metadata),
                Err(_) => PersistentIndexStore::recover_and_rebuild(database, vault),
            }
        }
        IndexCommand::Status => {
            PersistentIndexStore::open(database).and_then(|mut store| store.verify_freshness(vault))
        }
    };
    let (action, status) = match persistent {
        Ok(metadata) => {
            let action = match command {
                IndexCommand::Rebuild => "rebuilt",
                IndexCommand::Status => "status",
            };
            let freshness = match command {
                IndexCommand::Rebuild => "complete_rebuild_snapshot",
                IndexCommand::Status => "source_observed_snapshot",
            };
            (action, persistent_index_status(&metadata, freshness))
        }
        Err(error) if matches!(command, IndexCommand::Status) => {
            let mut index = MarkdownIndex::new();
            index.rebuild(vault);
            let mut status = direct_index_status(&index, vault);
            add_storage_failure(&mut status, &error.to_string());
            ("direct fallback", status)
        }
        Err(error) => return Err(error.into()),
    };
    Ok(Output {
        human: format_index_human(action, &status),
        data: status,
        raw_human: false,
    })
}

fn run_search(query: &str, vault: &Vault, database: &std::path::Path) -> Output {
    match persistent_search(query, vault, database) {
        Ok(output) => output,
        Err(error) => direct_search(query, vault, Some(&error)),
    }
}

fn direct_search(query: &str, vault: &Vault, fallback: Option<&StoreError>) -> Output {
    let mut index = MarkdownIndex::new();
    index.rebuild(vault);
    let matches = index.search(query);
    let results = matches
        .iter()
        .map(|note| {
            json!({
                "path": path_json(&note.path),
                "title": note.title,
                "fingerprint": note.fingerprint,
            })
        })
        .collect::<Vec<_>>();
    let mut status = direct_index_status(&index, vault);
    if let Some(error) = fallback {
        add_persistent_fallback(&mut status, error);
    }
    search_output(
        query,
        &matches
            .iter()
            .map(|note| (note.path.as_path(), note.title.as_str()))
            .collect::<Vec<_>>(),
        &results,
        status,
    )
}

fn persistent_search(
    query: &str,
    vault: &Vault,
    database: &std::path::Path,
) -> Result<Output, StoreError> {
    let mut store = PersistentIndexStore::open(database)?;
    let snapshot = store.search_verified(vault, query)?;
    let results = snapshot
        .notes
        .iter()
        .map(|note| {
            json!({
                "path": path_json(&note.path),
                "title": note.title,
                "fingerprint": note.fingerprint,
            })
        })
        .collect::<Vec<_>>();
    let display = snapshot
        .notes
        .iter()
        .map(|note| (note.path.as_path(), note.title.as_str()))
        .collect::<Vec<_>>();
    Ok(search_output(
        query,
        &display,
        &results,
        persistent_index_status(&snapshot.metadata, "source_observed_snapshot"),
    ))
}

fn add_persistent_fallback(status: &mut Value, error: &StoreError) {
    status["status"] = json!("degraded");
    status["degraded"] = json!(true);
    status["fallback_reason"] = json!("persistent_index_unavailable");
    if let StoreError::NotCurrent { freshness } = error {
        status["persistent_freshness"] = json!(match freshness {
            Freshness::Empty => "empty",
            Freshness::Current => "current",
            Freshness::Stale => "stale",
            Freshness::Degraded => "degraded",
        });
        status["diagnostics"]
            .as_array_mut()
            .expect("direct index diagnostics are always an array")
            .push(json!({
                "path": ".mg-vault/index.sqlite3",
                "message": "persistent index is not current",
            }));
    } else {
        add_storage_failure(status, &error.to_string());
    }
}

fn add_storage_failure(status: &mut Value, _error: &str) {
    status["status"] = json!("degraded");
    status["degraded"] = json!(true);
    status["storage_error"] = json!("persistent_index_unavailable");
    let diagnostic_message = status["storage_error"].clone();
    status["diagnostics"]
        .as_array_mut()
        .expect("direct index diagnostics are always an array")
        .push(json!({
            "path": ".mg-vault/index.sqlite3",
            "message": diagnostic_message,
        }));
}

fn search_output(
    query: &str,
    matches: &[(&std::path::Path, &str)],
    results: &[Value],
    mut data: Value,
) -> Output {
    data["query"] = json!(query);
    data["results"] = json!(results);
    let mut lines = matches
        .iter()
        .map(|(path, title)| format!("{}\t{title}", escape_path(path)))
        .collect::<Vec<_>>();
    lines.push(format_index_human("search", &data));
    if matches.is_empty() {
        lines.insert(0, format!("no matches for {query:?}"));
    }
    Output {
        human: lines.join("\n"),
        data,
        raw_human: false,
    }
}

fn direct_index_status(index: &MarkdownIndex, vault: &Vault) -> Value {
    match index.status() {
        IndexStatus::Empty => json!({
            "status": "empty", "generation": Value::Null, "note_count": 0,
            "degraded": false, "diagnostics": [],
            "freshness": "direct_rebuild_snapshot", "persistence": "none",
            "derived_from": "authoritative_vault_files",
        }),
        IndexStatus::Current { generation } => json!({
            "status": "current", "generation": generation, "note_count": index.notes().len(),
            "degraded": false, "diagnostics": [],
            "freshness": "direct_rebuild_snapshot", "persistence": "none",
            "derived_from": "authoritative_vault_files",
        }),
        IndexStatus::Degraded { generation, errors } => json!({
            "status": "degraded", "generation": generation, "note_count": index.notes().len(),
            "degraded": true,
            "freshness": "direct_rebuild_snapshot", "persistence": "none",
            "derived_from": "authoritative_vault_files",
            "diagnostics": errors.iter().map(|error| json!({
                "path": path_json(error.path.strip_prefix(vault.root()).unwrap_or(&error.path)),
                "message": error.message,
            })).collect::<Vec<_>>(),
        }),
    }
}

fn persistent_index_status(metadata: &IndexMetadata, freshness_evidence: &str) -> Value {
    json!({
        "status": match metadata.freshness {
            Freshness::Empty => "empty",
            Freshness::Current => "current",
            Freshness::Stale => "stale",
            Freshness::Degraded => "degraded",
        },
        "generation": metadata.generation,
        "note_count": metadata.note_count,
        "degraded": metadata.freshness == Freshness::Degraded,
        "diagnostics": metadata.diagnostics.iter().map(|diagnostic| json!({
            "path": path_json(&diagnostic.path),
            "message": diagnostic.message,
        })).collect::<Vec<_>>(),
        "freshness": freshness_evidence,
        "persistence": "sqlite",
        "schema_version": metadata.schema_version,
        "parser_version": metadata.parser_version,
        "completed_at_unix_ms": metadata.completed_at_unix_ms,
        "observed_at_unix_ms": metadata.observed_at_unix_ms,
        "derived_from": "authoritative_vault_files",
    })
}

fn format_index_human(action: &str, status: &Value) -> String {
    let state = status["status"].as_str().unwrap_or("unknown");
    let generation = status["generation"]
        .as_u64()
        .map_or_else(|| "-".to_owned(), |value| value.to_string());
    let count = status["note_count"].as_u64().unwrap_or(0);
    let freshness = status["freshness"].as_str().unwrap_or("unknown");
    let persistence = status["persistence"].as_str().unwrap_or("unknown");
    let mut output = format!(
        "index {action}: status={state} generation={generation} notes={count} source=authoritative_vault_files freshness={freshness} persistence={persistence}\n"
    );
    if let Some(diagnostics) = status["diagnostics"].as_array() {
        for diagnostic in diagnostics {
            let _ = writeln!(
                output,
                "degraded: {}: {}",
                json_path_human(&diagnostic["path"]),
                escape_diagnostic(diagnostic["message"].as_str().unwrap_or("<unknown error>"))
            );
        }
    }
    output
}

fn path_json(path: &Path) -> Value {
    if let Some(path) = path.to_str() {
        return json!(path);
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        let bytes_hex = path.as_os_str().as_bytes().iter().fold(
            String::with_capacity(path.as_os_str().as_bytes().len() * 2),
            |mut encoded, byte| {
                let _ = write!(encoded, "{byte:02x}");
                encoded
            },
        );
        json!({
            "encoding": "unix_bytes_hex",
            "value": bytes_hex,
            "display": escape_path(path),
        })
    }
    #[cfg(not(unix))]
    json!({
        "encoding": "platform_lossy",
        "value": path.to_string_lossy(),
        "display": escape_path(path),
    })
}

fn escape_path(path: &Path) -> String {
    if let Some(path) = path.to_str() {
        return path.chars().flat_map(char::escape_default).collect();
    }
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;

        path.as_os_str()
            .as_bytes()
            .iter()
            .map(|byte| match byte {
                b'\\' => "\\\\".to_owned(),
                0x20..=0x7e => char::from(*byte).to_string(),
                _ => format!("\\x{byte:02x}"),
            })
            .collect()
    }
    #[cfg(not(unix))]
    path.to_string_lossy()
        .chars()
        .flat_map(char::escape_default)
        .collect()
}

fn json_path_human(path: &Value) -> String {
    path.as_str().map_or_else(
        || path["display"].as_str().unwrap_or("<unknown>").to_owned(),
        |path| path.chars().flat_map(char::escape_default).collect(),
    )
}

fn escape_diagnostic(message: &str) -> String {
    message
        .chars()
        .fold(String::new(), |mut escaped, character| {
            if character.is_control() {
                escaped.extend(character.escape_default());
            } else {
                escaped.push(character);
            }
            escaped
        })
}

fn run_vault(command: &VaultCommand, registry: &mut VaultRegistry) -> Result<Output, Error> {
    match command {
        VaultCommand::Register { name, path } => {
            let record = registry.register(name, path)?;
            Ok(Output {
                human: format!("registered {} -> {}", record.name, record.path.display()),
                data: serde_json::to_value(record)?,
                raw_human: false,
            })
        }
        VaultCommand::List => {
            let records = registry.list();
            let human = if records.is_empty() {
                "no registered vaults".to_owned()
            } else {
                records
                    .iter()
                    .map(|record| {
                        let marker = if registry.selected_name() == Some(record.name.as_str()) {
                            "*"
                        } else {
                            " "
                        };
                        format!("{marker} {}\t{}", record.name, record.path.display())
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            Ok(Output {
                human,
                data: json!({
                    "selected": registry.selected_name(),
                    "vaults": records,
                }),
                raw_human: false,
            })
        }
        VaultCommand::Select { name } => {
            registry.select(name)?;
            Ok(Output {
                human: format!("selected {name}"),
                data: json!({ "selected": name }),
                raw_human: false,
            })
        }
    }
}

fn run_note(command: &NoteCommand, vault: &Vault) -> Result<Output, Error> {
    match command {
        NoteCommand::Create { path, body } => {
            let fingerprint = vault.create_note(path, body.as_bytes())?;
            Ok(Output {
                human: format!("created {} ({fingerprint})", path.display()),
                data: json!({ "path": path, "fingerprint": fingerprint }),
                raw_human: false,
            })
        }
        NoteCommand::Read { path } => {
            let note = vault.read_note(path)?;
            Ok(Output {
                human: note.content.clone(),
                data: json!({
                    "path": path,
                    "content": note.content,
                    "fingerprint": note.fingerprint,
                }),
                raw_human: true,
            })
        }
        NoteCommand::Write {
            path,
            body,
            expected,
        } => {
            let fingerprint = vault.write_note(path, body.as_bytes(), expected)?;
            Ok(Output {
                human: format!("wrote {} ({fingerprint})", path.display()),
                data: json!({ "path": path, "fingerprint": fingerprint }),
                raw_human: false,
            })
        }
        NoteCommand::Trash { path } => {
            let receipt = vault.trash_note(path)?;
            Ok(Output {
                human: format!(
                    "trashed {} ({})",
                    receipt.original_path.display(),
                    receipt.id
                ),
                data: serde_json::to_value(receipt)?,
                raw_human: false,
            })
        }
        NoteCommand::Restore { id } => {
            let receipt = vault.restore_note(id)?;
            Ok(Output {
                human: format!("restored {}", receipt.original_path.display()),
                data: serde_json::to_value(receipt)?,
                raw_human: false,
            })
        }
    }
}

fn print_success(cli: &Cli, output: Output) {
    if cli.json {
        println!(
            "{}",
            serde_json::to_string(&Envelope {
                version: 1,
                ok: true,
                data: output.data,
            })
            .expect("serializing a JSON value cannot fail")
        );
    } else if output.raw_human {
        print!("{}", output.human);
    } else {
        println!("{}", output.human);
    }
}

fn print_error(cli: &Cli, error: &CliError) {
    if cli.json {
        let envelope = json!({
            "version": 1,
            "ok": false,
            "error": {
                "code": error_code(error),
                "message": error.to_string(),
            }
        });
        eprintln!("{envelope}");
    } else {
        eprintln!("error: {}", escape_diagnostic(&error.to_string()));
    }
}

const fn error_code(error: &CliError) -> &'static str {
    match error {
        CliError::Store(_) => "index_storage",
        CliError::Client(_) => "client",
        CliError::Core(Error::UnsafePath(_)) => "unsafe_path",
        CliError::Core(Error::Collision(_)) => "collision",
        CliError::Core(Error::Conflict { .. }) => "conflict",
        CliError::Core(Error::NoVaultSelected) => "no_vault_selected",
        CliError::Core(Error::UnknownVault(_)) => "unknown_vault",
        CliError::Core(Error::InvalidVaultName(_)) => "invalid_vault_name",
        CliError::Core(Error::InvalidUtf8(_)) => "invalid_utf8",
        CliError::Core(Error::InvalidEditSpan { .. }) => "invalid_edit_span",
        CliError::Core(Error::InvalidTrashEntry(_)) => "invalid_trash_entry",
        CliError::Core(Error::Io { .. }) => "io",
        CliError::Core(Error::Json(_)) => "json",
    }
}
