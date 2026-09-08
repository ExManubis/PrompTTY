PrompTTY is a local-first terminal and agentic development environment, forked from [Warp](https://www.warp.dev). It keeps the desktop and TUI clients. Host builds are **macOS and Linux only**; SSH into Windows remotes still works. Warp-hosted cloud services (Drive, teams, billing, Oz, telemetry, autoupdate) are being removed.

The UI framework (`warpui_core` and `warpui`) is licensed under the [MIT license](LICENSE-MIT). The rest of the repository is licensed under the [AGPL v3](LICENSE-AGPL).

## Build

```bash
./script/bootstrap
./script/run
```

Headless TUI:

```bash
./script/run-tui
```

Checks:

```bash
./script/presubmit
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Security reports go through [SECURITY.md](SECURITY.md), not public issues.
