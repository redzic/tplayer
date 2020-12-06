use hashbrown::{HashMap, HashSet};
use rand::Rng;
use std::env;
use twitchchat::PrivmsgExt as _;
use twitchchat::{
    messages::{Commands, Privmsg},
    runner::{AsyncRunner, Status},
    UserConfig,
};

// TODO: log when there's an error with the mpv connection

mod jokes;
mod mpv;

macro_rules! get_env_var {
    ($v:expr) => {
        env::var($v).map_err(|e| {
            println!("{} not found", $v);
            anyhow::Error::from(e)
        })
    };
}

/// Get a user config from the environment variables, printing an error message and returning
/// and error if it fails.
fn get_user_config() -> anyhow::Result<twitchchat::UserConfig> {
    let name = get_env_var!("BOT_USERNAME")?.to_ascii_lowercase();

    let token = get_env_var!("OAUTH_TOKEN")?;

    UserConfig::builder()
        .name(name)
        .token(token)
        .enable_all_capabilities()
        .build()
        .map_err(anyhow::Error::from)
}

macro_rules! cmd {
    ($bot:ident, $cmd:ident) => {
        ($bot).add_command(concat!('!', stringify!($cmd)), &mut $cmd);
    };
}

#[allow(unused_macros)]
macro_rules! cmd_say {
    ($bot:ident, $cmd:ident, $str:expr) => {
        let mut $cmd = |args: Args| {
            args.writer.say(args.msg, $str);
        };
        cmd!($bot, $cmd);
    };
}

/// Send a command to mpv through the Unix socket
macro_rules! cmd_mpv {
    ($bot:ident, $cmd:ident, $str:expr) => {
        let mut $cmd = |_: Args| {
            let _ = mpv::send_command($str);
        };
        cmd!($bot, $cmd);
    };
}

// The !rewind and !forward commands are essentially the same except for one operation,
// which is to either add or subtract the desired offset from the current time. A
// macro implements both of these slightly different functionalities without any sort
// of dynamic dispatch and without having any code duplication.
macro_rules! cmd_offset {
    ($bot:ident, $name:ident, $op:tt) => {
        let mut $name = |args: Args| {
            if let CmdArgs::U16(seconds) = args.args {
                let time_pos = match mpv::get_property_as::<f64>("time-pos") {
                    Some(time_pos) => time_pos,
                    _ => return,
                };

                let _ = mpv::send_command(
                    format!(
                        "{{ \"command\": [\"set_property\", \"time-pos\", {}] }}",
                        time_pos $op seconds as f64
                    )
                    .as_str(),
                );
            }
        };
        cmd!($bot, $name);
    };
}

/// Formats the time of `s` (in seconds) into a human-readable string
#[must_use]
pub fn format_time(s: u64) -> String {
    let hours = ((s / 60) / 60) % 60;
    let minutes = (s / 60) % 60;
    let seconds = s % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}

fn main() -> anyhow::Result<()> {
    // The ? operator is not used because we don't want to return an error if a `.env` file is not found
    // we could possibly read from the regular environment variables instead
    let _ = dotenv::dotenv();

    let user_config = get_user_config()?;

    // TODO document lowercasing
    let channel = get_env_var!("CHANNEL_NAME")?.to_ascii_lowercase();

    let authorized_users: HashSet<Box<str>> = get_env_var!("AUTHORIZED_USERS")?
        .split(',')
        .map(str::to_ascii_lowercase)
        .map(String::into_boxed_str)
        .collect();

    let mut bot = Bot {
        commands: HashMap::with_capacity(16),
        rng: rand::thread_rng(),
        authorized_users,
    };

    cmd_mpv!(
        bot,
        play,
        r#"{ "command": ["set_property", "pause", false] }"#
    );

    cmd_mpv!(
        bot,
        pause,
        r#"{ "command": ["set_property", "pause", true] }"#
    );

    cmd_offset!(bot, rewind, -);
    cmd_offset!(bot, forward, +);

    let mut pos = |args: Args| {
        let time_pos = match mpv::get_property_as::<f64>("time-pos") {
            Some(time_pos) => time_pos,
            _ => return,
        };

        let duration = match mpv::get_property_as::<f64>("duration") {
            Some(duration) => duration,
            _ => return,
        };

        let _ = args.writer.say(
            args.msg,
            format!(
                "{} / {} ({} remaining)",
                format_time(time_pos as u64),
                format_time(duration as u64),
                format_time((duration - time_pos) as u64),
            )
            .as_str(),
        );
    };
    cmd!(bot, pos);

    let mut sub = |args: Args| {
        if let CmdArgs::U16(track) = args.args {
            let _ = mpv::send_command(
                format!("{{ \"command\": [\"set_property\", \"sid\", {}] }}", track).as_str(),
            );
        }
    };
    cmd!(bot, sub);

    let mut aud = |args: Args| {
        if let CmdArgs::U16(track) = args.args {
            let _ = mpv::send_command(
                format!("{{ \"command\": [\"set_property\", \"aid\", {}] }}", track).as_str(),
            );
        }
    };
    cmd!(bot, aud);

    let mut vol = |args: Args| {
        if let CmdArgs::U16(volume) = args.args {
            let _ = args.writer.say(args.msg, {
                match mpv::send_command(
                    format!(
                        "{{ \"command\": [\"set_property\", \"volume\", {}] }}",
                        volume
                    )
                    .as_str(),
                ) {
                    Ok(_) => format!("(volume has been set to {}%)", volume),
                    Err(e) => format!("failed to set volume: {}", e),
                }
                .as_str()
            });
        } else {
            let s;
            let _ = args.writer.say(args.msg, {
                if let Some(x) = mpv::get_property_as::<f64>("volume") {
                    s = format!("volume: {}%", x);
                    s.as_str()
                } else {
                    "(failed to get volume information)"
                }
            });
        }
    };
    cmd!(bot, vol);

    let mut joke = |args: Args| {
        if let CmdArgs::U16(index) = args.args {
            let _ = args.writer.say(args.msg, jokes::JOKES[index as usize]);
        }
    };
    cmd!(bot, joke);

    // run the bot in the executor
    smol::block_on(async move { bot.run(&user_config, &channel).await })
}

/// All possible argument types for the commands.
enum CmdArgs {
    U16(u16),
    None,
}

/// Struct passed when doing dynamic dispatch for commands
pub struct Args<'a, 'b> {
    msg: &'a Privmsg<'b>,
    args: CmdArgs,
    writer: &'a mut twitchchat::Writer,
}

/// Trait for providing dynamic dispatch to bot functionality
pub trait Command: Send + Sync {
    fn handle(&mut self, args: Args<'_, '_>);
}

impl<F> Command for F
where
    F: Fn(Args<'_, '_>) + Send + Sync,
{
    fn handle(&mut self, args: Args<'_, '_>) {
        (self)(args)
    }
}

/// Internal state needed for certain functionality
#[derive(Default)]
struct Bot<'a, R: Rng> {
    commands: HashMap<&'a str, &'a mut dyn Command>,
    authorized_users: HashSet<Box<str>>,
    rng: R,
}

impl<'a, R: Rng> Bot<'a, R> {
    fn add_command(&mut self, name: &'a str, cmd: &'a mut dyn Command) {
        self.commands.insert(name, cmd);
    }

    async fn run(&mut self, user_config: &UserConfig, channel: &str) -> anyhow::Result<()> {
        // this can fail if DNS resolution cannot happen
        let connector = twitchchat::connector::smol::Connector::twitch()?;

        let mut runner = AsyncRunner::connect(connector, user_config).await?;

        println!("Joining {}...", channel);
        if let Err(err) = runner.join(channel).await {
            eprintln!("[ERROR] Failed to join '{}': {}", channel, err);
        } else {
            println!("[INFO] Successfully joined {}", channel);
        }

        self.main_loop(&mut runner).await
    }

    async fn main_loop(&mut self, runner: &mut AsyncRunner) -> anyhow::Result<()> {
        // cloneable rate-limited writer
        let mut writer = runner.writer();

        loop {
            match runner.next_message().await? {
                Status::Message(Commands::Privmsg(pm)) => {
                    unsafe {
                        // optimize out check for len == 0, since twitch usernames/chats cannot be empty
                        if pm.name().is_empty() {
                            std::hint::unreachable_unchecked();
                        }
                        if pm.data().is_empty() {
                            std::hint::unreachable_unchecked();
                        }
                    }

                    if self.authorized_users.contains(pm.name()) {
                        let mut command_iter = pm.data().split_ascii_whitespace();

                        if let Some(command) = command_iter.next() {
                            if !command.starts_with('!') {
                                continue;
                            }

                            if let Some(dyn_command) = self.commands.get_mut(command) {
                                let args = match command {
                                    "!rewind" | "!forward" => CmdArgs::U16(
                                        match command_iter.next() {
                                            Some(x) => x.parse::<u16>().ok(),
                                            None => None,
                                        }
                                        .unwrap_or(10),
                                    ),
                                    "!sub" | "!aud" => CmdArgs::U16(
                                        match command_iter.next() {
                                            Some(x) => x.parse::<u16>().ok(),
                                            None => None,
                                        }
                                        // 0 is always the value for no subs
                                        .unwrap_or(0),
                                    ),
                                    "!joke" => CmdArgs::U16(
                                        self.rng.gen_range(0, jokes::JOKES.len() as u16),
                                    ),
                                    "!vol" => match command_iter.next() {
                                        Some(x) => match x.parse::<u16>() {
                                            Ok(x) => CmdArgs::U16(x),
                                            _ => CmdArgs::None,
                                        },
                                        None => CmdArgs::None,
                                    },
                                    _ => CmdArgs::None,
                                };

                                dyn_command.handle(Args {
                                    msg: &pm,
                                    args,
                                    writer: &mut writer,
                                });
                            }
                        }
                    }
                }
                Status::Quit | Status::Eof => break,
                _ => continue,
            }
        }

        Ok(())
    }
}
