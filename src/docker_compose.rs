use crate::utils::exec_command;

pub const DOCKER_COMPOSE: &str = "docker-compose";

/// Starts containers
pub fn start(build: bool, container: Option<String>) -> bool {
    if build {
        let mut args = vec!["build", "--force-rm", "--parallel"];
        if let Some(service) = &container {
            args.push(service);
        }
        exec_command(DOCKER_COMPOSE, args);
    }

    let mut args = vec!["up", "-d"];
    if let Some(service) = &container {
        args.push(service);
    }
    exec_command(DOCKER_COMPOSE, args)
}

/// Stops and removes all containers
pub fn stop(service: Option<&str>) -> bool {
    let mut cmd_params = vec!["rm", "--stop", "--force", "-v"];
    if let Some(service) = service {
        cmd_params.push(service);
    }
    exec_command(DOCKER_COMPOSE, cmd_params)
}

/// Restart all containers or just one
pub fn restart(all: bool, service: &str) -> bool {
    if all {
        exec_command(DOCKER_COMPOSE, vec!["restart"])
    } else {
        exec_command(DOCKER_COMPOSE, vec!["restart", service])
    }
}

/// Rebuild specific container
pub fn rebuild(service: &str) -> bool {
    if !stop(Some(service)) {
        return false;
    }
    if !exec_command(DOCKER_COMPOSE, vec!["build", "--force-rm", service]) {
        return false;
    }
    exec_command(DOCKER_COMPOSE, vec!["up", "-d", service])
}

/// Show containers status
pub fn status() -> bool {
    exec_command(DOCKER_COMPOSE, vec!["ps", "--all"])
}

/// Show logs for container
pub fn logs(service: &str, num_lines: i32, follow: bool) -> bool {
    let tail = format!("--tail={}", num_lines.clone());
    let mut args = vec!["logs", "--timestamps", &tail];
    if follow {
        args.push("--follow");
    }
    args.push(service);
    exec_command(DOCKER_COMPOSE, args)
}
