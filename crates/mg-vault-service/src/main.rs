use std::process::ExitCode;

use mg_vault_core::XdgPaths;
use mg_vault_service::Server;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("mg-vault-indexd: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let paths = XdgPaths::from_env()?;
    let server = Server::bind(&paths.service_socket(), &paths.registry_file())?;
    server.serve()?;
    Ok(())
}
