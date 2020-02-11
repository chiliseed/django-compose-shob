use crate::docker_compose::DOCKER_COMPOSE;
use crate::utils::exec_command;

/// Execute python manage.py command
fn exec_manage_command(service: &str, args: Vec<&str>) -> bool {
    let cmd_args = vec!["exec", service, "python", "manage.py"];
    exec_command(DOCKER_COMPOSE, [cmd_args, args].concat())
}

/// Run migrations for all or a specific application.
/// If `migration_number` is supplied, will not run makemigrations and instead wil migrate to specific migration.
/// This is essentially a rollback.
pub fn migrate(
    service: &str,
    application: Option<String>,
    migration_number: Option<String>,
    empty: bool,
    migration_name: Option<String>,
) -> bool {
    let mut make_migration_args = vec!["makemigrations"];

    if empty {
        make_migration_args.push("--empty");
        if let Some(mname) = &migration_name {
            make_migration_args.push("--name");
            make_migration_args.push(mname);
        }

        if let Some(app) = application {
            make_migration_args.push(&app);
            return exec_manage_command(service, make_migration_args);
        }
        eprintln!("Must provide application name");
        return false;
    }

    let mut migrate_args = vec!["migrate"];

    match application {
        Some(app) => {
            migrate_args.push(app.as_str());
            match migration_number {
                Some(migration) => {
                    migrate_args.push(migration.as_str());
                    exec_manage_command(service, migrate_args)
                }

                None => {
                    make_migration_args.push(&app);
                    if let Some(mname) = &migration_name {
                        make_migration_args.push("--name");
                        make_migration_args.push(mname);
                    }
                    if !exec_manage_command(service, make_migration_args) {
                        return false;
                    }
                    exec_manage_command(service, migrate_args)
                }
            }
        }
        None => {
            if !exec_manage_command(service, make_migration_args) {
                return false;
            }
            exec_manage_command(service, migrate_args)
        }
    }
}

/// ATTENTION! This is a destructive action.
/// Stops all containers and removes db folder.
/// `db_folder` is the local file system location where the db is mapped to.
/// By default assumes `./pg` directory path.
pub fn purge_db(db_folder: String, volume: Option<String>) -> bool {
    if !exec_command(DOCKER_COMPOSE, vec!["rm", "--stop", "--force"]) {
        return false;
    }
    match volume {
        Some(volume_name) => {
            if !exec_command("docker", vec!["volume", "rm", volume_name.as_str()]) {
                return false;
            }
        }
        None => {
            if !exec_command("rm", vec!["-rf", db_folder.as_str()]) {
                return false;
            }
        }
    }
    exec_command(DOCKER_COMPOSE, vec!["up", "-d"])
}

/// Executes django_extensions management command - show_urls
pub fn show_urls(service: &str) -> bool {
    exec_manage_command(service, vec!["show_urls"])
}

/// Add new django application
pub fn add_app(app_name: &str, service: &str) -> bool {
    exec_manage_command(service, vec!["startapp", app_name])
}

/// Execute pytest in container
pub fn pytest(path: Option<String>, service: &str) -> bool {
    let mut pytest_cmd = vec!["exec", service, "pytest"];
    match path {
        Some(tests) => {
            pytest_cmd.push(tests.as_str());
            exec_command(DOCKER_COMPOSE, pytest_cmd)
        }

        None => exec_command(DOCKER_COMPOSE, pytest_cmd),
    }
}

pub fn black(path: &str, service: &str) -> bool {
    exec_command(DOCKER_COMPOSE, vec!["exec", service, "black", path])
}

pub fn flake8(path: &str, service: &str) -> bool {
    exec_command(
        DOCKER_COMPOSE,
        vec!["exec", service, "flake8", path, "--exclude=migrations"],
    )
}

pub fn prospector(path: &str, service: &str) -> bool {
    exec_command(DOCKER_COMPOSE, vec!["exec", service, "prospector", path])
}

pub fn pydocstyle(path: &str, service: &str, convention: &str) -> bool {
    exec_command(
        DOCKER_COMPOSE,
        vec![
            "exec",
            service,
            "pydocstyle",
            "--convention",
            convention,
            path,
            "--match-dir=^(?!migrations).*",
        ],
    )
}

pub fn mypy(path: &str, service: &str, level: &str) -> bool {
    exec_command(
        DOCKER_COMPOSE,
        vec![
            "exec",
            service,
            "mypy",
            path,
            format!("--{}", level).as_str(),
        ],
    )
}

/// Run linters that don't require special configuration
pub fn lint(path: &str, service: &str) -> bool {
    if !exec_command(DOCKER_COMPOSE, vec!["exec", service, "black", path]) {
        return false;
    }
    if !exec_command(
        DOCKER_COMPOSE,
        vec!["exec", service, "flake8", path, "--exclude=migrations"],
    ) {
        return false;
    }
    exec_command(DOCKER_COMPOSE, vec!["exec", service, "prospector", path])
}
