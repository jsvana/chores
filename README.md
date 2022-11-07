# chores

I want to track chores that I need to do. Instead of a sensible solution like post-it notes or a phone reminder, I opted to write this unnecessary webserver.

## Usage

Start by adding your desired chores to your own `config.json` file along with their frequency in cron format. Then, start the webserver and navigate to the proper address and port. You'll see the list of chores you need to complete. Do the chore and click the "Mark Completed" button. That's it!

## Building

```
$ cargo install sqlx-cli --no-default-features --features rustls --features sqlite
$ DATABASE_URL=sqlite:data.db sqlx database create
$ DATABASE_URL=sqlite:data.db cargo build
```

## License
[MIT](LICENSE.md)
