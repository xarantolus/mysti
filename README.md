Linux packages for client (maybe):

	sudo dnf install libX11-devel xorg-x11-fonts-misc xorg-x11-font-utils

Ubuntu:

	sudo apt update && sudo apt install -y libx11-dev xfonts-base xcb libx11-xcb-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxcb-shape0-dev libxcb-xfixes0-dev

## Daemon Setup
The daemon runs in the background of your devices and connects to the server, syncing events (like clipboard changes). It is available for many different Windows and Linux-based operating systems.

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

Now upon a reboot, the client should run in the background. Check using `pidof mysti-daemon-fedora`
