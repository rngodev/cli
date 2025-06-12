# rngo CLI

See [docs.rngo.dev](https://docs.rngo.dev).

## Development

To release, set `package.version` in `Cargo.toml` and run `./script/release`. This will kick off the [build](.github/workflows/build.yml) and [release](.github/workflows/release.yml) workflows in Github Actions.

Once these workflows complete, go to the newly created release on github.com and add release notes.
