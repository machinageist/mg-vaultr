use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
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

fn run(cli: &Cli) -> Result<Output, Error> {
    let paths = XdgPaths::from_env()?;
    let mut registry = VaultRegistry::load(&paths.registry_file())?;
    match &cli.command {
        Command::Vault { command } => run_vault(command, &mut registry),
        Command::Note { command } => {
            let root = registry.resolve(cli.vault.as_deref())?;
            run_note(command, &Vault::open(&root)?)
        }
        Command::Index { command } => {
            let root = registry.resolve(cli.vault.as_deref())?;
            let vault = Vault::open(&root)?;
            Ok(run_index(
                command,
                &vault,
                &database_path(&paths.cache_dir().join("indexes"), &vault),
            ))
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
        Command::Interop { command } => {
            let root = registry.resolve(cli.vault.as_deref())?;
            let vault = Vault::open(&root)?;
            match command {
                InteropCommand::Export => {
                    let data = export_snapshot(&vault)?;
                    Ok(Output {
                        human: serde_json::to_string_pretty(&data)?,
                        data,
                        raw_human: true,
                    })
                }
            }
        }
    }
}

fn run_index(command: &IndexCommand, vault: &Vault, database: &std::path::Path) -> Output {
    let persistent = PersistentIndexStore::open(database).and_then(|mut store| match command {
        IndexCommand::Rebuild => store.rebuild(vault),
        IndexCommand::Status => store.metadata(),
    });
    let (action, status) = match persistent {
        Ok(metadata) => {
            let action = match command {
                IndexCommand::Rebuild => "rebuilt",
                IndexCommand::Status => "status",
            };
            (action, persistent_index_status(&metadata))
        }
        Err(error) => {
            let mut index = MarkdownIndex::new();
            index.rebuild(vault);
            let mut status = direct_index_status(&index, vault);
            add_storage_failure(&mut status, &error.to_string());
            ("direct fallback", status)
        }
    };
    Output {
        human: format_index_human(action, &status),
        data: status,
        raw_human: false,
    }
}

fn run_search(query: &str, vault: &Vault, database: &std::path::Path) -> Output {
    match persistent_search(query, vault, database) {
        Ok(Some(output)) => output,
        Ok(None) => direct_search(query, vault, None),
        Err(error) => direct_search(query, vault, Some(error.to_string())),
    }
}

fn direct_search(query: &str, vault: &Vault, storage_error: Option<String>) -> Output {
    let mut index = MarkdownIndex::new();
    index.rebuild(vault);
    let matches = index.search(query);
    let results = matches
        .iter()
        .map(|note| {
            json!({
                "path": note.path,
                "title": note.title,
                "fingerprint": note.fingerprint,
            })
        })
        .collect::<Vec<_>>();
    let mut status = direct_index_status(&index, vault);
    if let Some(error) = storage_error {
        add_storage_failure(&mut status, &error);
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
) -> Result<Option<Output>, StoreError> {
    let mut store = PersistentIndexStore::open(database)?;
    let metadata = store.rebuild(vault)?;
    if metadata.freshness != Freshness::Current {
        return Ok(None);
    }
    let snapshot = store.search_current(query)?;
    let results = snapshot
        .notes
        .iter()
        .map(|note| {
            json!({
                "path": note.path,
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
    Ok(Some(search_output(
        query,
        &display,
        &results,
        persistent_index_status(&snapshot.metadata),
    )))
}

fn add_storage_failure(status: &mut Value, error: &str) {
    status["status"] = json!("degraded");
    status["degraded"] = json!(true);
    status["storage_error"] = json!(error);
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
        .map(|(path, title)| format!("{}\t{title}", path.display()))
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
                "path": error
                    .path
                    .strip_prefix(vault.root())
                    .unwrap_or(&error.path),
                "message": error.message,
            })).collect::<Vec<_>>(),
        }),
    }
}

fn persistent_index_status(metadata: &IndexMetadata) -> Value {
    json!({
        "status": match metadata.freshness {
            Freshness::Empty => "empty",
            Freshness::Current => "current",
            Freshness::Degraded => "degraded",
        },
        "generation": metadata.generation,
        "note_count": metadata.note_count,
        "degraded": metadata.freshness == Freshness::Degraded,
        "diagnostics": metadata.diagnostics,
        "freshness": "complete_rebuild_snapshot",
        "persistence": "sqlite",
        "schema_version": metadata.schema_version,
        "parser_version": metadata.parser_version,
        "completed_at_unix_ms": metadata.completed_at_unix_ms,
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
                diagnostic["path"].as_str().unwrap_or("<unknown>"),
                diagnostic["message"].as_str().unwrap_or("<unknown error>")
            );
        }
    }
    output
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

fn print_error(cli: &Cli, error: &Error) {
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
        eprintln!("error: {error}");
    }
}

const fn error_code(error: &Error) -> &'static str {
    match error {
        Error::UnsafePath(_) => "unsafe_path",
        Error::Collision(_) => "collision",
        Error::Conflict { .. } => "conflict",
        Error::NoVaultSelected => "no_vault_selected",
        Error::UnknownVault(_) => "unknown_vault",
        Error::InvalidVaultName(_) => "invalid_vault_name",
        Error::InvalidUtf8(_) => "invalid_utf8",
        Error::InvalidEditSpan { .. } => "invalid_edit_span",
        Error::InvalidTrashEntry(_) => "invalid_trash_entry",
        Error::Io { .. } => "io",
        Error::Json(_) => "json",
    }
}
