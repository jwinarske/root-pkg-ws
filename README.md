# root-pkg-ws
Rust Package Workspace Yocto Dependency Tool

I created this tool as `cargo bitbake` was not working in my use cases (modern and very large Rust project).

The usage process should look like this:
* Create rust-toolchain file in root of project, if not present that matches the toolchain used in Yocto.  This ensure the correct packages are referenced.
    * This will pull specified version of rust toolchain
* Run root-pkg-ws pointing to Cargo.toml of desired project.
    * root-pkg-ws --manifest-path=`pwd`/Cargo.toml

Replace SRC_URI section of recipe with output from this tool.

The plan is to submit a PR to meta-rust/cargo-bitbake at some point.


Example rust-toolchain lock file:  https://github.com/google/crosvm/blob/main/rust-toolchain
