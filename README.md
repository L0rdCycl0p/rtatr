# Rust Task Tracker

This is a **Rust port and fork** of [`tsoding/tatr`](https://github.com/tsoding/tatr)
by **Alexey "tsoding" Kutepov**.

- **Upstream (original project):** [tsoding/tatr](https://github.com/tsoding/tatr) — written in C, licensed under GPLv2.
- **This fork:** maintained by **L0rdCycl0p** ([@L0rdCycl0p](https://github.com/L0rdCycl0p)).
- **What this fork is:** a complete Rust rewrite of the original codebase.


## Install

    cargo install rtatr

## Usage

    rtatr add --tag work --tag urgent "Fix login bug"
    rtatr ls ":work and not :done"
    rtatr done 20240101-120000
    rtatr update 20240101-120000 --add-tag blocked
    rtatr show 20240101-120000
    rtatr rm 20240101-120000

## Task file format

Files live under `./tasks/<huid>.md`:

    tags: :work :urgent
    due: 2024-12-31

    # Fix login bug

    Steps to reproduce…

## TQL

| Syntax             | Meaning                                   |
|--------------------|-------------------------------------------|
| `:tag`             | task has tag                              |
| `not :tag`         | negate                                    |
| `expr and expr`    | both                                      |
| `expr or expr`     | either                                    |
| `( expr )`         | grouping                                  |
| `"some words"`     | full-text search over the markdown body   |
| `word`             | same, single word                         |
| `key=value`        | property equality                         |
| `key`              | property exists                           |

## Migration notes from the C version

* `nob.c` -> `Cargo.toml` + `cargo`.
* `malloc`/`free` -> ownership; no unsafe required.
* Error returns -> `Result<T, RtatrError>` with `thiserror`.
* Hand-rolled TQL parser (previously C) lives in `src/query.rs`.

## License

This fork is distributed under the **GNU General Public License, version 2 (GPLv2)**,
inherited from the original project. See the [`LICENSE`](./LICENSE) file, which is
the unmodified GPLv2 license text from upstream.

- Original C code and design: © Alexey "tsoding" Kutepov, licensed under GPLv2.
- Rust port and modifications in this repository: © L0rdCycl0p, also licensed under GPLv2.