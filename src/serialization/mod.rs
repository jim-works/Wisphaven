use std::{mem::size_of, panic::catch_unwind, path::Path};

use bevy::{prelude::*, tasks::Task};
use futures_lite::future;
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::*;

use crate::world::{
    chunk::{ArrayChunk, ChunkCoord},
    BlockType, LevelLoadState, LevelSystemSet,
};

use self::queries::SAVE_CHUNK_DATA;

pub struct SerializationPlugin;

pub mod queries;
mod save;
mod setup;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup::on_level_created.in_set(OnUpdate(LevelLoadState::NotLoaded)))
            .add_systems((save::do_saving, save::save_all).in_set(LevelSystemSet::Main))
            .add_event::<SaveChunkEvent>()
            .insert_resource(SaveTimer(Timer::from_seconds(5.0, TimerMode::Repeating)));
    }
}

#[derive(Component)]
pub struct NeedsSaving;

#[derive(Resource)]
pub struct SaveTimer(Timer);

pub struct SaveChunkEvent(ChunkCoord);

//run length encoded format for chunks
//TODO: figure out how to do entities
pub struct ChunkSaveFormat {
    pub position: ChunkCoord,
    pub data: Vec<(BlockType, u16)>,
}

#[derive(Debug)]
pub enum ChunkSerializationError {
    InvalidFormat,
}

impl std::fmt::Display for ChunkSerializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChunkSerializationError::InvalidFormat => write!(f, "Invalid chunk format"),
        }
    }
}

impl std::error::Error for ChunkSerializationError {}

impl From<&ArrayChunk> for ChunkSaveFormat {
    fn from(value: &ArrayChunk) -> Self {
        let mut data = Vec::new();
        let mut run = 1;
        let mut curr_block_opt = None;
        for block in value.blocks.into_iter() {
            match curr_block_opt {
                None => curr_block_opt = Some(block),
                Some(curr_block) => {
                    if curr_block == block {
                        run += 1;
                    } else {
                        data.push((curr_block, run));
                        curr_block_opt = Some(block);
                        run = 1;
                    }
                }
            }
        }
        Self {
            position: value.position,
            data,
        }
    }
}

impl TryFrom<&[u8]> for ChunkSaveFormat {
    type Error = ChunkSerializationError;

    fn try_from(value: &[u8]) -> Result<Self, Self::Error> {
        if let Ok(position) = bincode::deserialize(value) {
            let mut result = Self {
                position,
                data: Vec::new(),
            };
            let mut idx = size_of::<ChunkCoord>();
            let caught = catch_unwind(move || {
                while idx < value.len() {
                    let length = u16::from_le_bytes([value[idx], value[idx + 1]]);
                    idx += 2;
                    let block_type = value[idx];
                    idx += 1;
                    match block_type {
                        0 => result.data.push((BlockType::Empty, length)),
                        1 => {
                            let id = u32::from_le_bytes([
                                value[idx],
                                value[idx + 1],
                                value[idx + 2],
                                value[idx + 3],
                            ]);
                            idx += 4;
                            result.data.push((BlockType::Basic(id), length));
                        }
                        _ => return Err(ChunkSerializationError::InvalidFormat),
                    }
                }
                return Ok(result);
            });
            return match caught {
                Ok(result) => result,
                Err(_) => Err(ChunkSerializationError::InvalidFormat),
            };
        }
        Err(ChunkSerializationError::InvalidFormat)
    }
}

impl ChunkSaveFormat {
    pub fn into_chunk(self, chunk_entity: Entity) -> ArrayChunk {
        let mut curr_idx = 0;
        let mut chunk = ArrayChunk::new(self.position, chunk_entity);
        for (block, length) in self.data.into_iter() {
            for idx in curr_idx..curr_idx + length as usize {
                chunk.blocks[idx] = block;
            }
            curr_idx += length as usize;
        }
        chunk
    }
    //position, [(run length (u16), blocktype (u8 then varies))]
    pub fn into_bits(self) -> Vec<u8> {
        let mut bits = Vec::new();
        bits.extend(bincode::serialize(&self.position).unwrap());
        for (block, length) in self.data.into_iter() {
            bits.extend(length.to_le_bytes());
            match block {
                BlockType::Empty => bits.push(0),
                BlockType::Basic(id) => {
                    bits.push(1);
                    bits.extend(id.to_le_bytes())
                }
                BlockType::Entity(_) => todo!(),
            }
        }
        bits
    }
}

#[derive(Resource)]
pub struct LevelDB {
    pool: Pool<SqliteConnectionManager>,
    current_task: Option<Task<Option<LevelDBErr>>>
}

#[derive(Copy, Clone)]
pub enum ChunkTable {
    Terrain = 0,
}

#[derive(Debug)]
pub enum LevelDBErr {
    BUSY,
    R2D2(r2d2::Error),
    Sqlite(rusqlite::Error),
}

impl LevelDB {
    pub fn new(path: &Path) -> Result<LevelDB, r2d2::Error> {
        let manager = SqliteConnectionManager::file(path);
        let pool = Pool::new(manager)?;
        Ok(Self { pool, current_task: None })
    }
    pub fn is_busy(&self) -> bool {
        if let Some(task) = self.current_task {
            future::block_on(future::poll_once(&mut task))
        }
    }
    pub fn create_chunk_table(&mut self) -> Option<LevelDBErr> {
        match self.pool.get() {
            Ok(conn) => conn
                .execute(queries::CREATE_CHUNK_TABLE, [])
                .map_err(|e| LevelDBErr::Sqlite(e))
                .err(),
            Err(e) => Some(LevelDBErr::R2D2(e)),
        }
    }
    pub fn save_chunk_data(
        &mut self,
        tid: ChunkTable,
        data: Vec<(ChunkCoord, Vec<u8>)>,
    ) -> Option<LevelDBErr> {

        match self.pool.get() {
            Ok(conn) => {
                match conn.prepare(SAVE_CHUNK_DATA) {
                    Ok(mut stmt) => {
                        for (coord, blob) in data {
                            if let Err(e) = stmt.execute(params![tid as i32, coord.x, coord.y, coord.z, blob]) {
                                return Some(LevelDBErr::Sqlite(e));
                            }
                        }
                        None
                    },
                    Err(e) => Some(LevelDBErr::Sqlite(e))
                }
            },
            Err(e) => Some(LevelDBErr::R2D2(e))
        }
    }
}
