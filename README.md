## just-a-dock

<img width="1920" height="171" alt="gscreenshot_2026-08-18-210033" src="https://github.com/user-attachments/assets/a020c93d-b0f7-459f-b690-61b10015a7cf" />

---

As the name implies just-a-dock is just a dock - nothing more, nothing less. 
It aims to provide a simple, beautiful, modern and lghtweight dock for your wlroots-based Wayland compositors.

## Features
- Lightweight (~45MB)
- Customizable through a GUI
- Cross-compositor support (runs on all your wlroots-based compositors)
  
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
