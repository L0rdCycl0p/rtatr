# Rust Task Tracker

This is a **Rust port and fork** of [ `tsoding/tatr` ](https://github.com/tsoding/tatr)
by **Alexey "tsoding" Kutepov** .

* **Upstream (original project):** [tsoding/tatr](https://github.com/tsoding/tatr) — written in C, licensed under GPLv2.
* **This fork:** maintained by **L0rdCycl0p** ([@L0rdCycl0p](https://github.com/L0rdCycl0p)).
* **What this fork is:** a complete Rust rewrite of the original codebase.

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

Tasks live at `tasks/<HUID>/TASK.md` , matching upstream `tatr` :

    # Fix login bug

    - STATUS: OPEN
    - PRIORITY: 100
    - TAGS: work,urgent
    Steps to reproduce…

Attachments (screenshots, logs, etc.) can be placed alongside `TASK.md` 
in the same directory.

## Commands

| Command | Notes |
|---|---|
| `init` | create `tasks/` |
| `new` (alias `add` ) | create a task; `-t TAG` , `-p PRIO` , `-s SUFFIX` |
| `ls [QUERY]` | `-c` , `-a` , `-id` , `-update OPS` , `-debug` |
| `todo <FILE…>` | interactive scan for `TODO:` |
| `find <HUID>` | report for a single task; `--path-only` |
| `id` | HUID of current task folder |
| `ref [HUID]` | grep for references to a task |
| `graph` | emit `graph.dot` (needs `neato` for SVG) |
| `summary` | tag counts; reads `tasks/tags` |
| `tag` / `untag` | **deprecated** , prints the `ls -update …` equivalent |
| `help [COMMAND]` | help for the program or a specific command |
| `version` | print version |

rtatr extensions (not in upstream): `done` , `rm` , `show` , 
`update <HUID> --add-tag/--remove-tag` .

## TQL

| Syntax | Meaning |
|---|---|
| `:tag` | task has tag |
| `not <expr>` | negation |
| `and` / `or` | logical operators |
| `[ <expr> ]` | grouping |
| `any` | always true |
| `tagged` | task has at least one tag |
| `priority` | task priority as integer |
| `<a> lt/le/gt/ge/eq/ne <b>` | integer comparison |
| `<HUID>` | task with that HUID |
| `<n>` | integer literal |
| `"some words"` | rtatr extension: full-text body search |
| `word` | rtatr extension: same, single word |
| `key=value` | rtatr extension: property equality |
| `key` | rtatr extension: property exists |


## Migration notes from the C version

* `nob.c` -> `Cargo.toml` + `cargo`.
* `malloc`/`free` -> ownership; no unsafe required.
* Error returns -> `Result<T, RtatrError>` with `thiserror`.
* Hand-rolled TQL parser (previously C) lives in `src/query.rs`.

## License

This fork is distributed under the **GNU General Public License, version 2 (GPLv2)** , 
inherited from the original project. See the [ `LICENSE` ](./LICENSE) file, which is
the unmodified GPLv2 license text from upstream.

* Original C code and design: © Alexey "tsoding" Kutepov, licensed under GPLv2.
* Rust port and modifications in this repository: © L0rdCycl0p, also licensed under GPLv2.
