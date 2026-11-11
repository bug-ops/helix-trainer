//! The module layer order, enforced.
//!
//! Every library module has a rank; a module may only depend on a strictly
//! lower rank (the order is in the constitution, Section I). The test parses
//! the library's real module tree with `syn`, collects every `crate::<module>`
//! reference in production code, and fails on any upward or same-rank edge.
//!
//! Skipped on purpose: every node carrying `#[cfg(test)]` (or
//! `cfg(all(test, ..))`) at any depth, doc comments (they are attributes, not
//! code), and the bin-crate modules, which sit above every library layer.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use proc_macro2::{TokenStream, TokenTree};
use syn::visit::Visit;
use syn::{Attribute, Expr, ImplItem, Item, Meta, Stmt, TraitItem, UseTree};

/// Library modules by rank; an edge may only point to a strictly lower rank.
const RANKS: &[(&str, u8)] = &[
    ("grammar", 0),
    ("limits", 0),
    ("constants", 0),
    ("numeric", 0),
    ("time", 0),
    ("ids", 1),
    ("security", 1),
    ("helix", 2),
    ("config", 3),
    ("learning", 4),
    ("sound", 4),
    ("gamification", 5),
    ("game", 6),
    ("minigame", 7),
    ("error", 8),
    ("input", 9),
    ("async_state", 10),
    ("ui", 11),
    ("data_loader", 12),
];

/// Upward edges that are known and tracked, `(from, to)`.
///
/// `security` holds `UserError` and `ErrorCause`, which wrap the domain errors
/// above it. Moving them into an error hub above every domain module needs a
/// domain error type per session and loader API.
// Tracked in #606: the user-facing error hub still sits below the domain modules it wraps.
const ALLOWED: &[(&str, &str)] = &[
    ("security", "config"),
    ("security", "gamification"),
    ("security", "helix"),
    ("security", "learning"),
];

fn rank_of(module: &str) -> Option<u8> {
    RANKS
        .iter()
        .find(|(name, _)| *name == module)
        .map(|(_, rank)| *rank)
}

fn has_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|attr| {
        attr.path().is_ident("cfg")
            && attr
                .parse_args_with(
                    syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated,
                )
                .is_ok_and(|args| args.iter().any(meta_requires_test))
    })
}

/// Whether a `cfg` predicate holds only in test builds: `test` or `all(.., test, ..)`.
fn meta_requires_test(meta: &Meta) -> bool {
    match meta {
        Meta::Path(path) => path.is_ident("test"),
        Meta::List(list) if list.path.is_ident("all") => list
            .parse_args_with(syn::punctuated::Punctuated::<Meta, syn::Token![,]>::parse_terminated)
            .is_ok_and(|args| args.iter().any(meta_requires_test)),
        _ => false,
    }
}

fn item_attrs(item: &Item) -> &[Attribute] {
    match item {
        Item::Const(i) => &i.attrs,
        Item::Enum(i) => &i.attrs,
        Item::ExternCrate(i) => &i.attrs,
        Item::Fn(i) => &i.attrs,
        Item::ForeignMod(i) => &i.attrs,
        Item::Impl(i) => &i.attrs,
        Item::Macro(i) => &i.attrs,
        Item::Mod(i) => &i.attrs,
        Item::Static(i) => &i.attrs,
        Item::Struct(i) => &i.attrs,
        Item::Trait(i) => &i.attrs,
        Item::TraitAlias(i) => &i.attrs,
        Item::Type(i) => &i.attrs,
        Item::Union(i) => &i.attrs,
        Item::Use(i) => &i.attrs,
        _ => &[],
    }
}

fn impl_item_attrs(item: &ImplItem) -> &[Attribute] {
    match item {
        ImplItem::Const(i) => &i.attrs,
        ImplItem::Fn(i) => &i.attrs,
        ImplItem::Type(i) => &i.attrs,
        ImplItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn trait_item_attrs(item: &TraitItem) -> &[Attribute] {
    match item {
        TraitItem::Const(i) => &i.attrs,
        TraitItem::Fn(i) => &i.attrs,
        TraitItem::Type(i) => &i.attrs,
        TraitItem::Macro(i) => &i.attrs,
        _ => &[],
    }
}

fn stmt_attrs(stmt: &Stmt) -> &[Attribute] {
    match stmt {
        Stmt::Local(local) => &local.attrs,
        Stmt::Macro(mac) => &mac.attrs,
        Stmt::Expr(..) | Stmt::Item(_) => &[],
    }
}

/// Collects the library modules a file names through `crate::`.
#[derive(Default)]
struct EdgeCollector {
    targets: BTreeSet<String>,
}

impl EdgeCollector {
    fn use_tree(&mut self, tree: &UseTree) {
        if let UseTree::Path(path) = tree
            && path.ident == "crate"
        {
            self.crate_subtree(&path.tree);
        }
    }

    fn crate_subtree(&mut self, tree: &UseTree) {
        match tree {
            UseTree::Path(path) => {
                self.targets.insert(path.ident.to_string());
            }
            UseTree::Name(name) => {
                self.targets.insert(name.ident.to_string());
            }
            UseTree::Rename(rename) => {
                self.targets.insert(rename.ident.to_string());
            }
            UseTree::Group(group) => {
                for item in &group.items {
                    self.crate_subtree(item);
                }
            }
            UseTree::Glob(_) => {}
        }
    }

    fn tokens(&mut self, tokens: TokenStream) {
        let trees: Vec<TokenTree> = tokens.into_iter().collect();
        for (index, tree) in trees.iter().enumerate() {
            match tree {
                TokenTree::Group(group) => self.tokens(group.stream()),
                TokenTree::Ident(ident) if ident == "crate" => {
                    if let [
                        TokenTree::Punct(first),
                        TokenTree::Punct(second),
                        TokenTree::Ident(target),
                    ] = &trees[(index + 1).min(trees.len())..(index + 4).min(trees.len())]
                        && first.as_char() == ':'
                        && second.as_char() == ':'
                    {
                        self.targets.insert(target.to_string());
                    }
                }
                _ => {}
            }
        }
    }
}

impl<'ast> Visit<'ast> for EdgeCollector {
    fn visit_item(&mut self, item: &'ast Item) {
        if has_cfg_test(item_attrs(item)) {
            return;
        }
        if let Item::Use(item_use) = item {
            self.use_tree(&item_use.tree);
            return;
        }
        syn::visit::visit_item(self, item);
    }

    fn visit_impl_item(&mut self, item: &'ast ImplItem) {
        if !has_cfg_test(impl_item_attrs(item)) {
            syn::visit::visit_impl_item(self, item);
        }
    }

    fn visit_trait_item(&mut self, item: &'ast TraitItem) {
        if !has_cfg_test(trait_item_attrs(item)) {
            syn::visit::visit_trait_item(self, item);
        }
    }

    fn visit_stmt(&mut self, stmt: &'ast Stmt) {
        if !has_cfg_test(stmt_attrs(stmt)) {
            syn::visit::visit_stmt(self, stmt);
        }
    }

    fn visit_field(&mut self, field: &'ast syn::Field) {
        if !has_cfg_test(&field.attrs) {
            syn::visit::visit_field(self, field);
        }
    }

    fn visit_arm(&mut self, arm: &'ast syn::Arm) {
        if !has_cfg_test(&arm.attrs) {
            syn::visit::visit_arm(self, arm);
        }
    }

    fn visit_expr(&mut self, expr: &'ast Expr) {
        syn::visit::visit_expr(self, expr);
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        if path.segments.len() >= 2 && path.segments[0].ident == "crate" {
            self.targets.insert(path.segments[1].ident.to_string());
        }
        syn::visit::visit_path(self, path);
    }

    fn visit_macro(&mut self, mac: &'ast syn::Macro) {
        self.tokens(mac.tokens.clone());
        syn::visit::visit_macro(self, mac);
    }
}

/// The library modules `source` depends on through `crate::`.
fn crate_edges(source: &str) -> Result<BTreeSet<String>, String> {
    let file = syn::parse_file(source).map_err(|e| format!("cannot parse source: {e}"))?;
    let mut collector = EdgeCollector::default();
    collector.visit_file(&file);
    Ok(collector.targets)
}

/// The violations of the layer order by the source of module `module`.
fn violations_in(module: &str, source: &str) -> Result<Vec<String>, String> {
    let Some(rank) = rank_of(module) else {
        return Err(format!("module `{module}` has no rank"));
    };
    let mut found = Vec::new();
    for target in crate_edges(source)? {
        if target == module {
            continue;
        }
        let Some(target_rank) = rank_of(&target) else {
            return Err(format!(
                "`{module}` names `crate::{target}`, which has no rank"
            ));
        };
        if target_rank >= rank {
            found.push(format!("{module} -> {target}"));
        }
    }
    Ok(found)
}

/// Declared `mod` items of `source` that are not test-only.
fn declared_modules(source: &str) -> Result<Vec<String>, String> {
    let file = syn::parse_file(source).map_err(|e| format!("cannot parse source: {e}"))?;
    Ok(file
        .items
        .iter()
        .filter_map(|item| match item {
            Item::Mod(item_mod) if item_mod.content.is_none() && !has_cfg_test(&item_mod.attrs) => {
                Some(item_mod.ident.to_string())
            }
            _ => None,
        })
        .collect())
}

fn src_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()))
}

/// The file of child module `name` declared in `parent`.
fn child_file(parent: &Path, name: &str) -> PathBuf {
    let dir = parent.parent().expect("a file has a parent");
    let owns_dir = matches!(
        parent.file_name().and_then(|n| n.to_str()),
        Some("lib.rs" | "main.rs" | "mod.rs")
    );
    let base = if owns_dir {
        dir.to_path_buf()
    } else {
        dir.join(parent.file_stem().expect("a file has a stem"))
    };
    let flat = base.join(format!("{name}.rs"));
    if flat.exists() {
        flat
    } else {
        base.join(name).join("mod.rs")
    }
}

/// Every file of the non-test module tree below `file`.
fn module_tree(file: &Path, out: &mut Vec<PathBuf>) {
    out.push(file.to_path_buf());
    let source = read(file);
    for name in declared_modules(&source).unwrap_or_else(|e| panic!("{}: {e}", file.display())) {
        let child = child_file(file, &name);
        assert!(
            child.exists(),
            "module `{name}` declared in {} has no file",
            file.display()
        );
        module_tree(&child, out);
    }
}

/// The modules of the bin crate: declared in `main.rs`, above every library layer.
fn bin_modules() -> BTreeSet<String> {
    let main = src_dir().join("main.rs");
    declared_modules(&read(&main))
        .unwrap_or_else(|e| panic!("main.rs: {e}"))
        .into_iter()
        .collect()
}

#[test]
fn the_library_obeys_the_layer_order() {
    let lib = src_dir().join("lib.rs");
    let bin = bin_modules();
    let lib_modules = declared_modules(&read(&lib)).unwrap_or_else(|e| panic!("lib.rs: {e}"));

    let mut broken = Vec::new();
    let mut seen_allowed = BTreeSet::new();
    for module in lib_modules {
        if bin.contains(&module) {
            continue;
        }
        let mut files = Vec::new();
        module_tree(&child_file(&lib, &module), &mut files);
        for file in files {
            let found = violations_in(&module, &read(&file))
                .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
            for edge in found {
                let (from, to) = edge.split_once(" -> ").expect("edge format");
                if ALLOWED.contains(&(from, to)) {
                    seen_allowed.insert((from.to_string(), to.to_string()));
                } else {
                    broken.push(format!("{edge} ({})", file.display()));
                }
            }
        }
    }
    assert!(
        broken.is_empty(),
        "layering violations:\n{}",
        broken.join("\n")
    );
    for (from, to) in ALLOWED {
        assert!(
            seen_allowed.contains(&((*from).to_string(), (*to).to_string())),
            "allow-list entry {from} -> {to} is no longer needed; remove it"
        );
    }
}

#[test]
fn every_library_module_has_a_rank() {
    let lib = read(&src_dir().join("lib.rs"));
    let bin = bin_modules();
    for module in declared_modules(&lib).expect("lib.rs parses") {
        assert!(
            bin.contains(&module) || rank_of(&module).is_some(),
            "module `{module}` has no rank in layering_tests.rs"
        );
    }
}

mod self_checks {
    use super::*;

    fn reported(module: &str, source: &str) -> Vec<String> {
        violations_in(module, source).expect("fixture parses and is ranked")
    }

    #[test]
    fn a_production_edge_after_a_test_module_declaration_is_reported() {
        let source = "#[cfg(test)]\nmod tests;\nuse crate::game::X;\n";
        assert_eq!(reported("helix", source), ["helix -> game"]);
    }

    #[test]
    fn an_edge_inside_a_cfg_test_block_is_ignored() {
        let source = "#[cfg(test)]\nmod t {\n    use crate::ui::X;\n}\n";
        assert!(reported("helix", source).is_empty());
        let all_test = "#[cfg(all(test, unix))]\nfn f() { crate::ui::f(); }\n";
        assert!(reported("helix", all_test).is_empty());
    }

    #[test]
    fn a_doc_link_is_not_an_edge() {
        let source = "/// See [`crate::ui::X`].\nfn f() {}\n";
        assert!(reported("helix", source).is_empty());
    }

    #[test]
    fn a_use_group_reports_every_target() {
        let source = "use crate::{game::A, ui::B};\n";
        assert_eq!(reported("helix", source), ["helix -> game", "helix -> ui"]);
    }

    #[test]
    fn an_expression_path_is_an_edge() {
        let source = "fn f() { crate::ui::f(); }\n";
        assert_eq!(reported("helix", source), ["helix -> ui"]);
    }

    #[test]
    fn an_edge_inside_a_macro_body_is_reported() {
        let source = "fn f(x: u8) { assert!(matches!(x, crate::ui::X)); tracing::warn!(\"{}\", crate::game::g()); }\n";
        assert_eq!(reported("helix", source), ["helix -> game", "helix -> ui"]);
    }

    #[test]
    fn a_same_rank_edge_is_reported() {
        assert_eq!(
            reported("ids", "use crate::security::X;\n"),
            ["ids -> security"]
        );
    }

    #[test]
    fn an_unranked_module_is_an_error() {
        assert!(violations_in("mystery", "fn f() {}\n").is_err());
        assert!(violations_in("helix", "use crate::mystery::X;\n").is_err());
    }

    #[test]
    fn a_cfg_test_fn_inside_an_impl_is_ignored() {
        let source = "struct S;\nimpl S {\n    #[cfg(test)]\n    fn t() { crate::ui::f(); }\n    fn p() {}\n}\n";
        assert!(reported("helix", source).is_empty());
    }

    #[test]
    fn an_unparsable_source_fails_loudly() {
        assert!(violations_in("helix", "fn (").is_err());
    }

    #[test]
    fn a_lower_rank_edge_is_fine() {
        assert!(
            reported(
                "ui",
                "use crate::game::X;\nuse crate::grammar::commands::C;\n"
            )
            .is_empty()
        );
    }
}
