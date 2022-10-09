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
    - Shows untracked files only
    - Uses the porcelain output formatting as that's a lot easier to do

## Upcoming features

I'm currently working on improving the `status` command to show more stuff than
just 
