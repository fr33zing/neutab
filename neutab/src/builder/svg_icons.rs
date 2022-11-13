//! Manages cloning the material design icon repository and building reusable SVG symbol defs. Also
//! provides some utility functions relevant to site icons.
//!
//! Icons repository: <https://github.com/marella/material-design-icons>

#![allow(clippy::missing_docs_in_private_items)]

use std::fmt::{self, Write};
use std::path::PathBuf;
use std::{
    fs,
    io::{self},
    path::Path,
};

use git2::Repository;
use itertools::Itertools;
use thiserror::Error;
use tokio::time::Instant;
use tracing::{debug, info, span, Level};

use crate::{config::Config, util};

/// Errors that may occur when cloning the icon or building svg icons.
#[derive(Error, Debug)]
pub enum SvgIconError {
    /// Occurs when writing the build output fails.
    #[error(transparent)]
    Output(#[from] fmt::Error),

    /// Occurs when no suitable place to clone the icon repo can be found.
    #[error("failed to locate cache dir")]
    CacheDir,

    /// Occurs when creating the icon repo directory fails.
    #[error(transparent)]
    MakeDir(#[from] io::Error),

    /// Occurs when [`git2`] encounters an error.
    #[error(transparent)]
    Repo(#[from] git2::Error),

    /// Occurs when loading an icon SVG from the icon repo fails.
    #[error("failed to load icon: '{1}' of style '{2}' @ '{3}' ({0})")]
    IconLoad(#[source] io::Error, String, String, PathBuf),

    /// Occurs when a requested icon could not be found in the icon repo.
    #[error("failed to find icon: '{0}' of style '{1}' @ '{2}'")]
    IconNotFound(String, String, PathBuf),
}

/// Generates a unique ID for an icon, based on the icon name and style.
pub fn svg_icon_id(icon_name: &str, icon_style: &str) -> String {
    format!(
        "svg-{}",
        util::sha1_base32(format!("{icon_name} {icon_style}").as_bytes())
    )
}

/// Clones or updates the icons repo and converts requested icons SVGs into SVG symbol definitions.
///
/// # Arguments
///
/// * `config` - The config to extract icon references from.
///
/// # Errors
///
/// Returns an error if cloning the icon repo or processing the icons fails.
///
/// # Returns
///
/// An HTML SVG containing symbol definitions. The IDs of the symbols are derived from their icon
/// name and style.
pub fn build_svg_icons(config: &Config) -> Result<String, SvgIconError> {
    let _span = span!(Level::INFO, "svg_icons").entered();
    info!("building svg icons");
    let sw = Instant::now();

    let repo_root = icons_repo()?;
    let mut symbol_defs = String::default();
    config
        .pages
        .iter()
        .map(|page| (page.icon.clone(), page.icon_style.clone()))
        .unique()
        .map(|t| load_icon(&repo_root, &t.0, &t.1).map(|src| (src, t.0, t.1)))
        .collect::<Result<Vec<(String, String, String)>, SvgIconError>>()?
        .iter()
        .map(|t| to_symbol_def(&t.0, &t.1, &t.2))
        .try_for_each(|sym_def| symbol_defs.write_str(&sym_def))?;

    debug!(
        elapsed_ms = sw.elapsed().as_millis(),
        "finished building svg icons"
    );
    Ok(format!(
        r#"<svg style="display:none"><defs>{symbol_defs}</defs></svg>"#
    ))
}

/// Clones or updates the icons repository and returns its root directory.
fn icons_repo() -> Result<PathBuf, SvgIconError> {
    let _span = span!(Level::DEBUG, "repo").entered();

    let cache_dir = dirs::cache_dir()
        .ok_or(SvgIconError::CacheDir)?
        .join("neutab");
    let repo_dir = cache_dir.join("material-design-icons");
    let repo_url = "https://github.com/marella/material-design-icons.git";

    fs::create_dir_all(repo_dir.clone())?;
    match Repository::open(repo_dir.clone()) {
        Ok(repo) => {
            debug!(
                repo_url,
                repo_dir = repo_dir.to_str(),
                "pulling svg icons repo"
            );
            pull(&repo)?;
        }
        Err(_) => {
            debug!(
                repo_url,
                repo_dir = repo_dir.to_str(),
                "cloning svg icons repo"
            );
            Repository::clone(repo_url, repo_dir.clone())?;
        }
    }

    Ok(repo_dir)
}

/// Locates and loads an icon SVG based on the provided icon name and style.
///
/// # Errors
///
/// Returns an error if the icon SVG file wasn't found or couldn't be read.
///
/// # Returns
///
/// SVG element markup.
fn load_icon(repo_dir: &Path, name: &str, style: &str) -> Result<String, SvgIconError> {
    let svgs_path = repo_dir.join("svg");
    let style_path = svgs_path.join(style);
    let icon_path = style_path.join(format!("{name}.svg"));

    if icon_path.exists() {
        fs::read_to_string(icon_path.clone())
            .map_err(|e| SvgIconError::IconLoad(e, name.into(), style.into(), icon_path))
    } else {
        Err(SvgIconError::IconNotFound(
            name.into(),
            style.into(),
            icon_path,
        ))
    }
}

/// Converts an SVG into an SVG symbol definition.
///
/// # Returns
///
/// SVG symbol element markup.
fn to_symbol_def(src: &str, name: &str, style: &str) -> String {
    let id = svg_icon_id(name, style);
    let remove_start = r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24""#;
    let remove_end = "</svg>";
    let add_start = format!(r#"<symbol id="{id}""#);
    let add_end = "</symbol>";
    let middle = src[(remove_start.len())..(src.len() - remove_end.len())].to_string();
    format!("{add_start}{middle}{add_end}")
}

// The following code was adapted from an example written by github.com/zaphar
// https://github.com/rust-lang/git2-rs/blob/master/examples/pull.rs

fn pull(repo: &Repository) -> Result<(), git2::Error> {
    let remote_name = "origin";
    let remote_branch = "main";
    let mut remote = repo.find_remote(remote_name)?;
    let fetch_commit = do_fetch(repo, &[remote_branch], &mut remote)?;
    do_merge(repo, remote_branch, fetch_commit)
}

fn do_fetch<'a>(
    repo: &'a git2::Repository,
    refs: &[&str],
    remote: &'a mut git2::Remote,
) -> Result<git2::AnnotatedCommit<'a>, git2::Error> {
    let mut cb = git2::RemoteCallbacks::new();

    // Print out our transfer progress.
    cb.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            debug!(
                "Resolving deltas {}/{}\r",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            debug!(
                "Received {}/{} objects ({}) in {} bytes\r",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        io::Write::flush(&mut io::stdout()).unwrap();
        true
    });

    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(cb);
    // Always fetch all tags.
    // Perform a download and also update tips
    fo.download_tags(git2::AutotagOption::All);
    debug!("Fetching {} for repo", remote.name().unwrap());
    remote.fetch(refs, Some(&mut fo), None)?;

    // If there are local objects (we got a thin pack), then tell the user
    // how many objects we saved from having to cross the network.
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        debug!(
            "\rReceived {}/{} objects in {} bytes (used {} local \
             objects)",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        debug!(
            "\rReceived {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    repo.reference_to_annotated_commit(&fetch_head)
}

fn fast_forward(
    repo: &Repository,
    lb: &mut git2::Reference,
    rc: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let name = match lb.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(lb.name_bytes()).to_string(),
    };
    let msg = format!("Fast-Forward: Setting {} to id: {}", name, rc.id());
    debug!("{}", msg);
    lb.set_target(rc.id(), &msg)?;
    repo.set_head(&name)?;
    repo.checkout_head(Some(
        git2::build::CheckoutBuilder::default()
            // For some reason the force is required to make the working directory actually get updated
            // I suspect we should be adding some logic to handle dirty working directory states
            // but this is just an example so maybe not.
            .force(),
    ))?;
    Ok(())
}

fn normal_merge(
    repo: &Repository,
    local: &git2::AnnotatedCommit,
    remote: &git2::AnnotatedCommit,
) -> Result<(), git2::Error> {
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;
    let mut idx = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    if idx.has_conflicts() {
        debug!("Merge conficts detected...");
        repo.checkout_index(Some(&mut idx), None)?;
        return Ok(());
    }
    let result_tree = repo.find_tree(idx.write_tree_to(repo)?)?;
    // now create the merge commit
    let msg = format!("Merge: {} into {}", remote.id(), local.id());
    let sig = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    // Do our merge commit and set current branch head to that commit.
    let _merge_commit = repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &msg,
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;
    // Set working tree to match head.
    repo.checkout_head(None)?;
    Ok(())
}

fn do_merge<'a>(
    repo: &'a Repository,
    remote_branch: &str,
    fetch_commit: git2::AnnotatedCommit<'a>,
) -> Result<(), git2::Error> {
    // 1. do a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // 2. Do the appopriate merge
    if analysis.0.is_fast_forward() {
        debug!("Doing a fast forward");
        // do a fast forward
        let refname = format!("refs/heads/{}", remote_branch);
        match repo.find_reference(&refname) {
            Ok(mut r) => {
                fast_forward(repo, &mut r, &fetch_commit)?;
            }
            Err(_) => {
                // The branch doesn't exist so just set the reference to the
                // commit directly. Usually this is because you are pulling
                // into an empty repository.
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("Setting {} to {}", remote_branch, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;
                repo.checkout_head(Some(
                    git2::build::CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        };
    } else if analysis.0.is_normal() {
        // do a normal merge
        let head_commit = repo.reference_to_annotated_commit(&repo.head()?)?;
        normal_merge(repo, &head_commit, &fetch_commit)?;
    } else {
        debug!("Nothing to do...");
    }
    Ok(())
}
