# tplayer-bot

`tplayer-bot` is a Twitch bot designed to make it easier to watch videos with others through Twitch by controlling mpv on Linux through Twitch commands. It allows for users that you authorize in twitch chat to use commands like `!pause`, `!play`, `!rewind`, `!forward` and more to control mpv through Unix sockets.

## List of commands

If a command has an argument, which is denoted through padding angle brackets, the angle brackets themselves are not supposed to be typed in the command. For example, the correct command to rewind by one minute would be `!rewind 60` and **not** `!rewind <60>`.

| Command | Description |
| - | - |
| `!play` | Play the video from a paused state. |
| `!pause` | Pause the video if it is playing. |
| `!rewind <s>` | Go back `s` seconds. |
| `!forward <s>` | Skip ahead `s` seconds. |
| `!pos` | In Twitch chat, display information about the current position of the video |
| `!sub <s>` | Set subtitle track to number `s`. A value of 0 turns subtitles off. |
| `!joke` | Tell a joke in the Twitch chat |

**Note:** The value `s` in the `!rewind`, `!forward`, and `!sub` commands are represented as unsigned 16-bit integers, so `s` must be an integer in the range [0, 65535]. If `s` is not specified, or if `s` failed to parse, a value of 10 (except for `!sub`, which will use 0) is used instead. This also means that floating point values, i.e. numbers with a decimal, cannot be used.

All messages in the Twitch chat that do not start with the bang character (`!`), or that do not match a command, are ignored. However, if a command's argument failed to parse, it will still run, though with the default argument as specified above.

## Basic setup

`tplayer-bot` only works on Linux with the mpv video player.

Ensure that `socat` is installed on the computer. `socat` is used to write to Unix sockets to communicate with mpv. On Arch-based distros, just do `pacman -S socat`.

### Launching mpv

This program uses Unix sockets to programmatically control mpv, so mpv must be launched like this:

```bash
mpv --input-ipc-server=/tmp/mpvsocket <video_file>
```

If you are using this program often, it might be a good idea to set an alias in your shell for `mpv --input-ipc-server=/tmp/mpvsocket` so that it's easier to type and remember.

For now, you have to use `/tmp/mpvsocket` as the directory for the IPC server. However, this might be configurable in the future.

### Streaming software

In theory, any streaming software to capture mpv could be used. However, OBS Studio is recommended since it works well and is open source.

#### **OBS Studio with hardware-accelerated encoding**

If you have a dedicated GPU on your computer, try to use hardware encoding, as it will generally be "smoother"/faster than software encoding. On OBS Studio, this option won't be available if you have an Nvidia GPU unless you have a version of ffmpeg that enables certain optional features necessary for proprietary hardware-accelerated encoding. On Arch-based distros, you can get a version of ffmpeg that enables these features (and more) via the `ffmpeg-full` AUR package. On other distros, you might have to compile ffmpeg yourself with the necessary proprietary hardware-accelerated encoding features enabled at build-time.

To get ONLY the output of mpv (and not the entire desktop), the following commands can be used:

```bash
# create a virtual sink called sink1 with this command:
pactl load-module module-virtual-sink sink_name=sink1

# open pavucontrol and set mpv to output audio to sink1

# open OBS Studio and capture audio output source from monitor of sink1
# in OBS Studio, also capture window output of mpv
```

### Configuring and launching `tplayer-bot`

Ensure that you have a Rust toolchain installed to compile `tplayer-bot`.

`tplayer-bot`'s configuration is simple; it is based on environment variables. It also automatically reads environment variables from a `.env` file if it exists in the current directory.

To use this `tplayer-bot`, you should create a Twitch account specifically for this bot. Then, get an API token from Twitch for that account to use it `tplayer-bot`.

#### **List of environment variables needed**
| | |
| - | - |
| `BOT_USERNAME` | Username of the bot you created |
| `CHANNEL_NAME` | Username of the channel on Twitch to read comments from (doesn't have to be the same as `BOT_USERNAME`, but can be) |
| `OAUTH_TOKEN` | Token from Twitch API for the bot as specified by `BOT_USERNAME` |
| `AUTHORIZED_USERS` | A comma-separated list of users that are authorized to use the bot (i.e., the bot only listens to usernames that are specified here) |

Once you have set up everything else (OBS Studio or any streaming software, created a Twitch account for this bot and have an API token from Twitch, launched mpv to use `/tmp/mpvsocket` as the IPC server), use the following commands to download, configure, compile, and run `tplayer-bot` itself:

```bash
git clone https://github.com/redzic/tplayer && cd tplayer

# create a .env file based on the required environment variables, as stated above
# vim is used as an example text editor
vim .env

# build and run the project in release mode
cargo run --release
```
