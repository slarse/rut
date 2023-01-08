# Rut - A Git clone written in Rust
Rut is a clone of Git, written in Rust. I use it as a learning project to
deepen my knowledge of both Git and Rust. I'm still rather new to Rust so
I would not recommend anyone looking at this project as a good example for how
to write Rust.

Currently I'm cheating a bit with this repository as I've not implemented
support for remote repositories yet (i.e pushing and pulling). So here,
I periodically push changes just so I have a backup of my project.

> This work is based on James Coglan's fantastic book [Building
> Git](https://shop.jcoglan.com/building-git/). It provides a detailed
> description of how Git works under the hood, as well as an implementation of a
> subset of Git in Ruby. I HIGHLY recommend it if you are interested in learning
> more about Git.

## How to use

`rut` requires Rust and Cargo to be installed. It's been tested to work on
Rust/Cargo version 1.61. Your mileage may vary on other versions.

To try `rut` out, clone the repository and build the project:

```bash
$ git clone https://github.com/slarse/rut.git
$ cd rut
$ cargo build
```

Then you can run commands like so:

```bash
$ cargo run <command>
```

For example, the commands I will run to commit this update of the README are
the following:

```bash
$ cargo run add README.md
$ echo 'Add usage instructions to README' > .git/COMMIT_EDITMSG
$ cargo run commit
```

## Current features

Rut currently supports the following subset of Git:

* `init`
    - Initializes a repository in the current directory
* `add`
    - It's possible to add a _single_ path at a time
    - If the path is a directory, every file the file tree rooted in that
      directory is added
* `rm`
    - It's possible to remove a single file at a time
* `commit`
    - Create a commit of the current index
    - Author and email is taken from the `GIT_AUTHOR_NAME` and
      `GIT_AUTHOR_EMAIL` environment variables
    - The commit message is taken from the `.git/COMMIT_EDITMSG` file
* `status`
    - Mostly up-to-par with `git status`
    - Currently does not attempt to identify renamed files. This requires
      diffing which is currently not implemented.
    - Accepts the `--porcelain` flag to output in a format that is easier to parse

## Upcoming features

I'm currently working on implementing `git diff`.
