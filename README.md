# rngo CLI

See [rngo.dev/docs/cli](https://rngo.dev/docs/cli).

## Development

### Test

Set up the database, run the checked-in simulation and inspect the db:

```bash
sqlite3 db1.sqlite < test/db1-schema.sql
cargo run sim
sqlite3 db1.sqlite
```

### Release

To release, set `package.version` in `Cargo.toml` and run `./script/release`. This will kick off the [build](.github/workflows/build.yml) and [release](.github/workflows/release.yml) workflows in Github Actions.

Once these workflows complete, go to the newly created release on github.com and add release notes.
