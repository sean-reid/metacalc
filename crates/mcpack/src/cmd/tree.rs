//! Goal trees and statements in Unicode.

use super::{read, unusable};
use mcpack::mm::{Database, Kind, Node};
use mcpack::page::{escape, unicode};
use std::collections::HashMap;
use std::fmt::Write;
use std::io::Read;
use std::path::Path;
use std::process::ExitCode;

/// How `outline` prints a tree.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// The tokens of the database, one indented line per goal.
    Ascii,
    /// The same lines with each token spelled in Unicode.
    Unicode,
    /// A nested HTML list.
    Html,
}

/// The page of each label, read from `LABEL PAGE` lines.
pub(super) fn read_links(path: &Path) -> Result<HashMap<String, String>, ExitCode> {
    let text = String::from_utf8(read(path)?)
        .map_err(|e| unusable(format_args!("{}: {e}", path.display())))?;
    text.lines()
        .map(|line| {
            line.split_once(' ')
                .map(|(label, page)| (label.to_string(), page.to_string()))
                .ok_or_else(|| unusable(format_args!("{}: no page in {line:?}", path.display())))
        })
        .collect()
}

fn words<'a>(db: &'a Database, node: &Node) -> Vec<&'a str> {
    node.expr
        .iter()
        .map(|&s| db.syms[s as usize].name.as_str())
        .collect()
}

fn walk<'a>(node: &'a Node, out: &mut Vec<&'a Node>) {
    out.push(node);
    for child in &node.children {
        walk(child, out);
    }
}

/// The tokens of the tree that have no Unicode spelling, in order of first use.
fn unspelled(db: &Database, root: &Node) -> Vec<String> {
    let mut nodes = Vec::new();
    walk(root, &mut nodes);
    let mut missing: Vec<String> = Vec::new();
    for node in nodes {
        for token in words(db, node) {
            if unicode::glyph(token).is_none() && !missing.iter().any(|m| m == token) {
                missing.push(token.to_string());
            }
        }
    }
    missing
}

fn plain(tokens: &[&str]) -> String {
    let spelled: Vec<&str> = tokens
        .iter()
        .map(|t| unicode::glyph(t).map_or(*t, |s| s.text))
        .collect();
    spelled.join(" ")
}

pub(super) fn render(
    db: &Database,
    root: &Node,
    form: Form,
    links: Option<&HashMap<String, String>>,
) -> Result<String, String> {
    if form == Form::Ascii {
        return Ok(root.render(db));
    }
    let missing = unspelled(db, root);
    if !missing.is_empty() {
        return Err(format!("no Unicode spelling for {}", missing.join(" ")));
    }
    let mut out = String::new();
    if form == Form::Unicode {
        text(db, root, 0, &mut out);
        return Ok(out);
    }
    let ascii = words(db, root).join(" ");
    let _ = writeln!(
        out,
        "<ul class=\"tree\" data-statement=\"{}\">",
        escape(&ascii)
    );
    item(db, root, 1, links, &mut out)?;
    out.push_str("</ul>\n");
    Ok(out)
}

fn text(db: &Database, node: &Node, depth: usize, out: &mut String) {
    let _ = writeln!(
        out,
        "{:pad$}{}  [{}]",
        "",
        plain(&words(db, node)),
        db.stmt(node.label).label,
        pad = depth * 2
    );
    for child in &node.children {
        text(db, child, depth + 1, out);
    }
}

fn item(
    db: &Database,
    node: &Node,
    depth: usize,
    links: Option<&HashMap<String, String>>,
    out: &mut String,
) -> Result<(), String> {
    let pad = depth * 2;
    let stmt = db.stmt(node.label);
    let label = escape(&stmt.label);
    let name = match links.and_then(|l| l.get(&stmt.label)) {
        _ if stmt.kind == Kind::Essential => format!("<span class=\"lb\">{label}</span>"),
        Some(page) => format!(
            "<a class=\"lb\" href=\"{}#{label}\">{label}</a>",
            escape(page)
        ),
        None if links.is_none() => format!("<span class=\"lb\">{label}</span>"),
        None => return Err(format!("{} is on no library page", stmt.label)),
    };
    let spelled = unicode::spell(&words(db, node));
    let head = format!("<li><span class=\"st\">{spelled}</span> {name}");
    if node.children.is_empty() {
        let _ = writeln!(out, "{:pad$}{head}</li>", "");
        return Ok(());
    }
    let _ = writeln!(out, "{:pad$}{head}", "");
    let _ = writeln!(out, "{:pad$}<ul>", "", pad = pad + 2);
    for child in &node.children {
        item(db, child, depth + 2, links, out)?;
    }
    let _ = writeln!(out, "{:pad$}</ul>", "", pad = pad + 2);
    let _ = writeln!(out, "{:pad$}</li>", "");
    Ok(())
}

/// Reads statements from standard input and prints each in Unicode.
pub fn spell() -> Result<ExitCode, ExitCode> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(|e| unusable(format_args!("standard input: {e}")))?;
    let mut missing: Vec<&str> = Vec::new();
    let mut out = String::new();
    for line in input.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        for token in &tokens {
            if unicode::glyph(token).is_none() && !missing.contains(token) {
                missing.push(token);
            }
        }
        out.push_str(&plain(&tokens));
        out.push('\n');
    }
    if !missing.is_empty() {
        return Err(unusable(format_args!(
            "no Unicode spelling for {}",
            missing.join(" ")
        )));
    }
    print!("{out}");
    Ok(ExitCode::SUCCESS)
}
