use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use mg_vault_core::{Error, SourceFingerprint, Vault, VaultRegistry, XdgPaths};
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
    }
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
        Error::InvalidTrashEntry(_) => "invalid_trash_entry",
        Error::Io { .. } => "io",
        Error::Json(_) => "json",
    }
}
