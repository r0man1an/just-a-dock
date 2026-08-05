## just-a-dock

<img width="3056" height="506" alt="Screenshot From 2026-08-04 15-20-51" src="https://github.com/user-attachments/assets/9ccf79bd-9dff-4f5f-b144-21df004e0df3" />


As the name implies just-a-dock is just a dock - nothing more, nothing less. 
It aims to provide a simple, beautiful, modern and lghtweight dock for your wlroots-based Wayland compositors.
## Tools used

* **Programming language:** [Rust](https://www.rust-lang.org/)
* **GUI toolkit:** [GTK4](https://gtk.org/)
* **Layer shell library:** [gtk4-layer-shell](https://github.com/wmww/gtk4-layer-shell)
* **Underlying protocol:** [Wayland](https://wayland.freedesktop.org/) (`wayland-client` + `wayland-protocols-wlr`)
## Building from Source

### Requirements

Install the required dependencies:

#### Arch Linux

```bash
sudo pacman -S rust cargo gtk4 gtk4-layer-shell pkgconf
```

#### Fedora

```bash
sudo dnf install rust cargo gtk4-devel gtk4-layer-shell-devel pkgconf-pkg-config
```

#### Ubuntu / Debian

```bash
sudo apt install rustc cargo libgtk-4-dev libgtk4-layer-shell-dev pkg-config
```

You also need Cargo's binary directory in your `PATH`.

Add this line to your shell configuration:

For Bash:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

For zsh:

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
```

### Clone the repository

```bash
git clone https://github.com/r0man1an/just-a-dock.git
cd just-a-dock
```

### Build

```bash
cargo build --release
```

### Install

Install `jdock`:

```bash
cargo install --path .
```

### Usage

Start Just a Dock:

```bash
jdock
```

Open the configuration GUI:

```bash
jdock --config
```
