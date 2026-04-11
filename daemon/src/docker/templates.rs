// Imports

pub struct Template {
    pub name: &'static str,
    pub image: &'static str,
    pub default_internal_port: u16,
}

// Here are the images that are presets.
// I want dynamic though
pub const TEMPLATES: &[Template] = &[
    Template { name: "python",  image: "python:3.12-slim", default_internal_port: 8000 },
    Template { name: "node",    image: "node:20-slim",     default_internal_port: 3000 },
    Template { name: "rust",    image: "rust:latest",      default_internal_port: 8080 },
    Template { name: "ubuntu",  image: "ubuntu:22.04",     default_internal_port: 22   },
    Template { name: "ubuntu",  image: "ubuntu:24.04",     default_internal_port: 22   },
    Template { name: "alpine",  image: "alpine:latest",    default_internal_port: 22   },
    Template { name: "postgres",image: "postgres:16",      default_internal_port: 5432 },
];

pub fn get_template(name: &str) -> Option<&'static Template> {
    TEMPLATES.iter().find(|t| t.name == name)
}

// note, please optimise this to its peak
// We need performance here, not just static shit