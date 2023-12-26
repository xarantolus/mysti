# Mysti
Mysti is a cross-platform event synchronization software. It can be used to sync clipboard events between different devices, turn computers on and off via the network and other features.

Current features:
- Copy text and images between different computers (works on Windows and Linux)
- Turn on a computer via Wake on LAN (if on the same network as your server and configured correctly)
- Turn off, log off or reboot a computer remotely

### Build from source
This section describes how to build the software from source.

### Client
You can build both the server and daemon from source. For the daemon, you might need to install additional libraries, which you can find listed in [the CI config file](.github/workflows/build-client.yml).

### Server
The server is built in Docker. Feel free to contribute additional common configurations.

## Server Setup
Before using the client, you need to have a working server setup. You need to install Docker on your system to run the server.

For your setup, you can copy one of the directories in the [deployment](deployment) directory.

```shell
deployment/arm64v8-namecheap$ tree -aA
.
├── Caddyfile
├── config.toml
├── docker-compose.yml
├── Dockerfile
├── Dockerfile.caddy
└── .env

1 directory, 6 files
```

First of all, make sure to edit the Caddyfile to add your own E-Mail and other host information. You might need to edit the `Dockerfile.caddy` to add custom Caddy modules to support your DNS provider.

Once all the information has been added, you can build your setup:

	docker compose build --pull



## Daemon Setup
The daemon should run in the background of your devices and connect to the server, syncing events (like clipboard changes) as they happen. It is available for many Windows and Linux-based operating systems.

This section shows how to set up the client to start on user login.

### Configuration
First of all, we need to tell the client which server to connect to. You can do this by creating a configuration file with [this content](deployment/daemon/daemon-config.toml):

```toml
# This is the mysti daemon configuration file.
# Depending on the operation system, it is expected in the following locations:
#   Linux/Mac: $XDG_CONFIG_HOME/mysti.toml, $HOME/.config/mysti.toml or working directory
#   Windows: %USERPROFILE%\.config\mysti.toml or working directory

# The client connects to the server specified
# If you use Caddy or some other reverse proxy,
# make sure to specify the correct port (the output port of Caddy)
server_host = "https://my.host.com:1234"

# The token is used to authenticate the client with the server.
# When you set up the server, you should have generated a token
# that is also specified in the servers' configuration file.
token = "my cool token"
```

It is recommended to put the configuration file into `~/.config/mysti.toml` on Linux.

### Windows

### Fedora Linux (39)
To create an autostart entry, edit the autostart file:

```
nano ~/.config/autostart/mysti-daemon.desktop
```

Paste in the following content and save the file:

```desktop
[Desktop Entry]
Name=Mysti daemon
Exec=mysti-daemon
Type=Application
```

Now upon a reboot, the daemon should run in the background. Check using `pidof mysti-daemon-fedora`
