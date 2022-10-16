use std::env;
use std::path::Path;
use std::path::PathBuf;

use cargo_metadata::CargoOpt;

use clap::Parser;
use git2::{Cred, Oid, RemoteCallbacks};
use glob::glob;
use indexmap::IndexSet;
use tempfile::tempdir;

#[derive(Parser)]
#[command(name = "root-pkg-ws")]
#[command(author = "Joel Winarske <joel.winarske@gmail.com>")]
#[command(version = "1.0")]
#[command(about = "Lists Yocto Recipe for a Root Package Workspace", long_about = None)]
struct Cli {
    #[arg(long)]
    manifest_path: String,
}

#[derive(Eq, Hash, PartialEq)]
struct GitRepo {
    url: String,
    commit: String,
}

fn dump_metadata(path: impl Into<PathBuf>, crates: &mut IndexSet<String>, git: &mut IndexSet<GitRepo>) -> Vec<String> {
    let mut file_list = Vec::new();

    let _metadata = cargo_metadata::MetadataCommand::new()
        .manifest_path(path)
        .features(CargoOpt::AllFeatures)
        .exec()
        .unwrap();

    //println!("workspace_root: {}", _metadata.workspace_root);
    //println!("target_directory: {}", _metadata.target_directory);

    //let _members = _metadata.workspace_members;
    //for _member in _members.iter() {
    // println!("member: {}", _member.repr);
    //}

    let _resolve = _metadata.resolve.unwrap();
    let _nodes = _resolve.nodes;
    for _node in _nodes.iter() {
        let iter: Vec<_> = _node.id.repr.split_whitespace().collect();
        if iter[2] == "(registry+https://github.com/rust-lang/crates.io-index)" {
            let mut crate_repo: String = "crate://crates.io/".to_owned();
            let crate_name: String = iter[0].to_owned();
            let crate_version: String = iter[1].to_owned();

            crate_repo.push_str(&crate_name);
            crate_repo.push_str(&*"/".to_owned());
            crate_repo.push_str(&crate_version);

            crates.insert(crate_repo);
        } else if iter[2].contains("(path+") {
            let repo: Vec<_> = iter[2].split('+').collect();
            let repository = repo[1].replace(")", "");
            let path: Vec<_> = repository.split("file://").collect();
            file_list.push(path[1].to_owned());
        } else if iter[2].contains("(git+") {
            let repo: Vec<_> = iter[2].split('+').collect();
            let repository = repo[1].replace(")", "");
            let elements: Vec<_> = repository.split(&['?', '#'][..]).collect();
            let url = elements[0].to_owned();
            let commit;
            if elements.len() > 2 {
                commit = elements[2].to_owned();
            } else {
                commit = elements[1].to_owned();
            }
            let git_repo = GitRepo {
                url,
                commit,
            };
            git.insert(git_repo);
        } else {
            println!("[not handled] {}", iter[2]);
        }
    }

    return file_list;
}

fn get_repo_folder_name(url: String) -> String {
    let last = url.split('/')
        .last()
        .unwrap()
        .to_string();
    let res: Vec<_> = last.split(".git").collect();
    return res[0].to_string();
}

fn main() {
    let cli = Cli::parse();
    //println!("manifest-path: {:?}", cli.manifest_path);

    let mut crate_list = IndexSet::new();
    let mut git_list = IndexSet::new();

    let _ = dump_metadata(cli.manifest_path, &mut crate_list, &mut git_list);

    //println!();
    //for _file in file_list {
        //let _ = dump_metadata(format!("{}/Cargo.toml", _file), &mut crate_list, &mut git_list);
        //println!("{}", _file);
    //}

    println!();
    println!("SRC_URI += \" \\");
    //let mut count = 0;
    for _crate in crate_list.iter() {
        println!("    {} \\", _crate);
        //count = count + 1;
    }
    //println!("\nCount: {}", count);

    let dir = tempdir().unwrap();

    // Prepare callbacks.
    let mut callbacks = RemoteCallbacks::new();
    callbacks.credentials(|_url, username_from_url, _allowed_types| {
        Cred::ssh_key(
            username_from_url.unwrap(),
            None,
            Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
            None,
        )
    });

    // Prepare fetch options.
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(callbacks);

    // Prepare builder.
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    for _git in git_list.iter() {
        let protocol: Vec<_> = _git.url.split("://").collect();
        let folder = get_repo_folder_name(protocol[1].to_string());
        println!("    git://{};lfs=0;nobranch=1;protocol={};destsuffix={};name={} \\", protocol[1], protocol[0], folder, folder);

        let sub_folder = get_repo_folder_name(_git.url.to_string());
        let folder = dir.path().join(sub_folder);
        let repo = builder.clone(&_git.url, Path::new(&folder)).expect("failed to clone repository");

        let oid = Oid::from_str(&_git.commit).unwrap();
        let commit = repo.find_commit(oid).unwrap();

        let _ = repo.branch(
            &_git.commit,
            &commit,
            false,
        );

        let obj = repo.revparse_single(&("refs/heads/".to_owned() + &_git.commit)).unwrap();

        let _ = repo.checkout_tree(
            &obj,
            None,
        );

        let _ = repo.set_head(&("refs/heads/".to_owned() + &_git.commit));

        let _glob = String::from(folder.join("**/Cargo.toml").to_string_lossy());
        for entry in glob(&_glob).unwrap() {
            match entry {
                Ok(manifest) => {
                    let mut _git_list = IndexSet::new();
                    let _ = dump_metadata(manifest, &mut crate_list, &mut _git_list);
                }
                Err(e) => println!("Err: {:?}", e),
            }
        }
    }
    dir.close().unwrap();
    println!("\"\n");

    for _git in git_list.iter() {
        let protocol: Vec<_> = _git.url.split("://").collect();
        let folder = get_repo_folder_name(protocol[1].to_string());
        println!("SRCREV_FORMAT .= \"_{}\"", folder);
        println!("SRCREV_{} = \"{}\"", folder, _git.commit);
    }

    if !git_list.is_empty() {
        println!();
        println!("EXTRA_OECARGO_PATHS += \"\\");
        for _git in git_list.iter() {
            let protocol: Vec<_> = _git.url.split("://").collect();
            let folder = get_repo_folder_name(protocol[1].to_string());
            println!("    ${{WORKDIR}}/{} \\", folder);
        }
        println!("\"");
    }
}
