# Song Sequence Director

Small web app for signaling song progression sequence, e.g. verse 1, chorus, verse 2, etc. Built using the [Leptos](https://github.com/leptos-rs/leptos) web framework and the [Leptos Axum starter template](https://github.com/leptos-rs/start-axum).

## Usage

Run the `start.bat` batch file to start the server. You can then open the homepage by navigating to `localhost:3000` on a browser on the same computer. You will most likely also need at least one device, such as a phone or tablet, connected to the same local network as the host computer, which the song leader can use. The alternative is to set up an instance of the server that can be accessed on the Internet, which will not be covered here.

The homepage is the director page with buttons for setting the signal. The letter buttons set the signal to the respective letters. The number buttons append the respective numbers to any of the letter signals. The number buttons will not have any effect if there is no current letter signal. The dash `-` clears the signal. The current signal is displayed at the top of the page.

The `/view` page simply displays the current signal. This is mainly intended to be used as an OBS browser source or similar to display the signal, but it can also be used directly in a browser if team members can access the web server from their own devices.

The signal displayed on the director page also synchronises with any changes from other directors, in case you have multiple song leaders.

The intended meaning for each letter is as follows, but you can of course agree on any meaning with your team:

- C: Chorus
- V: Verse
- B: Bridge
- P: Pre-chorus
- W: Worship (Instruments only)
- E: Ending/Last line
- X: Stop/Finish
- R: Repeat/Play on

## Building

Prerequisites:
1. Install the Rust nightly toolchain, e.g. using rustup.
2. Install [cargo-leptos](https://github.com/leptos-rs/cargo-leptos).

After running a `cargo leptos build --release` the minimum files needed are:

1. The server binary located in target/server/release
2. The site directory and all files within located in target/site

Copy these files to your remote server. The directory structure should be:

```
song-sequence-director
site/
```

Set the following environment variables:

```
LEPTOS_OUTPUT_NAME="song-sequence-director"
LEPTOS_SITE_ROOT="site"
LEPTOS_SITE_PKG_DIR="pkg"
LEPTOS_SITE_ADDR="0.0.0.0:3000"
LEPTOS_RELOAD_PORT="3001"
```

Finally, run the server binary.

Setting the environment variables and running can be done using a batch file, for example:

```
@echo off
set "LEPTOS_OUTPUT_NAME=song-sequence-director"
set "LEPTOS_SITE_ROOT=site"
set "LEPTOS_SITE_PKG_DIR=pkg"
set "LEPTOS_SITE_ADDR=0.0.0.0:3000"
set "LEPTOS_RELOAD_PORT=3001"
song-sequence-director.exe
```

The `0.0.0.0` address binds to all available IPs. If you instead want it to bind only to a specific IP, change it as appropriate.

### Pre-compressed static files

The static file serving is configured to support files pre-compressed using Brotli, so you can optionally pre-compress the files in the `site` folder to reduce the size of files transferred over the network. You can use a tool such as [static-compress](https://github.com/mqudsi/static-compress) to do this.

## Attributions

The music notes used for the favicon were obtained from <a href="https://www.flaticon.com/free-icons/music" title="music icons">Freepik on Flaticon</a>
