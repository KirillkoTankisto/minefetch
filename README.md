# MineFetch
[![Rustlang](https://img.shields.io/static/v1?label=Made%20with&message=Rust&logo=rust&labelColor=e82833&color=b11522)](https://www.rust-lang.org)
[![Github License](https://img.shields.io/github/license/KirillkoTankisto/minefetch?logo=mdBook)](https://github.com/KirillkoTankisto/minefetch/blob/main/LICENSE)
![Github commit activity](https://img.shields.io/github/commit-activity/t/KirillkoTankisto/minefetch)
![Status](https://img.shields.io/badge/development_status-alpha-purple?logo=GitHub)

> This project is inspired by [Ferium](https://github.com/gorilla-devs/ferium) but doesn't use its code. All code is written from scratch and may not work correctly
## Description
Minefetch is a Rust-based program created to simplify the process of downloading mods for Minecraft.
## Features:
### Search
#### You can search mods with this command:
```sh
minefetch search <your_query>
```
### Add
#### Adds a single mod. Not the best option. Using search is faster. Use this command to add one mod:
```sh
minefetch add <mod_slug_or_id>
```
### Profile
#### Firstly, you need to create a profile. You can add as much profiles as you want. To do this, use this command:
``` sh
minefetch profile create
```
#### To delete one profile, use this command:
``` sh
minefetch profile delete
```
#### And use this one to delete all profiles:
``` sh
minefetch profile delete all
```
#### To list all existing profiles, use this:
``` sh
minefetch profile list
```
#### To switch between profiles, use this command:
``` sh
minefetch profile switch
```
### Update
#### To update mods to the latest version, use:
``` sh
minefetch update
or
minefetch upgrade
```
### List
#### To list installed mods, use this command:
``` sh
minefetch list
```
### Version
#### To print current MineFetch version, type:
``` sh
minefetch version
```
## Installation
### Download
#### To download latest version, [Click Here](https://github.com/KirillkoTankisto/minefetch/releases/latest/download/minefetch)
### Build
#### To build MineFetch from source, you need Git, Rust and Cargo installed. Execute these commands to build MineFetch:
``` sh
git clone https://github.com/KirillkoTankisto/minefetch.git
cd minefetch
cargo build --release --target-dir .
```
#### executable will be created in *release* directory
## Contact
### To contact me, text me in Discord. My username:
```
notfunnyclown
```
