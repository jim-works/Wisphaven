pub const CREATE_CHUNK_TABLE: &str = "
        CREATE TABLE IF NOT EXISTS data (
            tid INTEGER NOT NULL,
            x INTEGER NOT NULL,
            y INTEGER NOT NULL,
            z INTEGER NOT NULL,
            data BLOB NOT NULL,
            PRIMARY KEY (tid,x,y,z)
        ) STRICT";
pub const CREATE_WORLD_INFO_TABLE: &str = "
        CREATE TABLE IF NOT EXISTS world_info (
            key TEXT NOT NULL,
            value BLOB NOT NULL,
            PRIMARY KEY (key)
        ) STRICT";
pub const SAVE_CHUNK_DATA: &str = "
            INSERT OR REPLACE INTO data (tid,x, y, z, data)
            VALUES (?1,?2,?3,?4,?5)";
pub const DELETE_CHUNK_DATA: &str = "
            DELETE FROM data
            WHERE tid = ?1 AND x = ?2 AND y = ?3 AND z = ?4";
pub const LOAD_CHUNK_DATA: &str = "
            SELECT data FROM data
            WHERE tid = ?1 AND x = ?2 AND y = ?3 AND z = ?4";
pub const INSERT_WORLD_INFO: &str = "
            INSERT OR REPLACE INTO world_info (key,value)
            VALUES (?1,?2)";
pub const LOAD_WORLD_INFO: &str = "
            SELECT value FROM world_info
            WHERE key = ?1";