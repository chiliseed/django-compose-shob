use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use structopt::StructOpt;
use std::path::Path;
use std::env;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "ddc-c2",
    about = "Django + docker-compose cli tools to ease the life of a developer :)"
)]
struct Opt {
    /// Docker compose service to operate on
    #[structopt(default_value = "api")]
    service: String,
    /// docker compose yml
    #[structopt(default_value = "docker-compose.yml")]
    docker_compose_file: String,
    #[structopt(subcommand)]
    cmd: CliCommand,
}

#[derive(Debug, StructOpt)]
enum CliCommand {
    /// Run server
    Start {
        #[structopt(short, long)]
        build: bool,
    },
    /// Rebuild and run service container
    Rebuild {},
    /// Restart service server
    Restart {
        /// Restart all containers
        #[structopt(long)]
        all: bool,
    },
    /// Stop and remove all containers
    Stop {},
    /// Remove pg and rebuild the database
    PurgeDb {},
    /// Create migration and apply it to the db
    Migrate {
        /// Name of a specific application to migrate
        application: Option<String>,
        /// Specific migration to rollback to
        migration_number: Option<String>,
    },
    /// Print out all service urls
    ShowUrls {},
    /// Add new application to django project
    AddApp {
        /// Application name
        name: String,
    },
}

const DOCKER_COMPOSE: &str = "docker-compose";

fn exec_command(cmd: &str, args: Vec<&str>) {
    let mut cli_command = Command::new(cmd)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    for line_result in BufReader::new(cli_command.stdout.as_mut().unwrap()).lines() {
        print!("{}", line_result.unwrap());
        print!("\r\n");
    }

    for line_result in BufReader::new(cli_command.stderr.as_mut().unwrap()).lines() {
        print!("{}", line_result.unwrap());
        print!("\r\n");
    }

    cli_command.wait().unwrap();
}

/// Execute python manage.py command
fn exec_manage_command(service: &str, args: Vec<&str>) {
    let cmd_args = vec![
        "exec",
        service,
        "python",
        "manage.py",
    ];
    exec_command(DOCKER_COMPOSE, [cmd_args, args].concat());
}

fn main() {
    let opts = Opt::from_args();
    let here = env::current_dir().expect("Error getting current dir");
    let is_docker_yml_found = Path::new(&here).join(opts.docker_compose_file).exists();
    let is_docker_yaml_found = Path::new(&here).join("docker-compose.yaml").exists();
    if !is_docker_yml_found && !is_docker_yaml_found {
        eprintln!("No docker compose file found")
    }

    match opts.cmd {
        CliCommand::Start { build } => {
            if build {
                exec_command(
                    DOCKER_COMPOSE,
                    vec![
                        "build",
                        "--force-rm",
                        "--parallel",
                    ],
                );
            }
            exec_command(DOCKER_COMPOSE, vec!["up", "-d"]);
        }

        CliCommand::Migrate { application, migration_number } => {
            let mut make_migration_args = vec!["makemigrations"];
            let mut migrate_args = vec!["migrate"];

            match application {
                Some(app) => {
                    migrate_args.push(app.as_str());
                    match migration_number {
                        Some(migration) => {
                            migrate_args.push(migration.as_str());
                            exec_manage_command(opts.service.as_str(), migrate_args);
                        }

                        None => {
                            make_migration_args.push(app.as_str());
                            exec_manage_command(opts.service.as_str(), make_migration_args);
                            exec_manage_command(opts.service.as_str(), migrate_args);
                        }
                    }
                }
                None => {
                    exec_manage_command(opts.service.as_str(), make_migration_args);
                    exec_manage_command(opts.service.as_str(), migrate_args);
                }
            }
        },

        CliCommand::Restart { all } => {
            if all {
                exec_command(DOCKER_COMPOSE, vec!["restart"]);
            } else {
                exec_command(
                    DOCKER_COMPOSE,
                    vec!["restart", opts.service.as_str()],
                );
            }
        }

        CliCommand::Stop {} => {
            exec_command(
                DOCKER_COMPOSE,
                vec![
                    "rm",
                    "--stop",
                    "--force",
                    "-v",
                ],
            );
        }

        CliCommand::PurgeDb {} => {
            exec_command(
                DOCKER_COMPOSE,
                vec![
                    "rm",
                    "--stop",
                    "--force",
                ],
            );
            exec_command("rm", vec!["-rf", "pg"]);
            exec_command(DOCKER_COMPOSE, vec!["up", "-d"]);
        }

        CliCommand::Rebuild {} => {
            exec_command(
                DOCKER_COMPOSE,
                vec![
                    "rm",
                    "-s",
                    "-f",
                    "-v",
                    opts.service.as_str(),
                ],
            );
            exec_command(
                DOCKER_COMPOSE,
                vec![
                    "build",
                    "--force-rm",
                    opts.service.as_str(),
                ],
            );
            exec_command(DOCKER_COMPOSE, vec!["up", "-d"]);
        }

        CliCommand::ShowUrls {} => {
            exec_manage_command(opts.service.as_str(), vec!["show_urls"]);
        }

        CliCommand::AddApp { name } => {
            exec_manage_command(opts.service.as_str(), vec!["startapp", name.as_str()]);
        }
    }
}
