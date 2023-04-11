# A couple of SQLite benchmarks

The following are a couple of benchmarks based on SQLite. I'm doing this for the
sake of it, partly to get a hand of SQLite's performance and familiarize myself
with using SQLite from rust, partly to try replicate others' numbers. Therefore
expect a lot of hand-wavey-ness.

## 15K inserts/s with Rust and SQLite

The first benchmark is based on
[Kerkour's](https://kerkour.com/high-performance-rust-with-sqlite) SQLite insert
benchmarking. Here's a quick overview:

We've got the following table:

```sql
CREATE TABLE IF NOT EXISTS users (
    id BLOB PRIMARY KEY NOT NULL,
    created_at TEXT NOT NULL,
    username TEXT NOT NULL
);

CREATE UNIQUE INDEX idx_users_on_id ON users(id);
```

And here's how the data is generated and inserted:

```rust
#[derive(Debug)]
struct User {
    id: uuid::Uuid,
    created_at: chrono::DateTime<chrono::Utc>,
    username: String,
}

let u = User {
    id: uuid::Uuid::new_v4(),
    created_at: chrono::Utc::now(),
    username: String::from("Someone"),
};

conn.execute(
    "INSERT INTO users(id, created_at, username) VALUES (?, ?, ?)",
    (&u.id.to_string(), &u.created_at.to_rfc3339(), &u.username),
)?;
```

My code differs quite a bit from Kerkour, particularly in the following ways:

- usage of the [rusqlite](https://github.com/rusqlite/rusqlite) instead of
  [sqlx](https://github.com/launchbadge/sqlx)
- usage of threads directly instead of tokio (which sqlx uses);

Relying solely on the SQLite and the driver's defaults, I get a paltry 756
inserts per second. Building and running with release mode, I get 764 inserts
per second, so I probably need to tune some knobs here and there.
