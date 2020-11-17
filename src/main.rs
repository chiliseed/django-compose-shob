pub mod deploy;
pub mod django;
pub mod docker_compose;
pub mod utils;

use std::env;
use std::path::Path;

use structopt::StructOpt;

#[macro_use]
extern crate log;

#[derive(Debug, StructOpt)]
#[structopt(
    name = "ddc-shob",
    about = "Django + docker-compose cli tools to ease the life of a developer :)"
)]
struct Opt {
    /// Docker compose service to operate on
    #[structopt(default_value = "api")]
    service: String,
    /// path to docker compose yml
    #[structopt(default_value = "docker-compose.yml")]
    docker_compose_file: String,
    #[structopt(subcommand)]
    cmd: CliCommand,
}

#[derive(Debug, StructOpt)]
enum CliCommand {
    /// Purge docker cache & storage
    PurgeDocker {},
    /// Remove local db folder and rebuild the database
    PurgeDb {
        /// Local db folder defined via `volumes`, defaults to `pg/`
        #[structopt(default_value = "pg")]
        db_folder: String,
        /// Docker volume name to which you mapped your db container
        #[structopt(short, long)]
        volume: Option<String>,
    },
    /// Run server
    Start {
        /// If provided, will start/build only this service
        service_name: Option<String>,
        /// Indicate if you want to build the images before starting up
        #[structopt(short, long)]
        build: bool,
    },
    /// Build service container
    Build {
        /// Name of docker compose service to build
        service_name: Option<String>,
    },
    /// Rebuild and run service container
    Rebuild {
        /// If provided, will start/build only this service
        service_name: Option<String>,
    },
    /// Restart service server
    Restart {
        /// If provided, will start/build only this service
        service_name: Option<String>,
        /// Restart all containers
        #[structopt(long)]
        all: bool,
    },
    /// Stop and remove all containers
    Stop {
        /// If provided, will stop only this service
        service_name: Option<String>,
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
        /// Run py-test without warnings with report showing only number of failed/skipped/errored tests
        #[structopt(short, long)]
        simple: bool,
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
    /// Gzips provided directory, uploads to remote server, builds docker images
    /// and stars docker compose with `-d`
    /// Only login with ssh key is supported at the moment
    Deploy {
        /// Remote server IP
        server_ip: String,
        /// Server user to login to
        #[structopt(default_value = "ubuntu")]
        server_user: String,
        /// Path to ssh key to connect to remote server.
        /// If not provided, will authenticated via ssh-agent
        ssh_key: Option<String>,
    },
    /// Show logs for container
    Logs {
        /// Number of lines to show
        #[structopt(short = "n", default_value = "20")]
        lines: i32,
        /// Enable live streaming of logs
        #[structopt(short, long)]
        follow: bool,
        /// Output all services logs
        #[structopt(short, long)]
        all: bool,
    },
    /// Launch python shell via django-extensions shell_plus command
    ShellPlus {},
    /// Execute `python manage.py` commands inside container
    ManagePy {
        /// DIR Path to workdir directory for this command.
        #[structopt(long, short)]
        workdir: Option<String>,
        #[structopt(subcommand)]
        cmd: Option<ManagePyCommand>,
    },
    /// Execute arbitrary command inside container
    Exec {
        /// DIR Path to workdir directory for this command.
        #[structopt(long, short)]
        workdir: Option<String>,
        #[structopt(subcommand)]
        cmd: ExecCommand,
    },
}

#[derive(Debug, StructOpt)]
enum ManagePyCommand {
    /// any manage.py command, i.e. createsuperuser
    #[structopt(external_subcommand)]
    Command(Vec<String>),
}

#[derive(Debug, StructOpt)]
enum ExecCommand {
    /// Execute any command inside container
    #[structopt(external_subcommand)]
    Command(Vec<String>),
}

#[derive(Debug, StructOpt)]
enum LintCommands {
    /// Run black formatter
    Black {
        /// Optional path for specific file/module to be formatted
        custom_path: Option<String>,
    },
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
    pretty_env_logger::try_init_custom_env("DDC_SHOB_LOG")
        .expect("Cannot initialize the logger that was already initialized.");

    let opts = Opt::from_args();
    let here = env::current_dir().expect("Error getting current dir");
    let is_docker_yml_found = Path::new(&here).join(opts.docker_compose_file).exists();
    let is_docker_yaml_found = Path::new(&here).join("docker-compose.yaml").exists();
    if !is_docker_yml_found && !is_docker_yaml_found {
        eprintln!("No docker compose file found. There might be errors executing commands");
    }

    let service = |service| {
        move |name: Option<String>| {
            if let Some(s) = name {
                s
            } else {
                service
            }
        }
    };
    let service = service(opts.service.clone());

    match opts.cmd {
        CliCommand::PurgeDocker {} => {
            utils::exec_command("docker", vec!["system", "prune"]);
        }

        CliCommand::PurgeDb { db_folder, volume } => {
            django::purge_db(db_folder, volume);
        }

        CliCommand::Exec { workdir, cmd } => match cmd {
            ExecCommand::Command(command) => {
                docker_compose::exec(&opts.service, command, workdir);
            }
        },

        CliCommand::ManagePy { workdir, cmd } => match cmd {
            Some(py_cmd) => match py_cmd {
                ManagePyCommand::Command(manage_py_command) => {
                    django::exec_manage_py_cmd(&opts.service, Some(manage_py_command), workdir);
                }
            },

            None => {
                django::exec_manage_py_cmd(&opts.service, None, workdir);
            }
        },

        CliCommand::Start {
            service_name,
            build,
        } => {
            docker_compose::start(build, service_name);
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

        CliCommand::Restart { service_name, all } => {
            let service_to_restart = service(service_name);
            docker_compose::restart(all, &service_to_restart);
            docker_compose::logs(&service_to_restart, 10, false, all);
        }

        CliCommand::Stop { service_name } => {
            docker_compose::stop(service_name);
        }

        CliCommand::Rebuild { service_name } => {
            let service_to_rebuild = service(service_name);
            docker_compose::rebuild(&service_to_rebuild);
            docker_compose::logs(&service_to_rebuild, 10, false, false);
        }

        CliCommand::Build { service_name } => {
            docker_compose::build(&service(service_name));
        }

        CliCommand::ShowUrls {} => {
            django::show_urls(opts.service.as_str());
        }

        CliCommand::AddApp { name } => {
            django::add_app(name.as_str(), opts.service.as_str());
        }

        CliCommand::PyTest { tests_path, simple } => {
            django::pytest(tests_path, simple, opts.service.as_str());
        }

        CliCommand::Lint { cmd, path } => match cmd {
            Some(lint_job) => match lint_job {
                LintCommands::Black { custom_path } => {
                    if let Some(p) = custom_path {
                        django::black(p.as_str(), opts.service.as_str());
                    } else {
                        django::black(path.as_str(), opts.service.as_str());
                    }
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
            ssh_key,
        } => {
            deploy::execute(server_ip.as_str(), server_user.as_str(), ssh_key);
        }

        CliCommand::Logs { lines, follow, all } => {
            docker_compose::logs(&opts.service, lines, follow, all);
        }

        CliCommand::ShellPlus {} => {
            django::shell_plus(&opts.service);
        }
    }
}
