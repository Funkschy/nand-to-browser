# VM development targeting the web browser with Rust and WebAssembly

A Nand to Tetris Emulator implementation that can run in the browser or on your Desktop

# Dependencies
## General
- [Rust](https://www.rust-lang.org/)
## Web
- [wasm-pack](https://rustwasm.github.io/wasm-pack/installer/)
- [npm](https://github.com/npm/cli)
## Desktop
- [SDL2](https://www.libsdl.org/)

# Web Mode

To use the web version, we need to build a WebAssembly library and then host the Javascript frontend with some file server (npm run start will start the webpack development server)

``` shell
# switch into the web ui folder
cd www
# compile the rust code into a wasm lib
wasm-pack build --release
# pull the javascript dependencies
npm ci
# run the javascript server
npm run start
```

Then just open [localhost on port 8080](http://localhost:8080) in your browser

# Desktop Mode
## Compilation
``` shell
# compile the application into a desktop version
# for desktop mode (with a graphical user interface)
cargo build --release --features desktop

# for headless mode (to only run test scripts without seeing the UI)
cargo build --release
```

## Usage

``` shell
target/release/n2t <DIR> # dir should contain the VM files, and at most one Test and one Compare script
```

To get a list of all available options just run `target/release/n2t --help`

# Progress

See the notes.org file for a list of checkboxes
