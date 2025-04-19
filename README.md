# MineFetch

[![Rustlang](https://img.shields.io/static/v1?label=Made%20with&message=Rust&logo=rust&labelColor=e82833&color=b11522)](https://www.rust-lang.org) [![Github License](https://img.shields.io/github/license/KirillkoTankisto/minefetch?logo=mdBook)](https://github.com/KirillkoTankisto/minefetch/blob/main/LICENSE) ![Github commit activity](https://img.shields.io/github/commit-activity/t/KirillkoTankisto/minefetch) ![Status](https://img.shields.io/badge/development_status-beta-orange?logo=GitHub)

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

Lock, unlock and list your locks with these commands:

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

## Installation

### Download Pre-built Binary

Download the latest version [here](https://github.com/KirillkoTankisto/minefetch/releases/latest/download/minefetch).

### Build from Source

To build MineFetch from source, ensure you have Git, Rust, and Cargo installed, then run:

```sh
git clone https://github.com/KirillkoTankisto/minefetch.git
cd minefetch
./build
```

The executable will be located in the `release` directory.

## Contact

For inquiries or support, reach me on Discord at `notfunnyclown`.
