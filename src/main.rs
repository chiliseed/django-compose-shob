pub mod deploy;
pub mod django;
pub mod docker_compose;
pub mod utils;

use std::env;
use std::path::Path;

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
        /// Local db folder defined via `volumes`, defaults to `pg/`
        #[structopt(default_value = "pg")]
        db_folder: String,
        /// Docker volume name to which you mapped your db container
        #[structopt(short, long)]
        volume: Option<String>,
    },
    /// Create migration and apply it to the db
    Migrate {
        /// Name of a specific application to migrate
        application: Option<String>,
        /// Specific migration to rollback to
        migration_number: Option<String>,
        /// Create empty migration file for data migration
        #[structopt(long)]
        empty: bool,
        /// Provide specific migration name. If none provided, django will generate the name for you.
        #[structopt(short = "n", long = "name")]
        migration_name: Option<String>,
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
    /// Show services status
    Status {},
    /// Purge docker cache & storage
    PurgeDocker {},
    /// Gzips provided directory, uploads to remote server, builds docker images
    /// and stars docker compose with `-d`
    /// Only login with ssh key is supported at the moment
    Deploy {
        /// Remote server IP
        server_ip: String,
        /// Server user to login to
        #[structopt(default_value = "ubuntu")]
        server_user: String,
        /// Path to directory which contains docker-compose.yml in its root
        /// and has the code that you want to deploy
        deploy_dir: String,
        /// Path to ssh key to connect to remote server.
        /// If not provided, will authenticated via ssh-agent
        ssh_key: Option<String>,
        #[structopt(long)]
        excludes: Option<Vec<String>>,
    },
    /// Show logs for container
    Logs {
        /// Number of lines to show
        #[structopt(short = "n", default_value = "10")]
        lines: i32,
        /// Enable live streaming of logs
        #[structopt(short)]
        follow: bool,
    },
    /// Launch python shell via django-extensions shell_plus command
    ShellPlus {},
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

fn main() {
    let opts = Opt::from_args();
    let here = env::current_dir().expect("Error getting current dir");
    let is_docker_yml_found = Path::new(&here).join(opts.docker_compose_file).exists();
    let is_docker_yaml_found = Path::new(&here).join("docker-compose.yaml").exists();
    if !is_docker_yml_found && !is_docker_yaml_found {
        eprintln!("No docker compose file found. There might be errors executing commands");
    }

    match opts.cmd {
        CliCommand::PurgeDocker {} => {
            utils::exec_command("docker", vec!["system", "prune"]);
        }

        CliCommand::Start { build } => {
            docker_compose::start(build);
            docker_compose::logs(&opts.service, 10, false);
        }

        CliCommand::Migrate {
            application,
            migration_number,
            empty,
            migration_name,
        } => {
            django::migrate(
                opts.service.as_str(),
                application,
                migration_number,
                empty,
                migration_name,
            );
        }

        CliCommand::Restart { all } => {
            docker_compose::restart(all, opts.service.as_str());
            docker_compose::logs(&opts.service, 10, false);
        }

        CliCommand::Stop {} => {
            docker_compose::stop(None);
        }

        CliCommand::PurgeDb { db_folder, volume } => {
            django::purge_db(db_folder, volume);
        }

        CliCommand::Rebuild {} => {
            docker_compose::rebuild(opts.service.as_str());
            docker_compose::logs(&opts.service, 10, false);
        }

        CliCommand::ShowUrls {} => {
            django::show_urls(opts.service.as_str());
        }

        CliCommand::AddApp { name } => {
            django::add_app(name.as_str(), opts.service.as_str());
        }

        CliCommand::PyTest { tests_path } => {
            django::pytest(tests_path, opts.service.as_str());
        }

        CliCommand::Lint { cmd, path } => match cmd {
            Some(lint_job) => match lint_job {
                LintCommands::Black {} => {
                    django::black(path.as_str(), opts.service.as_str());
                }

                LintCommands::Flake8 {} => {
                    django::flake8(path.as_str(), opts.service.as_str());
                }

                LintCommands::Prospector {} => {
                    django::prospector(path.as_str(), opts.service.as_str());
                }

                LintCommands::Pydocstyle { convention } => {
                    django::pydocstyle(path.as_str(), opts.service.as_str(), convention.as_str());
                }

                LintCommands::Mypy { level } => {
                    django::mypy(path.as_str(), opts.service.as_str(), level.as_str());
                }
            },

            None => {
                django::lint(path.as_str(), opts.service.as_str());
            }
        },

        CliCommand::Status {} => {
            docker_compose::status();
        }

        CliCommand::Deploy {
            server_ip,
            server_user,
            deploy_dir,
            ssh_key,
            excludes,
        } => {
            let mut excluded_patterns: Vec<String> = Vec::new();
            if let Some(e) = excludes {
                excluded_patterns.extend(e.iter().cloned());
            }
            deploy::execute(
                server_ip.as_str(),
                server_user.as_str(),
                ssh_key,
                deploy_dir.as_str(),
                Some(excluded_patterns),
            );
        }

        CliCommand::Logs { lines, follow } => {
            docker_compose::logs(&opts.service, lines, follow);
        }

        CliCommand::ShellPlus {} => {
            django::shell_plus(&opts.service);
        }
    }
}
