use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

use crate::error::AppError;
use crate::types::{ActivityEvent, Alert, HistoryPoint};

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(app_dir: &Path) -> Result<Self, AppError> {
        std::fs::create_dir_all(app_dir)?;
        let conn = Connection::open(app_dir.join("history.db"))?;
        conn.execute_batch(
            r#"
            PRAGMA journal_mode = WAL;
            PRAGMA synchronous = NORMAL;
            CREATE TABLE IF NOT EXISTS samples (
                ts INTEGER PRIMARY KEY,
                cpu REAL NOT NULL,
                gpu REAL NOT NULL,
                memory REAL NOT NULL,
                disk REAL NOT NULL,
                net_down REAL NOT NULL,
                net_up REAL NOT NULL
            );
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY,
                ts INTEGER NOT NULL,
                kind TEXT NOT NULL,
                severity TEXT NOT NULL,
                category TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                why TEXT,
                impact TEXT,
                recommendation TEXT,
                process TEXT
            );
            CREATE TABLE IF NOT EXISTS alerts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ts INTEGER NOT NULL,
                severity TEXT NOT NULL,
                category TEXT NOT NULL,
                title TEXT NOT NULL,
                body TEXT NOT NULL,
                acknowledged INTEGER NOT NULL DEFAULT 0,
                fingerprint TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_samples_ts ON samples(ts);
            CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts);
            CREATE INDEX IF NOT EXISTS idx_alerts_ts ON alerts(ts);
            "#,
        )?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert_sample(&self, point: &HistoryPoint) -> Result<(), AppError> {
        let conn = self.conn.lock().expect("db");
        conn.execute(
            "INSERT OR REPLACE INTO samples (ts, cpu, gpu, memory, disk, net_down, net_up)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                point.timestamp,
                point.cpu,
                point.gpu,
                point.memory,
                point.disk,
                point.network_down,
                point.network_up
            ],
        )?;
        Ok(())
    }

    pub fn insert_event(&self, event: &ActivityEvent) -> Result<(), AppError> {
        let conn = self.conn.lock().expect("db");
        conn.execute(
            "INSERT OR IGNORE INTO events
             (id, ts, kind, severity, category, title, body, why, impact, recommendation, process)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                event.id,
                event.timestamp,
                event.kind,
                event.severity,
                event.category,
                event.title,
                event.body,
                event.why,
                event.impact,
                event.recommendation,
                event.process
            ],
        )?;
        Ok(())
    }

    pub fn insert_alert_if_new(
        &self,
        fingerprint: &str,
        ts: i64,
        severity: &str,
        category: &str,
        title: &str,
        body: &str,
        cooldown_sec: i64,
    ) -> Result<Option<Alert>, AppError> {
        let conn = self.conn.lock().expect("db");
        let open: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM alerts WHERE fingerprint = ?1 AND acknowledged = 0",
                params![fingerprint],
                |row| row.get(0),
            )
            .unwrap_or(0);
        if open > 0 {
            return Ok(None);
        }
        let recent: Option<i64> = conn
            .query_row(
                "SELECT ts FROM alerts WHERE fingerprint = ?1 ORDER BY ts DESC LIMIT 1",
                params![fingerprint],
                |row| row.get(0),
            )
            .ok();
        if let Some(prev) = recent {
            if ts - prev < cooldown_sec {
                return Ok(None);
            }
        }
        conn.execute(
            "INSERT INTO alerts (ts, severity, category, title, body, acknowledged, fingerprint)
             VALUES (?1, ?2, ?3, ?4, ?5, 0, ?6)",
            params![ts, severity, category, title, body, fingerprint],
        )?;
        let id = conn.last_insert_rowid();
        Ok(Some(Alert {
            id,
            timestamp: ts,
            title: title.to_string(),
            body: body.to_string(),
            severity: severity.to_string(),
            category: category.to_string(),
            acknowledged: false,
        }))
    }

    pub fn recent_alerts(&self, limit: usize) -> Result<Vec<Alert>, AppError> {
        let conn = self.conn.lock().expect("db");
        let mut stmt = conn.prepare(
            "SELECT id, ts, severity, category, title, body, acknowledged
             FROM alerts ORDER BY ts DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map(params![limit as i64], |row| {
            Ok(Alert {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                severity: row.get(2)?,
                category: row.get(3)?,
                title: row.get(4)?,
                body: row.get(5)?,
                acknowledged: row.get::<_, i64>(6)? != 0,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn acknowledge(&self, id: i64) -> Result<(), AppError> {
        let conn = self.conn.lock().expect("db");
        conn.execute(
            "UPDATE alerts SET acknowledged = 1 WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn acknowledge_all(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().expect("db");
        conn.execute("UPDATE alerts SET acknowledged = 1 WHERE acknowledged = 0", [])?;
        Ok(())
    }

    pub fn history_since(&self, from_ts: i64) -> Result<Vec<HistoryPoint>, AppError> {
        let conn = self.conn.lock().expect("db");
        let mut stmt = conn.prepare(
            "SELECT ts, cpu, gpu, memory, disk, net_down, net_up FROM samples
             WHERE ts >= ?1 ORDER BY ts ASC",
        )?;
        let rows = stmt.query_map(params![from_ts], |row| {
            Ok(HistoryPoint {
                timestamp: row.get(0)?,
                cpu: row.get(1)?,
                gpu: row.get(2)?,
                memory: row.get(3)?,
                disk: row.get(4)?,
                network_down: row.get(5)?,
                network_up: row.get(6)?,
            })
        })?;
        let points: Vec<HistoryPoint> = rows.filter_map(|r| r.ok()).collect();
        Ok(downsample_history(points, 720))
    }

    pub fn events_since(&self, from_ts: i64) -> Result<Vec<ActivityEvent>, AppError> {
        let conn = self.conn.lock().expect("db");
        let mut stmt = conn.prepare(
            "SELECT id, ts, kind, severity, category, title, body, why, impact, recommendation, process
             FROM events WHERE ts >= ?1 ORDER BY ts DESC LIMIT 400",
        )?;
        let rows = stmt.query_map(params![from_ts], |row| {
            Ok(ActivityEvent {
                id: row.get(0)?,
                timestamp: row.get(1)?,
                kind: row.get(2)?,
                severity: row.get(3)?,
                category: row.get(4)?,
                title: row.get(5)?,
                body: row.get(6)?,
                why: row.get(7)?,
                impact: row.get(8)?,
                recommendation: row.get(9)?,
                process: row.get(10)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn prune(&self, retention_days: u32) -> Result<(), AppError> {
        let cutoff = chrono::Utc::now().timestamp() - (retention_days as i64 * 86400);
        let conn = self.conn.lock().expect("db");
        conn.execute("DELETE FROM samples WHERE ts < ?1", params![cutoff])?;
        conn.execute("DELETE FROM events WHERE ts < ?1", params![cutoff])?;
        conn.execute(
            "DELETE FROM alerts WHERE ts < ?1 AND acknowledged = 1",
            params![cutoff],
        )?;
        Ok(())
    }
}

fn downsample_history(points: Vec<HistoryPoint>, max_points: usize) -> Vec<HistoryPoint> {
    if points.len() <= max_points {
        return points;
    }
    let stride = points.len() as f64 / max_points as f64;
    let mut out = Vec::with_capacity(max_points);
    let mut i = 0.0;
    while out.len() < max_points && (i as usize) < points.len() {
        out.push(points[i as usize].clone());
        i += stride;
    }
    if let Some(last) = points.last() {
        if out.last().map(|p| p.timestamp) != Some(last.timestamp) {
            out.push(last.clone());
        }
    }
    out
}
