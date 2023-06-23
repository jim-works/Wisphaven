use std::{collections::VecDeque, mem::size_of, panic::catch_unwind, path::Path};

use bevy::{
    app::AppExit,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use futures_lite::future;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::*;

use crate::world::{
    chunk::{ArrayChunk, ChunkCoord},
    BlockType, LevelLoadState, LevelSystemSet,
};

use self::queries::{READ_CHUNK_DATA, SAVE_CHUNK_DATA};

pub struct SerializationPlugin;

pub mod queries;
mod save;
mod setup;

impl Plugin for SerializationPlugin {
    fn build(&self, app: &mut App) {
        app.add_system(setup::on_level_created.in_set(OnUpdate(LevelLoadState::NotLoaded)))
            .add_systems((save::do_saving, save::save_all, tick_db).in_set(LevelSystemSet::Main))
            .add_system(finish_up.in_base_set(CoreSet::PostUpdate))
            .add_event::<SaveChunkEvent>()
            .add_event::<DataFromDBEvent>()
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
                Ok(result)
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
    current_task: Option<Task<Result<LevelDBResult, LevelDBErr>>>,
    //FIFO queue
    queue: VecDeque<LevelDBCommand>,
}

enum LevelDBCommand {
    Save(Vec<(ChunkTable, ChunkCoord, Vec<u8>)>),
    Load(Vec<(ChunkTable, ChunkCoord)>),
}

enum LevelDBResult {
    Save(usize),
    Load(Vec<DataFromDBEvent>),
}

pub struct DataFromDBEvent(ChunkTable, Vec<u8>);

#[derive(Copy, Clone)]
pub enum ChunkTable {
    Terrain = 0,
}

#[derive(Debug)]
pub enum LevelDBErr {
    R2D2(r2d2::Error),
    Sqlite(rusqlite::Error),
}

impl LevelDB {
    pub fn new(path: &Path) -> Result<LevelDB, r2d2::Error> {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
        ));
        let pool = Pool::new(manager)?;
        Ok(Self {
            pool,
            current_task: None,
            queue: VecDeque::new(),
        })
    }
    pub fn immediate_create_chunk_table(&mut self) -> Option<LevelDBErr> {
        match self.pool.get() {
            Ok(conn) => conn
                .execute(queries::CREATE_CHUNK_TABLE, [])
                .map_err(LevelDBErr::Sqlite)
                .err(),
            Err(e) => Some(LevelDBErr::R2D2(e)),
        }
    }
    //adds chunks to the buffer to be saved
    pub fn save_chunk_data(&mut self, data: Vec<(ChunkTable, ChunkCoord, Vec<u8>)>) {
        if !data.is_empty() {
            self.queue.push_back(LevelDBCommand::Save(data));
        }
    }
    //adds chunks to the queue to be loaded, will write to DataFromDBEvent when loaded
    pub fn load_chunk_data(&mut self, data: Vec<(ChunkTable, ChunkCoord, Vec<u8>)>) {
        if !data.is_empty() {
            self.queue.push_back(LevelDBCommand::Save(data));
        }
    }
}

//contacts the db, should be done in a single thread
fn do_saving(
    conn: PooledConnection<SqliteConnectionManager>,
    data: Vec<(ChunkTable, ChunkCoord, Vec<u8>)>,
) -> Result<LevelDBResult, LevelDBErr> {
    match conn.prepare_cached(SAVE_CHUNK_DATA) {
        Ok(mut stmt) => {
            let len = data.len();
            for (tid, coord, blob) in data {
                if let Err(e) = stmt.execute(params![tid as i32, coord.x, coord.y, coord.z, blob]) {
                    return Err(LevelDBErr::Sqlite(e));
                }
            }
            Ok(LevelDBResult::Save(len))
        }
        Err(e) => Err(LevelDBErr::Sqlite(e)),
    }
}

//contacts the db, should be done in a single thread
fn do_loading(
    conn: PooledConnection<SqliteConnectionManager>,
    data: Vec<(ChunkTable, ChunkCoord)>,
) -> Result<LevelDBResult, LevelDBErr> {
    match conn.prepare_cached(READ_CHUNK_DATA) {
        Ok(mut stmt) => {
            let mut results = Vec::new();
            for (tid, coord) in data {
                let result = stmt
                    .query_map(params![tid as i32, coord.x, coord.y, coord.z], |row| {
                        Ok(DataFromDBEvent(tid, row.get(0)?))
                    });
                match result {
                    Ok(data) => results.extend(data.map(|row| row.unwrap())),
                    Err(e) => return Err(LevelDBErr::Sqlite(e)),
                }
            }
            Ok(LevelDBResult::Load(results))
        }
        Err(e) => Err(LevelDBErr::Sqlite(e)),
    }
}

//checks if the db's current_task is finished, and if so, will send an event depending on the task.
//if there is no current task or it's finished, it will start a new task from the db's command queue
fn tick_db(mut db: ResMut<LevelDB>, mut load_writer: EventWriter<DataFromDBEvent>) {
    let mut finished = false;
    if let Some(ref mut task) = &mut db.current_task {
        if let Some(data) = future::block_on(future::poll_once(task)) {
            //previous task is done
            //output result
            match data {
                Ok(result) => match result {
                    LevelDBResult::Save(count) => info!("Saved {} chunks.", count),
                    LevelDBResult::Load(events) => load_writer.send_batch(events),
                },
                Err(e) => error!("DB Error: {:?}", e),
            }
            finished = true;
            db.current_task = None;
        }
    }
    //start next task if needed
    if finished || db.current_task.is_none() {
        if let Some(command) = db.queue.pop_front() {
            //work in background
            let pool = AsyncComputeTaskPool::get();
            match db.pool.get() {
                Ok(conn) => match command {
                    LevelDBCommand::Save(chunks) => {
                        db.current_task = Some(pool.spawn(async { do_saving(conn, chunks) }))
                    }

                    LevelDBCommand::Load(chunks) => {
                        db.current_task = Some(pool.spawn(async { do_loading(conn, chunks) }))
                    }
                },
                Err(e) => error!("Error establishing DB connection: {:?}", e),
            }
        }
    }
}

//runs all save commands when the app exits
fn finish_up(mut db: ResMut<LevelDB>, reader: EventReader<AppExit>) {
    if reader.is_empty() {
        return;
    }
    if let Some(task) = &mut db.current_task {
        //finish current task
        let _ = future::block_on(task);
    }
    //run all saving tasks before closing
    while let Some(command) = db.queue.pop_front() {
        match command {
            LevelDBCommand::Save(data) => {
                if let Ok(conn) = db.pool.get() {
                    let _ = conn.execute_batch(
                        "PRAGMA journal_mode=WAL;
                         PRAGMA synchronous=NORMAL;",
                    );
                    let _ = do_saving(conn, data);
                }
            }
            LevelDBCommand::Load(_) => {}
        }
    }
    info!("Finished saving!");
}
