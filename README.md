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
inserts per second. This is with one thread. Building and running with release
mode, I get 764 inserts per second, so I probably need to tune some knobs here
and there.

## Concurrent (multi-threaded) Inserts

With 4 threads and 10,000 inserts per thread, I get:
`DatabaseBusy...database is locked` inserts per second (aka some error). My
first guess is that it's probably the `threading mode` configuration. I know
that SQLite does not allow for concurrent write transactions but it should allow
for concurrent connections?

From SQLite's [documentation](https://www.sqlite.org/threadsafe.html), SQLite
supports the following threading modes:

1. Single-thread: all mutexes are disabled, unsafe to use in more than a
   single-thread at once
2. Multi-thread: can be used safely across multiple threads as long as no
   database connection is used simulataneously in two or more threads.
3. Serialized: safe to use by multiple threads with no restriction

The threading mode can be configured at compile-time, application start-time or
when creating a connection. SQLite's default mode is `serialized` which is what
I suspect is causing the `DatabaseBusy` error. However, as per rusqlite's docs,
rusqlite overrides this setting during connection into multi-threaded mode.
Assumption invalidated, so the error is probably at some other level.

My second hunch is that once a connection is used for Insert/Update/Drop, it
acquires a write-lock that it holds throughout the entirety of the connection
rather than per each statement execution. I'll definitely have to dig into
SQLite docs/internals at some point to confirm this but for the time-being, I'll
go by rustqlite's docs which don't (seem to) indicate that connections are
created lazily.
