//! Module for communicating with mpv through Unix sockets

use std::process::{Command, Stdio};

pub fn send_command(command: &str) -> std::io::Result<String> {
    let mut stdout = String::with_capacity(32);

    let mut echo = Command::new("echo")
        .arg(command)
        .stdout(Stdio::piped())
        .spawn()?;

    if let Some(echo) = echo.stdout.take() {
        let socat = Command::new("socat")
            .args(&["-", "/tmp/mpvsocket"])
            .stdin(echo)
            .stdout(Stdio::piped())
            .spawn()?;

        if let Ok(s) = std::str::from_utf8(socat.wait_with_output()?.stdout.as_slice()) {
            stdout.push_str(s);
        }
    }

    Ok(stdout)
}

pub trait GetJsonAs {
    fn get_as(value: &serde_json::Value) -> Option<Self>
    where
        Self: Sized;
}

macro_rules! impl_get_json_as {
    ($type:ty, $func:ident) => {
        impl GetJsonAs for $type {
            fn get_as(value: &serde_json::Value) -> Option<Self> {
                value.$func()
            }
        }
    };
}

impl_get_json_as!(i64, as_i64);
impl_get_json_as!(u64, as_u64);
impl_get_json_as!(f64, as_f64);
impl_get_json_as!(bool, as_bool);

pub enum MpvError {
    Serde(serde_json::Error),
    Io(std::io::Error),
}

macro_rules! impl_MpvError_from {
    ($error:ty, $variant:ident) => {
        impl From<$error> for MpvError {
            fn from(e: $error) -> Self {
                MpvError::$variant(e)
            }
        }
    };
}

impl_MpvError_from!(serde_json::Error, Serde);
impl_MpvError_from!(std::io::Error, Io);

pub fn get_property(property: &str) -> Result<serde_json::Value, MpvError> {
    serde_json::from_str(
        send_command(format!("{{ \"command\": [\"get_property\", \"{}\"] }}", property).as_str())?
            .as_str(),
    )
    .map_err(MpvError::from)
}

pub fn get_property_as<T: GetJsonAs>(property: &str) -> Option<T> {
    T::get_as(&get_property(property).ok()?)
}
