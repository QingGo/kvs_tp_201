extern crate anyhow;
extern crate bson;
extern crate rand;
extern crate ron;
extern crate serde;

use ron::ser::to_writer;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::io::prelude::*;
use std::io::{Read, Write};
use std::str::from_utf8;

#[derive(Debug, Serialize, Deserialize)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Serialize, Deserialize)]
struct Move {
    direction: Direction,
    steps: u32,
}

impl Move {
    fn random_move() -> Move {
        let direction = match rand::random::<u8>() % 4 {
            0 => Direction::Up,
            1 => Direction::Down,
            2 => Direction::Left,
            3 => Direction::Right,
            _ => panic!("invalid direction"),
        };
        let steps = rand::random::<u32>() % 10;
        Move { direction, steps }
    }
}

// wraper type of Vec<u8>
pub struct VecByte(Vec<u8>);
impl Read for VecByte {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let mut len = 0;
        if !self.0.is_empty() {
            len = self.0.len().min(buf.len());
            buf[..len].copy_from_slice(&self.0[..len]);
            self.0.drain(..len);
        }
        Ok(len)
    }
}
impl Write for VecByte {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn main() -> Result<(), anyhow::Error> {
    let a = Move {
        direction: Direction::Up,
        steps: 10,
    };
    println!("{:?}", a);

    // why can not use same file in writer and reader?
    // writer serialized json to file
    let mut file = fs::OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .read(true)
        .open("serde_result.json")?;
    {
        let mut writer = io::BufWriter::new(&file);
        let serialized = serde_json::to_string(&a)?;
        writer.write_all(serialized.as_bytes())?;
        writer.flush()?;
    }

    // read and deserialize json from file
    // without seek 0, which make us read from the beginning of file, reader will read nothing
    file.seek(io::SeekFrom::Start(0))?;
    let mut reader = io::BufReader::new(
        fs::OpenOptions::new()
            .read(true)
            .open("serde_result.json")?,
    );
    let mut contents = String::new();
    reader.read_to_string(&mut contents)?;
    let b: Move = serde_json::from_str(&contents)?;
    println!("{:?}", b);

    // serialize to a Vec<u8> buffer with RON format
    let mut buf = Vec::new();
    to_writer(&mut buf, &a)?;
    println!("{}", from_utf8(&buf)?);

    // Serialize and deserialize 1000 data structures with serde (BSON).
    let mut file = fs::OpenOptions::new()
        .truncate(true)
        .write(true)
        .create(true)
        .read(true)
        .open("random_moves.bson")?;
    for _ in 0..1000 {
        let random_move = Move::random_move();
        let random_move_bson = bson::to_document(&random_move)?;
        random_move_bson.to_writer(&file)?;
    }

    file.seek(io::SeekFrom::Start(0))?;
    for _ in 0..1000 {
        let random_move_bson: Move = bson::from_reader(&file)?;
        println!("{:?}", random_move_bson);
    }
    // try it again with a Vec<u8>
    let mut buf = VecByte(Vec::new());
    for _ in 0..1000 {
        let random_move = Move::random_move();
        let random_move_bson = bson::to_document(&random_move)?;
        random_move_bson.to_writer(&mut buf)?;
    }
    for _ in 0..1000 {
        let random_move_bson: Move = bson::from_reader(&mut buf)?;
        println!("{:?}", random_move_bson);
    }
    Ok(())
}
