// for simplify reason, use json for client/server communication potocol
extern crate serde;

use std::backtrace::Backtrace;

use crate::KvsError;
use serde::{Deserialize, Serialize};

use super::error::Result;

#[derive(Debug, Serialize, Deserialize)]
pub enum Command {
    Get(String),
    Set(String, String),
    Remove(String),
}

pub struct CommandResult(pub Result<Command>);

impl From<(String, String, Option<String>)> for CommandResult {
    fn from((command_type, key, value): (String, String, Option<String>)) -> Self {
        let command: Command;
        match command_type.as_str() {
            "get" => {
                if let Some(value) = value {
                    return CommandResult(Err(KvsError::UnexpectedCommand {
                        command: format!("{:?} {:?} {:?}", command_type, key, value),
                        backtrace: Backtrace::force_capture(),
                    }));
                }
                command = Command::Get(key);
            }
            "set" => {
                if value.is_none() {
                    return CommandResult(Err(KvsError::UnexpectedCommand {
                        command: format!("{:?}", value),
                        backtrace: Backtrace::force_capture(),
                    }));
                }
                command = Command::Set(key, value.unwrap());
            }
            "rm" => {
                if let Some(value) = value {
                    return CommandResult(Err(KvsError::UnexpectedCommand {
                        command: format!("{:?}", value),
                        backtrace: Backtrace::force_capture(),
                    }));
                }
                command = Command::Remove(key);
            }
            _ => {
                return CommandResult(Err(KvsError::UnexpectedCommand {
                    command: format!("{:?} {:?} {:?}", command_type, key, value),
                    backtrace: Backtrace::force_capture(),
                }));
            }
        }
        CommandResult(Ok(command))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Response {
    Success(Option<String>),
    Error(String),
}
