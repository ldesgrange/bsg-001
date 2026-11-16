use std::fs;
use std::path::Path;

use syn::visit::{self, Visit};

#[derive(Clone, Copy)]
pub enum Layer {
    Domain,
    Application,
    Adapters,
}

struct Checker {
    layer: Layer,
    violations: Vec<String>,
}

impl Checker {
    fn inspect(&mut self, path: &[String]) {
        let forbidden = match self.layer {
            Layer::Domain => &["application", "adapters", "btleplug", "clap", "tokio"][..],
            Layer::Application => &["adapters", "btleplug", "clap", "tokio"][..],
            Layer::Adapters => &[],
        };
        let first = path.iter().find(|part| !["crate", "self", "super"].contains(&part.as_str()));
        if first.is_some_and(|part| forbidden.contains(&part.as_str())) {
            self.violations.push(path.join("::"));
        }
    }

    fn use_tree(&mut self, tree: &syn::UseTree, prefix: &mut Vec<String>) {
        match tree {
            syn::UseTree::Path(path) => {
                prefix.push(path.ident.to_string());
                self.use_tree(&path.tree, prefix);
                prefix.pop();
            }
            syn::UseTree::Name(name) => {
                prefix.push(name.ident.to_string());
                self.inspect(prefix);
                prefix.pop();
            }
            syn::UseTree::Rename(rename) => {
                prefix.push(rename.ident.to_string());
                self.inspect(prefix);
                prefix.pop();
            }
            syn::UseTree::Glob(_) => self.inspect(prefix),
            syn::UseTree::Group(group) => {
                for item in &group.items {
                    self.use_tree(item, prefix);
                }
            }
        }
    }
}

impl<'ast> Visit<'ast> for Checker {
    fn visit_item_extern_crate(&mut self, node: &'ast syn::ItemExternCrate) {
        self.inspect(&[node.ident.to_string()]);
    }

    fn visit_item_use(&mut self, node: &'ast syn::ItemUse) {
        self.use_tree(&node.tree, &mut Vec::new());
    }

    fn visit_path(&mut self, node: &'ast syn::Path) {
        self.inspect(&node.segments.iter().map(|segment| segment.ident.to_string()).collect::<Vec<_>>());
        visit::visit_path(self, node);
    }
}

pub fn check_source(layer: Layer, source: &str) -> Result<Vec<String>, syn::Error> {
    let file = syn::parse_file(source)?;
    let mut checker = Checker { layer, violations: Vec::new() };
    checker.visit_file(&file);
    Ok(checker.violations)
}

pub fn check_tree(root: &Path) -> Result<(), String> {
    for (directory, layer) in [("domain", Layer::Domain), ("application", Layer::Application), ("adapters", Layer::Adapters)] {
        scan_directory(&root.join(directory), layer)?;
    }
    Ok(())
}

fn scan_directory(directory: &Path, layer: Layer) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| format!("{}: {error}", directory.display()))? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.is_dir() {
            scan_directory(&path, layer)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source = fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
            let violations = check_source(layer, &source).map_err(|error| format!("{}: {error}", path.display()))?;
            if !violations.is_empty() {
                return Err(format!("{}: forbidden references: {}", path.display(), violations.join(", ")));
            }
        }
    }
    Ok(())
}
