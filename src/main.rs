use std::env;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{exit, Command, Stdio};

use structopt::StructOpt;

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
    /// Remove local db folder and rebuild the database
    PurgeDb {
        /// local db folder defined via `volumes`, defaults to `pg/`
        #[structopt(default_value = "pg")]
        db_folder: String,
    },
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
    /// Run tests in container
    PyTest {
        /// Optional path for specific tests to run
        tests_path: Option<String>,
    },
    /// Run linters in container
    Lint {
        /// Specific lint job to execute in container. If not specified, runs all lint jobs
        /// All paths must be relative to container structure.
        /// I.e, if you `COPY`ied your local source code to `/app`, the path should be `/app/mypackage/mymodule.py`
        #[structopt(subcommand)]
        cmd: Option<LintCommands>,
        /// Specific file or folder to format/analyze
        #[structopt(default_value = "/app")]
        path: String,
    },
}

#[derive(Debug, StructOpt)]
enum LintCommands {
    /// Run black formatter
    Black {},
    /// Run prospector checks
    Prospector {},
    /// Run flake8 checks with minimal python version of 3.7.0
    Flake8 {},
    /// Run pydocstyle checks, skipping migrations folders
    Pydocstyle {
        /// Specific convention to check. Defaults to `numpy`
        #[structopt(default_value = "numpy")]
        convention: String,
    },
    /// Run mypy checks
    Mypy {
        /// Strictness level, defaults to strict
        #[structopt(default_value = "strict")]
        level: String,
    },
}

const DOCKER_COMPOSE: &str = "docker-compose";

fn exec_command(cmd: &str, args: Vec<&str>) -> bool {
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

    cli_command.wait().unwrap().success()
}

/// Execute python manage.py command
fn exec_manage_command(service: &str, args: Vec<&str>) -> bool {
    let cmd_args = vec!["exec", service, "python", "manage.py"];
    exec_command(DOCKER_COMPOSE, [cmd_args, args].concat())
}

fn main() {
    let opts = Opt::from_args();
    let here = env::current_dir().expect("Error getting current dir");
    let is_docker_yml_found = Path::new(&here).join(opts.docker_compose_file).exists();
    let is_docker_yaml_found = Path::new(&here).join("docker-compose.yaml").exists();
    if !is_docker_yml_found && !is_docker_yaml_found {
        eprintln!("No docker compose file found. There might be errors executing commands");
    }

    match opts.cmd {
        CliCommand::Start { build } => {
            if build {
                exec_command(DOCKER_COMPOSE, vec!["build", "--force-rm", "--parallel"]);
            }
            exec_command(DOCKER_COMPOSE, vec!["up", "-d"]);
        }

        CliCommand::Migrate {
            application,
            migration_number,
        } => {
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
        }

        CliCommand::Restart { all } => {
            if all {
                exec_command(DOCKER_COMPOSE, vec!["restart"]);
            } else {
                exec_command(DOCKER_COMPOSE, vec!["restart", opts.service.as_str()]);
            }
        }

        CliCommand::Stop {} => {
            exec_command(DOCKER_COMPOSE, vec!["rm", "--stop", "--force", "-v"]);
        }

        CliCommand::PurgeDb { db_folder } => {
            exec_command(DOCKER_COMPOSE, vec!["rm", "--stop", "--force"]);
            exec_command("rm", vec!["-rf", db_folder.as_str()]);
            exec_command(DOCKER_COMPOSE, vec!["up", "-d"]);
        }

        CliCommand::Rebuild {} => {
            exec_command(
                DOCKER_COMPOSE,
                vec!["rm", "-s", "-f", "-v", opts.service.as_str()],
            );
            exec_command(
                DOCKER_COMPOSE,
                vec!["build", "--force-rm", opts.service.as_str()],
            );
            exec_command(DOCKER_COMPOSE, vec!["up", "-d"]);
        }

        CliCommand::ShowUrls {} => {
            exec_manage_command(opts.service.as_str(), vec!["show_urls"]);
        }

        CliCommand::AddApp { name } => {
            exec_manage_command(opts.service.as_str(), vec!["startapp", name.as_str()]);
        }

        CliCommand::PyTest { tests_path } => {
            let mut pytest_cmd = vec!["pytest"];
            match tests_path {
                Some(tests) => {
                    pytest_cmd.push(tests.as_str());
                    exec_command(DOCKER_COMPOSE, pytest_cmd);
                }

                None => {
                    exec_command(DOCKER_COMPOSE, pytest_cmd);
                }
            }
        }

        CliCommand::Lint { cmd, path } => match cmd {
            Some(lint_job) => match lint_job {
                LintCommands::Black {} => {
                    exec_command(
                        DOCKER_COMPOSE,
                        vec!["exec", opts.service.as_str(), "black", path.as_str()],
                    );
                }

                LintCommands::Flake8 {} => {
                    exec_command(
                        DOCKER_COMPOSE,
                        vec![
                            "exec",
                            opts.service.as_str(),
                            "flake8",
                            path.as_str(),
                            "--exclude=migrations",
                        ],
                    );
                }

                LintCommands::Prospector {} => {
                    exec_command(
                        DOCKER_COMPOSE,
                        vec!["exec", opts.service.as_str(), "prospector", path.as_str()],
                    );
                }

                LintCommands::Pydocstyle { convention } => {
                    exec_command(
                        DOCKER_COMPOSE,
                        vec![
                            "exec",
                            opts.service.as_str(),
                            "pydocstyle",
                            "--convention",
                            convention.as_str(),
                            path.as_str(),
                            "--match-dir=^(?!migrations).*",
                        ],
                    );
                }

                LintCommands::Mypy { level } => {
                    exec_command(
                        DOCKER_COMPOSE,
                        vec![
                            "exec",
                            opts.service.as_str(),
                            "mypy",
                            path.as_str(),
                            format!("--{}", level).as_str(),
                        ],
                    );
                }
            },

            None => {
                if !exec_command(
                    DOCKER_COMPOSE,
                    vec!["exec", opts.service.as_str(), "black", path.as_str()],
                ) {
                    exit(1)
                }
                if !exec_command(
                    DOCKER_COMPOSE,
                    vec![
                        "exec",
                        opts.service.as_str(),
                        "flake8",
                        path.as_str(),
                        "--exclude=migrations",
                    ],
                ) {
                    exit(1)
                }
                exec_command(
                    DOCKER_COMPOSE,
                    vec!["exec", opts.service.as_str(), "prospector", path.as_str()],
                );
            }
        },
    }
}
