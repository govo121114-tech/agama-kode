use std::path::Path;
use std::fs;

pub struct ProjectScaffold {
    pub name: String,
    pub language: String,
    pub description: String,
}

pub fn create_project(scaffold: &ProjectScaffold) -> Result<(), String> {
    let dir = Path::new(&scaffold.name);
    if dir.exists() {
        return Err(format!("Directory '{}' already exists", scaffold.name));
    }
    fs::create_dir_all(dir).map_err(|e| format!("Cannot create directory: {e}"))?;

    let files = match scaffold.language.to_lowercase().as_str() {
        "rust" | "rs" => rust_template(&scaffold.name),
        "python" | "py" => python_template(&scaffold.name),
        "javascript" | "js" => js_template(&scaffold.name),
        "typescript" | "ts" => ts_template(&scaffold.name),
        "go" => go_template(&scaffold.name),
        "c" => c_template(&scaffold.name),
        "cpp" | "c++" | "cxx" => cpp_template(&scaffold.name),
        "html" => html_template(&scaffold.name),
        _ => {
            let ai = try_ollama_generate(scaffold);
            match ai {
                Ok(files) => files,
                Err(_) => generic_template(&scaffold.name, &scaffold.language),
            }
        }
    };

    for (path, content) in &files {
        let full_path = dir.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("Cannot create directory: {e}"))?;
        }
        fs::write(&full_path, content).map_err(|e| format!("Cannot write file: {e}"))?;
    }

    Ok(())
}

fn try_ollama_generate(scaffold: &ProjectScaffold) -> Result<Vec<(String, String)>, String> {
    let prompt = format!(
        r##"Create a project called "{}" using {} language.
Description: {}

Return ONLY a JSON object where keys are file paths and values are file contents.
Example: {{"src/main.py": "print('hello')", "README.md": "# Project"}}
No explanation, no markdown, just raw JSON."##,
        scaffold.name, scaffold.language, scaffold.description
    );

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("HTTP client error: {e}"))?;

    let body = serde_json::json!({
        "model": "codellama",
        "prompt": prompt,
        "stream": false,
        "temperature": 0.2,
    });

    let resp = client
        .post("http://localhost:11434/api/generate")
        .json(&body)
        .send()
        .map_err(|e| format!("Ollama API error: {e} (is Ollama running?)"))?;

    let json: serde_json::Value = resp
        .json()
        .map_err(|e| format!("Invalid JSON response: {e}"))?;

    let text = json["response"]
        .as_str()
        .ok_or("No 'response' field in Ollama output")?;

    let cleaned = text
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let files: serde_json::Value = serde_json::from_str(cleaned)
        .map_err(|e| format!("Cannot parse generated JSON: {e}\nRaw: {text}"))?;

    let obj = files.as_object().ok_or("Response is not a JSON object")?;
    let mut result = Vec::new();
    for (path, content) in obj {
        let content_str = content.as_str().unwrap_or("");
        result.push((path.clone(), content_str.to_string()));
    }

    if result.is_empty() {
        Err("No files generated".to_string())
    } else {
        Ok(result)
    }
}

fn rust_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("Cargo.toml".into(), format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2021"

[dependencies]
"#, name)),
        ("src/main.rs".into(), format!(
            r#"fn main() {{
    println!("Hello, {}!");
}}
"#, name)),
        (".gitignore".into(), "/target\n".to_string()),
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
    ]
}

fn python_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
        ("requirements.txt".into(), "# Add dependencies here\n".to_string()),
        ("src/__init__.py".into(), "".to_string()),
        ("src/main.py".into(), format!(
            r#"def main():
    print("Hello from {}!")


if __name__ == "__main__":
    main()
"#, name)),
        (".gitignore".into(), "__pycache__/\n*.pyc\n.venv/\n".to_string()),
    ]
}

fn js_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("package.json".into(), format!(
            r#"{{
  "name": "{}",
  "version": "1.0.0",
  "main": "src/index.js",
  "scripts": {{
    "start": "node src/index.js"
  }}
}}
"#, name)),
        ("src/index.js".into(), format!(
            r#"console.log("Hello from {}!");
"#, name)),
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
        (".gitignore".into(), "node_modules/\n".to_string()),
    ]
}

fn ts_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("package.json".into(), format!(
            r#"{{
  "name": "{}",
  "version": "1.0.0",
  "main": "dist/index.js",
  "scripts": {{
    "build": "tsc",
    "start": "node dist/index.js"
  }}
}}
"#, name)),
        ("tsconfig.json".into(), r#"{
  "compilerOptions": {
    "target": "ES2020",
    "module": "commonjs",
    "outDir": "./dist",
    "strict": true
  },
  "include": ["src"]
}
"#.to_string()),
        ("src/index.ts".into(), format!(
            r#"console.log("Hello from {}!");
"#, name)),
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
        (".gitignore".into(), "node_modules/\ndist/\n".to_string()),
    ]
}

fn go_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("go.mod".into(), format!(
            r#"module {}

go 1.21
"#, name)),
        ("main.go".into(), format!(
            r#"package main

import "fmt"

func main() {{
    fmt.Println("Hello from {}!")
}}
"#, name)),
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
    ]
}

fn c_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("Makefile".into(), r#"CC = gcc
CFLAGS = -Wall -Wextra -O2

all: main

main: main.c
	$(CC) $(CFLAGS) -o main main.c

clean:
	rm -f main
"#.to_string()),
        ("main.c".into(), format!(
            r#"#include <stdio.h>

int main() {{
    printf("Hello from {}!\\n");
    return 0;
}}
"#, name)),
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
    ]
}

fn cpp_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("Makefile".into(), r#"CXX = g++
CXXFLAGS = -Wall -Wextra -O2 -std=c++17

all: main

main: main.cpp
	$(CXX) $(CXXFLAGS) -o main main.cpp

clean:
	rm -f main
"#.to_string()),
        ("main.cpp".into(), format!(
            r#"#include <iostream>

int main() {{
    std::cout << "Hello from {}!" << std::endl;
    return 0;
}}
"#, name)),
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
    ]
}

fn html_template(name: &str) -> Vec<(String, String)> {
    vec![
        ("index.html".into(), format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{}</title>
    <link rel="stylesheet" href="style.css">
</head>
<body>
    <h1>Hello from {}!</h1>
    <script src="script.js"></script>
</body>
</html>
"#, name, name)),
        ("style.css".into(), "body {\n    font-family: sans-serif;\n    margin: 2rem;\n}\n".to_string()),
        ("script.js".into(), format!("console.log(\"Hello from {}!\");\n", name)),
        ("README.md".into(), format!("# {}\n\n{}", name, desc_placeholder())),
    ]
}

fn generic_template(name: &str, lang: &str) -> Vec<(String, String)> {
    vec![
        ("README.md".into(), format!(
            r#"# {}

Language: {}

{}
"#, name, lang, desc_placeholder())),
        ("src/main.txt".into(), format!("// {} project\n", name)),
    ]
}

fn desc_placeholder() -> String {
    "Your project description here.".to_string()
}
