This thing should give you a page to explore mwo mechs and their quirks, maybe.

# Running

## Prerequisites

You'd need:
 - Rust (get from https://www.rust-lang.org/tools/install)
 - `wasm-pack` (get via `cargo install wasm-pack`)
 - maaaaybe `miniserve` (get via `cargo install miniserve`)

## Extracting the data from the game files

```sh
cargo run "D:\games\SteamLibrary\steamapps\common\MechWarrior Online\"
```
or whatever is your path to the game on your system.

## Create the website

```sh
wasm-pack build --target web --out-name wasm --out-dir ./static/pkg
```

## Actually serve the resulting thing

Any of your common web servers should do. But as a specific example:

```sh
miniserve ./static --index index.html
```

# Development status

The code is rather very hacky, contributing is not advised.

As of 2021-04-28 this is still in active development. If you're reading this more than a month into the future, then i've either forgotten to update the number or (more likely) i got disinterested and abandoned the project (in which case, PRs welcome).