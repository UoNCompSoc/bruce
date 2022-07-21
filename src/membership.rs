use fallible_iterator::FallibleIterator;
use rusqlite::{params, Connection};

#[derive(Debug)]
pub struct Membership {
    pub student_id: u32,
    pub name: String,
    pub discord_id: Option<u64>,
    pub should_drop: bool,
}

impl Membership {
    pub fn init_table(conn: &Connection) {
        conn.execute("CREATE TABLE IF NOT EXISTS memberships (student_id INT NOT NULL PRIMARY KEY, name VARCHAR NOT NULL, discord_id BIGINT, should_drop BIT NOT NULL)", params![]).expect("create memberships table");
    }

    pub fn get_by_student_id(conn: &Connection, student_id: u32) -> Option<Self> {
        let mut stmt = conn
            .prepare("SELECT name, discord_id, should_drop FROM memberships WHERE student_id = ?1")
            .expect("get membership by student id");
        stmt.query_row(params![student_id], |r| {
            Ok(Self {
                student_id,
                name: r.get(0)?,
                discord_id: r.get(1)?,
                should_drop: r.get(2)?,
            })
        })
        .ok()
    }

    pub fn get_by_discord_id(conn: &Connection, discord_id: u64) -> Option<Self> {
        let mut stmt = conn
            .prepare("SELECT student_id, name, should_drop FROM memberships WHERE discord_id = ?1")
            .expect("get membership by discord id");
        stmt.query_row(params![discord_id], |r| {
            Ok(Self {
                student_id: r.get(0)?,
                name: r.get(1)?,
                discord_id: Some(discord_id),
                should_drop: r.get(2)?,
            })
        })
        .ok()
    }

    pub fn get_all(conn: &Connection) -> Vec<Self> {
        let mut stmt = conn
            .prepare("SELECT student_id, name, discord_id, should_drop FROM memberships")
            .expect("get all memberships statement generation");
        stmt.query(params![])
            .expect("get all memberships")
            .map(|r| {
                Ok(Self {
                    student_id: r.get(0)?,
                    name: r.get(1)?,
                    discord_id: r.get(2)?,
                    should_drop: r.get(3)?,
                })
            })
            .collect()
            .expect("get all memberships")
    }

    pub fn update_disord_id(&mut self, conn: &Connection, discord_id: Option<u64>) {
        conn.execute(
            "UPDATE memberships SET discord_id = ?1 WHERE student_id = ?2",
            params![discord_id, self.student_id],
        )
        .expect("updating membership discord id");
        self.discord_id = discord_id;
    }

    pub fn update_should_drop(&mut self, conn: &Connection, should_drop: bool) {
        conn.execute(
            "UPDATE memberships SET should_drop = ?1 WHERE student_id = ?2",
            params![should_drop, self.student_id],
        )
        .expect("updating should drop");
        self.should_drop = should_drop;
    }

    pub fn insert(&self, conn: &Connection) {
        conn.execute(
            "INSERT OR IGNORE INTO memberships (student_id, name, should_drop) VALUES (?1, ?2, 0)",
            params![self.student_id, self.name],
        )
        .expect("inserting membership");
    }

    pub fn delete(self, conn: &Connection) {
        conn.execute(
            "DELETE FROM memberships WHERE student_id = ?1",
            params![self.student_id],
        )
        .expect("deleting membershipSa");
    }
}
