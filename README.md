# rngo CLI

See [docs.rngo.dev](https://docs.rngo.dev).

## Development

To release version `1.2.3`, do the following:

1. Set `package.version` in `Cargo.toml` to "1.2.3"
2. Run `git add Cargo.toml Cargo.lock`
3. Run `git commit -m "1.2.3"`
4. Run `git tag 1.2.3`
5. Run `git push origin main --tags`

This will kick off the [build](.github/workflows/build.yml) and [release](.github/workflows/release.yml) workflows in Github Actions.

Once these workflows complete, go to the newly created release on github.com and add release notes.
