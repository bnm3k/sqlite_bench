#![allow(dead_code, unreachable_code)]
use anyhow::format_err;
use clap::*;
use log::info;
use rusqlite::Connection;
use std::thread;
use std::time::Instant;
use thousands::Separable;

#[derive(Debug)]
struct User {
    id: uuid::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    username: String,
}

impl User {
    fn gen() -> Self {
        Self {
            id: uuid::Uuid::new_v4(),
            created_at: chrono::Utc::now(),
            username: String::from("Someone"),
        }
    }
}

fn init_db(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    // only set if not 'memory'
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.execute(
        "CREATE TABLE users(
            id BLOB PRIMARY KEY NOT NULL,
            created_at TEXT NOT NULL,
            username TEXT NOT NULL
        )",
        (),
    )?;
    conn.execute("CREATE UNIQUE INDEX idx_users_on_id ON users(id)", ())?;
    Ok(())
}

fn run_reads(conn: &rusqlite::Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("SELECT count(*) from users")?;
    let count: u64 = stmt.query_row([], |row| row.get(0))?;
    println!("num rows for `users` table: {}", count);
    Ok(())
}

/// Sqlite inserts benchmarking based on
/// kerkour.com/high-performance-rust-with-sqlite
#[derive(Parser, Debug)]
#[command(about, long_about = None)]
struct Args {
    /// Number of threads to spawn for concurrent inserts
    #[arg(short = 'c', long = "concurrency", default_value_t = 1)]
    num_threads: u64,

    /// Number of inserts per thread
    #[arg(short, long, default_value_t = 1)]
    num_inserts_per_thread: u64,
}

fn main() -> anyhow::Result<()> {
    pretty_env_logger::init();

    // get CLI args
    let Args {
        num_inserts_per_thread,
        num_threads,
    } = Args::parse();
    let num_inserts = num_inserts_per_thread * num_threads;
    info!(
        "inserts: {}, concurrency: {}",
        num_inserts.separate_with_commas(),
        num_threads
    );

    // common
    let db_path = "db.sqlite";
    let get_conn = move || Connection::open(&db_path);

    // delete db if it already exists
    if let Err(e) = std::fs::remove_file(&db_path) {
        if e.kind() != std::io::ErrorKind::NotFound {
            return Err(format_err!("{}: {}", e, db_path));
        }
    }

    // create table
    {
        let conn = get_conn()?;
        init_db(&conn).unwrap();
    }

    // start timing
    let start = Instant::now();
    let num_threads = 1; // focus on single-threaded for now

    // run concurrent inserts
    let mut handles = Vec::with_capacity(num_threads as usize);
    for i in 1..=num_threads {
        let handle = thread::spawn(move || -> anyhow::Result<()> {
            let thread_id = format!("[thread {}]", i);
            info!("{thread_id} start");
            let conn = get_conn().unwrap();
            conn.pragma_update(None, "synchronous", "NORMAL")?;
            for _ in 0..num_inserts_per_thread {
                let u = User::gen();
                conn.execute(
                    "INSERT INTO users(id, created_at, username) VALUES (?, ?, ?)",
                    (&u.id.to_string(), &u.created_at.to_rfc3339(), &u.username),
                )
                .unwrap();
            }
            info!("{thread_id} complete");
            Ok(())
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap()?;
    }

    // get duration
    let duration = start.elapsed();
    let inserts_per_sec = num_inserts as f64 / duration.as_secs_f64();
    println!(
        "Benchmark: insert {} records ({}/{}): {:?} ({} inserts/s)",
        num_inserts.separate_with_commas(),
        num_inserts_per_thread.separate_with_commas(),
        num_threads,
        duration,
        inserts_per_sec.round().separate_with_commas()
    );

    // get number of inserts
    {
        let conn = get_conn()?;
        run_reads(&conn)?;
    }
    Ok(())
}
