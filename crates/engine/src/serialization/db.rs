use std::{collections::VecDeque, path::Path};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bincode::ErrorKind;
use futures_lite::future;
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::*;

use super::queries::*;
use crate::world::chunk::*;

#[derive(Resource)]
pub struct LevelDB {
    pool: Pool<SqliteConnectionManager>,
    current_task: Option<Task<Result<LevelDBResult, LevelDBErr>>>,
    //FIFO queues, we always save before loading
    save_queue: VecDeque<Vec<SaveCommand>>,
    load_queue: VecDeque<Vec<LoadCommand>>,
}

pub struct SaveCommand(pub ChunkTable, pub ChunkCoord, pub Vec<u8>);
//will load all entries in to_load for chunk at position, then delete the specified entries
pub struct LoadCommand {
    pub position: ChunkCoord,
    pub to_load: Vec<ChunkTable>,
}

enum LevelDBResult {
    Save(usize),
    Load(Vec<DataFromDBEvent>),
}

#[derive(Event)]
pub struct DataFromDBEvent(pub ChunkCoord, pub Vec<(ChunkTable, Vec<u8>)>);

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum ChunkTable {
    Terrain = 0,
    Buffers = 1,
}

#[derive(Debug)]
pub enum LevelDBErr {
    R2D2(r2d2::Error),
    Sqlite(rusqlite::Error),
    Bincode(Box<ErrorKind>),
    NewWorldVersion,
    InvalidWorldVersion,
}

impl LevelDB {
    pub fn new(path: &Path) -> Result<LevelDB, r2d2::Error> {
        let manager = SqliteConnectionManager::file(path).with_init(|conn| {
            conn.execute_batch(
                "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
            )
        });
        let pool = Pool::new(manager)?;
        Ok(Self {
            pool,
            current_task: None,
            save_queue: VecDeque::new(),
            load_queue: VecDeque::new(),
        })
    }
    pub fn execute_command_sync(
        &mut self,
        f: impl FnOnce(PooledConnection<SqliteConnectionManager>) -> Result<usize, rusqlite::Error>
            + Send,
    ) -> Option<LevelDBErr> {
        match self.pool.get() {
            Ok(conn) => f(conn).map_err(LevelDBErr::Sqlite).err(),
            Err(e) => Some(LevelDBErr::R2D2(e)),
        }
    }
    pub fn execute_query_sync<T>(
        &mut self,
        sql: &str,
        p: impl Params,
        f: impl FnOnce(&Row) -> Result<T>,
    ) -> Result<T, LevelDBErr> {
        match self.pool.get() {
            Ok(conn) => conn.query_row(sql, p, f).map_err(LevelDBErr::Sqlite),
            Err(e) => Err(LevelDBErr::R2D2(e)),
        }
    }
    //adds chunks to the buffer to be saved
    pub fn save_chunk_data(&mut self, data: Vec<SaveCommand>) {
        if !data.is_empty() {
            self.save_queue.push_back(data);
        }
    }
    //adds chunks to the queue to be loaded, will write to DataFromDBEvent when loaded
    pub fn load_chunk_data(&mut self, data: Vec<LoadCommand>) {
        if !data.is_empty() {
            self.load_queue.push_back(data);
        }
    }

    fn flush_saves(&mut self) {
        info!("flush_saves");
        if let Some(task) = &mut self.current_task {
            //finish current task
            let _ = future::block_on(task);
        }
        let mut saved = 0;
        //run all saving tasks before closing
        while let Some(command) = self.save_queue.pop_front() {
            if let Ok(conn) = self.pool.get() {
                saved += command.len();
                if let Err(e) = do_saving(conn, command) {
                    error!("Error saving chunks: {:?}", e);
                }
            }
        }
        info!(
            "Finished saving! Saved {} chunks after last command.",
            saved
        );
    }
}

impl Drop for LevelDB {
    fn drop(&mut self) {
        self.flush_saves();
    }
}

//contacts the db, should be done in a single thread
fn do_saving(
    conn: PooledConnection<SqliteConnectionManager>,
    data: Vec<SaveCommand>,
) -> Result<LevelDBResult, LevelDBErr> {
    match conn.prepare_cached(SAVE_CHUNK_DATA) {
        Ok(mut stmt) => {
            let len = data.len();
            for SaveCommand(tid, coord, blob) in data {
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
    data: Vec<LoadCommand>,
) -> Result<LevelDBResult, LevelDBErr> {
    let mut results = Vec::new();
    match conn.prepare_cached(LOAD_CHUNK_DATA) {
        Ok(mut load_stmt) => {
            for LoadCommand { position, to_load } in data {
                let mut coord_result = Vec::new();
                //loading
                for tid in to_load {
                    let result = load_stmt.query_row(
                        params![tid as i32, position.x, position.y, position.z],
                        |row| row.get(0),
                    );
                    match result {
                        Ok(data) => coord_result.push((tid, data)),
                        Err(rusqlite::Error::QueryReturnedNoRows) => {
                            coord_result.push((tid, Vec::new()))
                        }
                        Err(e) => return Err(LevelDBErr::Sqlite(e)),
                    }
                }
                results.push(DataFromDBEvent(position, coord_result));
            }
            Ok(LevelDBResult::Load(results))
        }
        Err(e) => Err(LevelDBErr::Sqlite(e)),
    }
}

//checks if the db's current_task is finished, and if so, will send an event depending on the task.
//if there is no current task or it's finished, it will start a new task from the db's command queue
pub fn tick_db(mut db: ResMut<LevelDB>, mut load_writer: EventWriter<DataFromDBEvent>) {
    let mut finished = false;
    if let Some(ref mut task) = &mut db.current_task {
        if let Some(data) = future::block_on(future::poll_once(task)) {
            //previous task is done
            //output result
            match data {
                Ok(result) => match result {
                    LevelDBResult::Save(count) => info!("Saved {} chunks.", count),
                    LevelDBResult::Load(events) => {
                        info!("Loaded {} chunks.", events.len());
                        load_writer.send_batch(events);
                    }
                },
                Err(e) => error!("DB Error: {:?}", e),
            }
            finished = true;
            db.current_task = None;
        }
    }
    //start next task if needed
    if finished || db.current_task.is_none() {
        //do saves loads, important for chunk buffers
        if let Some(save_command) = db.save_queue.pop_front() {
            assign_db_work(db.pool.get(), &mut db, move |conn| {
                do_saving(conn, save_command)
            });
        } else if let Some(load_command) = db.load_queue.pop_front() {
            assign_db_work(db.pool.get(), &mut db, move |conn| {
                do_loading(conn, load_command)
            });
        }
    }
}

fn assign_db_work(
    conn_result: Result<PooledConnection<SqliteConnectionManager>, r2d2::Error>,
    db: &mut ResMut<'_, LevelDB>,
    f: impl FnOnce(PooledConnection<SqliteConnectionManager>) -> Result<LevelDBResult, LevelDBErr>
        + Send
        + 'static,
) {
    //work in background
    let pool = AsyncComputeTaskPool::get();
    match conn_result {
        Ok(conn) => db.current_task = Some(pool.spawn(async { f(conn) })),
        Err(e) => error!("Error establishing DB connection: {:?}", e),
    }
}
