use std::path::PathBuf;

pub struct ModuleResolver {
    pub(crate) templates: Vec<String>,
}

impl ModuleResolver {
    pub fn new(templates: Vec<String>) -> Self {
        Self { templates }
    }
    pub fn dotted_to_path(name: &str) -> String {
        name.replace('.', "/")
    }
    pub fn resolve(&self, module_name: &str) -> Option<PathBuf> {
        let mod_path = Self::dotted_to_path(module_name);
        for t in &self.templates {
            let candidate = t.replace('?', &mod_path);
            let p = PathBuf::from(&candidate);
            if p.exists() {
                return Some(p);
            }
        }
        None
    }
}
