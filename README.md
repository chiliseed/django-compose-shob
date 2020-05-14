# django-compose-shob (ddc-shob)
Command line utility to simplify developers life. Short and precise cli commands to interact with django server running in docker container via docker compose.


# What's included

By default, `ddc-shob` works with `api` docker compose service, unless another service name is provided.
This means that if in your docker-compose file, you defined a service named `api`, you can call all commands
without providing service name.

1. `start` - start all docker compose services, with optional build step.
2. `restart` - restarts all or only one container. 
3. `stop` - stops all docker compose services and removes their containers.
4. `rebuild` - cleanly stops relevant container, runs docker compose build and starts the services again.
5. `purge-db` - stops all docker compose services, removes db folder and starts all services again. Defaults to `pg` folder, unless another path was provided.
6. `migrate` - runs `python manage.py migrate`. If you provide specific application and optionally migration number (for rollback), it will be passed on to migrate command. To fully revert all migrations for a specific application provide `zero` as migration number.
If there will be an interest, I will add a separate command for full rollback. 
7. `show-urls` - useful if you have `django-extensions`, this will output all urls in the service.
8. `add-app` - adds new django application in your project.
9. `lint` - run different lint jobs. For a full list run with `--help`
10. `py-test` - run pytest inside the container.
11. `logs` - show logs for container.
12. `shell-plus` - useful if you have `django-extensions`, this will open python shell in provided container.
13. `deploy` - experimental feature at this point. Simply call deploy from inside a directory ready to be packaged, uploaded to the server, that has docker-compose. 
On server, docker-compose will be used to build the images and start the service in daemon mode.

# Example usage

If your `docker-compose.yml` has service named `api`, all command that target specific service, will use `api` container by default:

```bash
ddc-shob restart
```

will restart `api` container.

To apply commands to different container, provide name of that container in your command:

```bash
ddc-shob web restart
```

will restart `web` container.

# Supported operating systems

1. Mac OS x
2. All Unix OS

# How to install via cargo

1. Clone the repo
2. cd to/repo
3. `cargo install --path .`


# How to download the executable

Once it will reach a stable version, we will setup an easy way to download the executable.


# Issues and features suggestions

Please open a ticket with relevant details.
