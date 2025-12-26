# MineFetch

[![Rust Language](https://img.shields.io/badge/Built%20with-Rust-red?logo=rust&style=for-the-badge)](https://www.rust-lang.org)
[![Github License](https://img.shields.io/badge/License-GPLv3-green?logo=gplv3&style=for-the-badge)](https://github.com/KirillkoTankisto/minefetch/blob/main/LICENSE)
![Github Commits](https://img.shields.io/github/commit-activity/t/KirillkoTankisto/minefetch?logo=git&style=for-the-badge)
![Status](https://img.shields.io/badge/development_status-beta-orange?logo=GitHub&style=for-the-badge)

> This project is inspired by [Ferium](https://github.com/gorilla-devs/ferium) but doesn’t use its code. All code is written from scratch and may not work correctly.

## Description

Minefetch is a Rust-based program designed to simplify downloading mods for Minecraft.

## Features

### Search

Search for mods using:

```sh
minefetch search <your_query>
```

### Add

Add a single mod directly with its slug or ID:

```sh
minefetch add <mod_slug_or_id>
```

**Note:** For a more efficient workflow, consider using the `search` command.

### Profile Management

Manage your profiles with these commands:

- **Create a profile:**
  ```sh
  minefetch profile create
  ```

- **Delete a profile:**
  ```sh
  minefetch profile delete
  ```

- **Delete all profiles:**
  ```sh
  minefetch profile delete all
  ```

- **List all profiles:**

  ```sh
  minefetch profile list
  ```

- **Switch between profiles:**
  ```sh
  minefetch profile switch
  ```

### Update Mods

Update your mods to the latest version with either:

```sh
minefetch update
```

or

```sh
minefetch upgrade
```

### List Installed Mods

List all installed mods:

```sh
minefetch list
```

### Lock mods

Locks enable you to decide what mods shouldn't update. Lock, unlock and list your locks with these commands:

- **Add a lock**

```sh
minefetch lock add
```

- **Remove a lock**

```sh
minefetch lock remove
```

- **List locks**
```sh
minefetch lock list
```

### Edit mods

Edit the mod:

```sh
minefetch edit
```

### Check MineFetch Version

Display the current MineFetch version:

```sh
minefetch version
```

### Help

Display help message:

```sh
minefetch help
```

# Installation

## Download Pre-built Binary

Download the latest version [here](https://github.com/KirillkoTankisto/minefetch/releases/latest).

## Build from Source

To build MineFetch from source, ensure you have `make`, `git`, `rustup` and `cargo-zigbuild` installed.

### 1. Clone the repo:
```sh
git clone https://github.com/KirillkoTankisto/minefetch.git
cd minefetch
```

### 2.1 Build for x86_64, aarch64, riscv64:
```sh
make build
```
The output binaries will be packaged in ./build-cross/

### 2.2 Build for your host's target:
```sh
make native
```
The output binary will appear in ./target/release/

### 2.3 Build for a custom target:
```sh
make <target-triple>
```

example:
```sh
make x86_64-pc-windows-gnullvm
```
the output binary will appear in ./target/<target-triple>/

## Contact
For inquiries or support, reach me on Discord at `notfunnyclown`.
