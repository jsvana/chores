# chores

## Building

```
$ cargo install sqlx-cli --no-default-features --features rustls --features sqlite
$ DATABASE_URL=sqlite:data.db sqlx database create
$ DATABASE_URL=sqlite:data.db cargo run
