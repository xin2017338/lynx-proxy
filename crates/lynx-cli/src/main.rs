use anyhow::Result;
use clap::Parser;
use lynx_cli::cert_cmd::{self, CertOptions};
use lynx_cli::daemon::DaemonManager;
use lynx_cli::rules_cmd::{RulesOptions, run_apply, run_pull, run_push, run_schema_export};
use lynx_cli::version_check;
use lynx_cli::{
    Args, CertCommands, Commands, LogConfig, ProxyServerApp, RulesCommands, RulesSchemaCommands,
    ServerArgs, resolve_data_dir,
};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Soft reminder only — no interactive update prompt
    if let Some(ref v) = version_check::check_for_updates().await {
        version_check::print_update_banner(v);
    }

    match args.command {
        Commands::Start {
            server_args:
                ServerArgs {
                    port,
                    data_dir,
                    log_level,
                    local_only,
                    user,
                    pass,
                },
        } => {
            let resolved_data_dir = resolve_data_dir(data_dir)?;
            let manager = DaemonManager::new(None)?;
            manager
                .start_daemon(
                    port,
                    Some(resolved_data_dir.to_string_lossy().to_string()),
                    log_level,
                    local_only,
                    user,
                    pass,
                )
                .await?;
        }
        Commands::Stop => {
            let manager = DaemonManager::new(None)?;
            manager.stop_daemon()?;
        }
        Commands::Restart {
            port,
            data_dir,
            log_level,
            local_only,
            user,
            pass,
        } => {
            let manager = DaemonManager::new(None)?;
            manager
                .restart_daemon(port, data_dir, log_level, local_only, user, pass)
                .await?;
        }
        Commands::Status => {
            let manager = DaemonManager::new(None)?;
            manager.show_status()?;
        }
        Commands::Run {
            server_args:
                ServerArgs {
                    port,
                    data_dir,
                    log_level,
                    local_only,
                    user,
                    pass,
                },
            daemon,
        } => {
            let resolved_data_dir = resolve_data_dir(data_dir)?;

            let mut log_config = LogConfig::new(log_level);
            if daemon {
                log_config = log_config.with_file(LogConfig::lynx_log_file(&resolved_data_dir))
            } else {
                log_config = log_config.with_console(true);
            }
            log_config.init_logging()?;

            let app = ProxyServerApp::new(
                port,
                Some(resolved_data_dir.to_string_lossy().to_string()),
                daemon,
                local_only,
                user,
                pass,
            );
            app.start_server().await?;

            println!("Proxy server is running...");
            if !daemon {
                signal::ctrl_c().await?;
                println!("\nReceived Ctrl+C, shutting down...");
            }
        }
        Commands::Rules { command } => match command {
            RulesCommands::Push { args } => {
                run_push(RulesOptions {
                    file: args.file,
                    data_dir: args.data_dir,
                    project: args.project,
                })
                .await?;
            }
            RulesCommands::Pull { args } => {
                run_pull(RulesOptions {
                    file: args.file,
                    data_dir: args.data_dir,
                    project: args.project,
                })
                .await?;
            }
            RulesCommands::Apply { args } => {
                run_apply(RulesOptions {
                    file: args.file,
                    data_dir: args.data_dir,
                    project: args.project,
                })
                .await?;
            }
            RulesCommands::Schema { command } => match command {
                RulesSchemaCommands::Export { out } => {
                    run_schema_export(out).await?;
                }
            },
        },
        Commands::Cert { command } => match command {
            CertCommands::Install { args } => {
                cert_cmd::run_install(CertOptions {
                    data_dir: args.data_dir,
                })?;
            }
            CertCommands::Uninstall { args } => {
                cert_cmd::run_uninstall(CertOptions {
                    data_dir: args.data_dir,
                })?;
            }
            CertCommands::Status { args } => {
                cert_cmd::run_status(CertOptions {
                    data_dir: args.data_dir,
                })?;
            }
        },
    }

    Ok(())
}
