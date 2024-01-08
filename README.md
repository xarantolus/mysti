# Mysti
Mysti is a cross-platform event synchronization software. It can be used to sync clipboard events between different devices, turn computers on and off via the network and other features.

Current features:
- Copy text and images between different computers (e.g. copy between a Windows PC and Linux Laptop)
- Turn on a computer via Wake on LAN (if on the same network as your server and configured correctly)
- Any other user-defined shell command: turn off, log off, reboot a computer remotely, etc.

## Overview
Mysti has three components:
- A **server** that clients can connect to for exchanging events
- A **daemon** that runs in the background on client PCs to process events (runs commands and sends clipboard changes to the server)
- A **CLI** that can be used to send commands interactively

![Architecture Diagram](.github/img/architecture.png)

- Table of contents
  - [Overview](#overview)
  - [Build from source](#build-from-source)
  - [Server](#server)
    - [Server Setup](#server-setup)
  - [Daemon and CLI Setup](#daemon-and-cli-setup)
    - [Client Configuration](#client-configuration)
    - [Automatically start the daemon on boot](#automatically-start-the-daemon-on-boot)
      - [Windows](#windows)
      - [Fedora Linux (39)](#fedora-linux-39)
    - [Sending commands](#sending-commands)
  - [Limitations](#limitations)

<details>

<summary>Instruction for building from source</summary>

### Build from source
This section describes how to build the software from source.

You can build both the server, daemon and CLI from source using `cargo build` in their directories (or `make` in the main directory to install CLI/Daemon).

For the daemon, you might need to install additional libraries, which you can find listed in [the CI config file](.github/workflows/build-client.yml).

## Server
The server is built in Docker. Feel free to contribute additional common configurations.

</details>


### Server Setup
Before using the daemon/CLI, you need to have a working server setup. You need to install Docker on your system to run the server.

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

Then make sure to edit `config.toml` with the actual settings of the Mysti server:

```toml
# the HTTP port the server should listen on
web_port = 43853

# a custom token to use for authentication - you can e.g. generate one with
#    xxd -l 30 -p /dev/urandom
token = "your custom token to use as password"

[wake_on_lan]
# The target address is the MAC address of the PC you want to wake up.
# The mysti server must be on the same network as the PC.
target_addr = "AA:AA:AA:AA:AA:AA"
router_addr = "255.255.255.255"
```

Once all the information has been added, you can build your setup:

	docker compose build --pull

Make sure the correct ports are exposed and then try running the daemon.

You can also add custom clipboard actions, which are commands that are executed when the clipboard matches a regex:

```toml
[[clipboard_action]]
regex = '(http(?:.*?)music\.youtube\.com(?:\S+))'
command = "curl 'http://sensiblehub-server:128/add?format=json' -X POST --data-raw '{\"searchTerm\":\"$1\"}'"
```

In this example, every time we find a YouTube Music URL, it gets sent to [a server](https://github.com/xarantolus/sensibleHub) via a cURL command. You can execute almost any command. Note that these commands run in the container, however, since we mount the host at `/host`, we can still run commands kind of on the host. This means that many commands will work, except for scripts that expect fixed paths (e.g. in a shebang). For Python scripts, instead of directly executing them (thus using the shebang), run `python script.py` or `python -m my_module` instead of `./script.py` or a typical wrapper that has a shebang.

## Daemon and CLI Setup
The daemon should run in the background of your devices and connect to the server, syncing events (like clipboard changes) as they happen. It is available for many Windows and Linux-based operating systems. The CLI is an additional helper for sending remote commands to other connected clients.

First of all, you can download the daemon and CLI binaries for your client(s) from [the latest release here on GitHub](http://github.com/xarantolus/mysti/releases/latest). For Windows, you would download and unzip `mysti-windows.zip`. Then you can unzip it into a directory that is available on your `$PATH`. The same goes for Linux.

### Client Configuration
First of all, we need to tell the daemon which server to connect to. You can do this by creating a configuration file with [this content](deployment/daemon/daemon-config.toml):

```toml
# This is the mysti daemon configuration file.
# Depending on the operation system, it is expected in the following locations:
#   Linux/Mac: $XDG_CONFIG_HOME/mysti.toml, $HOME/.config/mysti.toml or working directory
#   Windows: %USERPROFILE%\.config\mysti.toml or working directory

# The daemon connects to the server specified
# If you use Caddy or some other reverse proxy,
# make sure to specify the correct port (the output port of Caddy)
server_host = "https://my.host.com:1234"

# The token is used to authenticate the client with the server.
# When you set up the server, you should have generated a token
# that is also specified in the servers' configuration file.
token = "my cool token"

# How to send a wake on lan request to the PC specified on the server
# E.g. if set to "on", you run "mysti on" on your command line, otherwise the second part is whatever you specify here
wol_shortcut = "on"

# Here you can define any number of actions that are possible on this client device. Only the configuration used for the current OS is used (e.g. linux on Linux).
# You do not need to specify all possible options, e.g. you can omit the windows one on Linux
[[action]]
name = "Shutdown"
linux = "shutdown -h now"
windows = "shutdown /s /f /t 0"

# Just copy and edit this section as many times as you want:
[[action]]
name = "Reboot"
linux = "shutdown -r now"
windows = "shutdown /r /f /t 0"
```

You can add any number of actions, as long as they have a different name.

It is recommended to put the configuration file into `~/.config/mysti.toml` on both Linux and `%USERPROFILE%\.config\mysti.toml` on Windows (you might have to create the `.config` directory yourself). That way, both the CLI and daemon can find the same configuration file.

### Automatically start the daemon on boot
This section shows how to set up the daemon to start on user login. This is sadly very different between operating systems, so make sure to look for additional guides for your specific setup.

#### Windows
First of all, create a directory where you unzip `mysti.exe` and `mysti-daemon.exe`. Add that directory to the `PATH` environment variable.

Since we want to start the daemon without a console window visible, we need to create a wrapper script that starts the daemon. Create `mysti.vbs` with the following content:

```vbs
Set objShell = CreateObject("WScript.Shell")
objShell.Run "mysti-daemon", 0
Set objShell = Nothing
```

Then you can add a shortcut to `mysti.vbs` to your startup folder (open it with `Win+R` and `shell:startup`). Now when you start your machine, the script will be run, which runs the daemon.

#### Fedora Linux (39)
Again unzip the release file for Fedora into some directory, run `chmod +x *` in that directory and add it to your `$PATH`.

Then, to create an autostart entry, edit the autostart file:

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

Now upon a reboot, the daemon should run in the background. Check using `pidof mysti-daemon-fedora`.


### Sending commands
After defining commands and setting up daemons on at least one device, you can now run `mysti` in your terminal to control other devices:

```
$ mysti
Select a client to run an action |
> phili on Philipp-PC (Microsoft Windows 11 Pro), connected 442s ago
  philipp on philipp (Fedora Linux 39 (Workstation Edition)), connected 459s ago
```

Select the device you want to run a command on. By default, the first device that is not the device you ran `mysti` on will be selected (but you can also send commands to yourself). If only one device is available, it's selected by default and the menu will not be shown.

Now we see the device action menu:

```
$ mysti
Select a client to run an action: phili on Philipp-PC (Microsoft Windows 11 Pro), connected 442s ago
Which action do you want to run? |
> Shutdown
  Reboot
```

Select the action you want to run and press enter.

```
Running action Shutdown on client phili on Philipp-PC (Microsoft Windows 11 Pro)
Sent action.
```

This sends the action to the server. Note that we don't get feedback of whether the action worked correctly.

### Limitations
- On Linux, clipboard sync support depends on your setup (X11 vs. Wayland). In theory all options are supported, but I can't really test it. On my Fedora 39 machine with Wayland (and `Xwayland`), clipboard events works
- Some image copy operations might not be synced. However, the daemon tries its best to convert between different formats and uses a common format to sync between all platforms

### [License](LICENSE)
This is free as in freedom software. Do whatever you like with it.
