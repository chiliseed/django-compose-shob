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
    /// Show services status
    Status {},
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
        CliCommand::Start { build } => {
            docker_compose::start(build);
        }

        CliCommand::Migrate {
            application,
            migration_number,
        } => {
            django::migrate(opts.service.as_str(), application, migration_number);
        }

        CliCommand::Restart { all } => {
            docker_compose::restart(all, opts.service.as_str());
        }

        CliCommand::Stop {} => {
            docker_compose::stop(None);
        }

        CliCommand::PurgeDb { db_folder } => {
            django::purge_db(db_folder);
        }

        CliCommand::Rebuild {} => {
            docker_compose::rebuild(opts.service.as_str());
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
    }
}
