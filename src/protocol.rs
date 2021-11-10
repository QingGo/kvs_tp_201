use anyhow::Result;
use serde::{de, ser, Deserialize, Serialize};
use std::str;

#[derive(Debug, PartialEq, Clone)]
pub enum RESP {
    SimpleString(String),
    Error(String),
    BulkString(String),
    Array(Vec<RESP>),
}

impl RESP {
    fn serialize(&self) -> String {
        match self {
            RESP::SimpleString(s) => format!("+{}\r\n", s),
            RESP::Error(s) => format!("-{}\r\n", s),
            RESP::BulkString(s) => format!("${}\r\n{}\r\n", s.len(), s),
            RESP::Array(v) => {
                let mut s = String::new();
                s.push_str(format!("*{}\r\n", v.len()).as_str());
                for c in v {
                    s.push_str(&c.serialize());
                }
                s
            }
        }
    }

    // implenent parser from redis command
    fn deserialize(s: &str) -> Result<RESP> {
        let mut iter = s.split_whitespace();
        RESP::_deserialize(&mut iter)
    }

    fn _deserialize(iter: &mut str::SplitWhitespace) -> Result<RESP> {
        let cmd = iter.next().unwrap();
        match cmd.chars().next() {
            Some('+') => Ok(RESP::SimpleString(cmd[1..].to_string())),
            Some('-') => Ok(RESP::Error(cmd[1..].to_string())),
            Some('$') => {
                let len = cmd[1..].parse::<usize>()?;
                let s = iter
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("invalid command"))?
                    .to_string();
                if s.len() != len {
                    return Err(anyhow::anyhow!("invalid command"));
                }
                Ok(RESP::BulkString(s))
            }
            Some('*') => {
                let len = cmd[1..].parse::<usize>()?;
                let mut v = Vec::new();
                for _ in 0..len {
                    v.push(RESP::_deserialize(iter)?)
                }
                Ok(RESP::Array(v))
            }
            _ => Err(anyhow::anyhow!("invalid command")),
        }
    }
}

impl Serialize for RESP {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ser::Serializer,
    {
        serializer.serialize_str(self.serialize().as_str())
    }
}

impl<'de> Deserialize<'de> for RESP {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        RESP::deserialize(&s).map_err(de::Error::custom)
    }
}
