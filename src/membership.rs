use rusqlite::{params, Connection};

pub struct Membership {
    pub student_id: u32,
    pub name: String,
    pub discord_id: Option<u64>,
}

impl Membership {
    pub fn get_by_student_id(conn: &Connection, student_id: u32) -> Option<Self> {
        let mut stmt = conn
            .prepare("SELECT name, discord_id FROM members WHERE student_id = ?1")
            .unwrap();
        stmt.query_row(params![student_id], |r| {
            Ok(Self {
                student_id,
                name: r.get(0).unwrap(),
                discord_id: r.get(1).unwrap(),
            })
        })
        .ok()
    }
    pub fn get_by_discord_id(conn: &Connection, discord_id: u64) -> Option<Self> {
        let mut stmt = conn
            .prepare("SELECT student_id, name FROM members WHERE discord_id = ?1")
            .unwrap();
        stmt.query_row(params![discord_id], |r| {
            Ok(Self {
                student_id: r.get(0).unwrap(),
                name: r.get(1).unwrap(),
                discord_id: Some(discord_id),
            })
        })
            .ok()
    }

    pub fn update(&mut self, conn: &Connection, discord_id: Option<u64>) {
        conn.execute(
            "UPDATE members SET discord_id = ?1 WHERE student_id = ?2",
            params![discord_id, self.student_id],
        )
        .unwrap();
        self.discord_id = discord_id;
    }

    pub fn insert(conn: &Connection, membership: &Self) {
        conn.execute(
            "INSERT OR IGNORE INTO members (student_id, student_name) VALUES (?1, ?2)",
            params![membership.student_id, membership.name],
        )
        .unwrap();
    }
}
