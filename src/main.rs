use anyhow::Result;
use jsonrpc_stdio_server::jsonrpc_core::{
    Error, IoHandler, Params, Result as JsonRpcResult, Value,
};
use serde_json::{Map, json};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tree_sitter::{Parser, Query, QueryCursor};
use walkdir::WalkDir;

/// Estrutura principal que contém a lógica do servidor MCP
struct RpcHandler;

impl RpcHandler {
    fn new() -> Self {
        RpcHandler
    }

    /// Implementa o método `initialize` do protocolo MCP
    /// Retorna as capacidades do servidor
    fn initialize(&self, _params: Params) -> JsonRpcResult<Value> {
        let capabilities = json!({
            "protocolVersion": "2025-03-26",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "workspace-context-server",
                "version": "1.0.0"
            }
        });
        Ok(capabilities)
    }

    /// Implementa o método `list_tools` do protocolo MCP
    /// Retorna a definição da nossa única ferramenta
    fn list_tools(&self, _params: Params) -> JsonRpcResult<Value> {
        let tools = json!({
            "tools": [
                {
                    "name": "get_workspace_context",
                    "description": "Analisa a estrutura do workspace atual (ficheiros e símbolos de código) e retorna-a como contexto. Otimizado para evitar excesso de tokens.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "workspace_path": {
                                "type": "string",
                                "description": "Caminho opcional para o diretório do workspace a analisar. Se não fornecido, usa o diretório atual ou diretório pai se estiver em workspace-context."
                            },
                            "max_files": {
                                "type": "number",
                                "description": "Número máximo de arquivos a analisar (padrão: 200)",
                                "default": 200
                            },
                            "max_symbols_per_file": {
                                "type": "number",
                                "description": "Número máximo de símbolos a mostrar por arquivo (padrão: 10)",
                                "default": 10
                            },
                            "max_depth": {
                                "type": "number",
                                "description": "Profundidade máxima de recursão em diretórios (padrão: 8)",
                                "default": 8
                            },
                            "summary_only": {
                                "type": "boolean",
                                "description": "Se true, retorna apenas um resumo estatístico sem símbolos detalhados (padrão: false)",
                                "default": false
                            }
                        },
                        "additionalProperties": false
                    }
                }
            ]
        });

        Ok(tools)
    }

    /// Implementa o método `execute_tool` do protocolo MCP
    /// Retorna uma representação hierárquica e bem formatada do workspace
    fn execute_tool(&self, params: Params) -> JsonRpcResult<Value> {
        // Parse dos parâmetros
        let params_map: Map<String, Value> = match params {
            Params::Map(map) => map,
            _ => return Err(Error::invalid_params("Expected object parameters")),
        };

        let tool_name = params_map
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::invalid_params("Missing tool name"))?;

        match tool_name {
            "get_workspace_context" => {
                let arguments = params_map.get("arguments");

                // Verificar se foi especificado um workspace_path nos argumentos
                let workspace_dir = if let Some(workspace_path) = arguments
                    .and_then(|args| args.get("workspace_path"))
                    .and_then(|v| v.as_str())
                {
                    PathBuf::from(workspace_path)
                } else {
                    // Tentar obter workspace_path da variável de ambiente
                    if let Ok(env_workspace) = std::env::var("WORKSPACE_PATH") {
                        PathBuf::from(env_workspace)
                    } else {
                        // Fallback: usar o diretório pai do diretório atual se estivermos em workspace-context
                        let current_dir =
                            std::env::current_dir().map_err(|_| Error::internal_error())?;
                        if current_dir.file_name().and_then(|n| n.to_str())
                            == Some("workspace-context")
                        {
                            current_dir.parent().unwrap_or(&current_dir).to_path_buf()
                        } else {
                            current_dir
                        }
                    }
                };

                // Extrair parâmetros configuráveis
                let max_files = arguments
                    .and_then(|args| args.get("max_files"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(200) as usize;

                let max_symbols_per_file = arguments
                    .and_then(|args| args.get("max_symbols_per_file"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(10) as usize;

                let max_depth = arguments
                    .and_then(|args| args.get("max_depth"))
                    .and_then(|v| v.as_u64())
                    .unwrap_or(8) as usize;

                let summary_only = arguments
                    .and_then(|args| args.get("summary_only"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                // Verificar se o diretório existe
                if !workspace_dir.exists() {
                    return Err(Error::invalid_params(&format!(
                        "Workspace directory does not exist: {}",
                        workspace_dir.display()
                    )));
                }

                // Coletar ficheiros do projeto com limites configuráveis
                let files = collect_project_files_with_limits(&workspace_dir, max_files, max_depth);

                // Construir a representação hierárquica
                let context = if summary_only {
                    format_workspace_summary(&workspace_dir, &files)
                } else {
                    format_workspace_tree_with_limits(&workspace_dir, &files, max_symbols_per_file)
                };

                let result = json!({
                    "content": [
                        {
                            "type": "text",
                            "text": context
                        }
                    ]
                });
                Ok(result)
            }
            _ => Err(Error::method_not_found()),
        }
    }
}

/// Formata a saída do workspace como uma árvore hierárquica legível
fn format_workspace_tree_with_limits(
    root_dir: &Path,
    files: &[PathBuf],
    max_symbols_per_file: usize,
) -> String {
    use std::collections::BTreeMap;

    let mut tree = BTreeMap::new();
    let mut total_symbols = 0;
    let mut files_with_symbols = 0;

    // Construir a estrutura da árvore
    for file in files {
        if let Ok(relative_path) = file.strip_prefix(root_dir) {
            let components: Vec<&std::ffi::OsStr> = relative_path.iter().collect();
            insert_into_tree(&mut tree, &components, file);

            // Contar símbolos para estatísticas
            if let Ok(symbols) = extract_symbols_from_file(file) {
                if !symbols.is_empty() {
                    total_symbols += symbols.len();
                    files_with_symbols += 1;
                }
            }
        }
    }

    // Construir a string formatada
    let mut result = String::new();
    result.push_str("📁 Workspace Analysis\n");
    result.push_str("══════════════════════════════════\n\n");

    format_tree_node_with_limits(&tree, &mut result, "", true, max_symbols_per_file);

    // Adicionar estatísticas detalhadas no final
    result.push_str(&format!(
        "\n📊 Summary:\n\
         • {} files analyzed (limited for performance)\n\
         • {} files contain symbols\n\
         • {} total symbols found\n\
         • Max {} symbols shown per file\n\
         • Root: {}\n",
        files.len(),
        files_with_symbols,
        total_symbols,
        max_symbols_per_file,
        root_dir.display()
    ));

    result
}

fn format_workspace_summary(root_dir: &Path, files: &[PathBuf]) -> String {
    let mut result = String::new();
    result.push_str("📁 Workspace Summary\n");
    result.push_str("═══════════════════\n\n");

    // Agrupar arquivos por extensão
    let mut extensions = BTreeMap::new();
    let mut total_symbols = 0;
    let mut files_with_symbols = 0;

    for file in files {
        if let Some(ext) = file.extension() {
            let ext_str = ext.to_string_lossy().to_lowercase();
            *extensions.entry(ext_str).or_insert(0) += 1;
        }

        // Contar símbolos
        if let Ok(symbols) = extract_symbols_from_file(file) {
            if !symbols.is_empty() {
                total_symbols += symbols.len();
                files_with_symbols += 1;
            }
        }
    }

    result.push_str("📂 File Types:\n");
    for (ext, count) in extensions.iter() {
        result.push_str(&format!("  • .{}: {} files\n", ext, count));
    }

    result.push_str(&format!(
        "\n📊 Statistics:\n\
         • {} total files\n\
         • {} files with symbols\n\
         • {} total symbols\n\
         • Root: {}\n",
        files.len(),
        files_with_symbols,
        total_symbols,
        root_dir.display()
    ));

    result
}

/// Estrutura para representar um nó na árvore
#[derive(Debug)]
struct TreeNode {
    file_path: Option<PathBuf>,
    children: BTreeMap<std::ffi::OsString, TreeNode>,
}

impl TreeNode {
    fn new() -> Self {
        TreeNode {
            file_path: None,
            children: BTreeMap::new(),
        }
    }
}

/// Insere um ficheiro na estrutura da árvore
fn insert_into_tree(
    tree: &mut BTreeMap<std::ffi::OsString, TreeNode>,
    components: &[&std::ffi::OsStr],
    full_path: &Path,
) {
    if components.is_empty() {
        return;
    }

    let component = components[0].to_os_string();
    let node = tree.entry(component).or_insert_with(TreeNode::new);

    if components.len() == 1 {
        // É um ficheiro
        node.file_path = Some(full_path.to_path_buf());
    } else {
        // É uma diretoria, continuar recursivamente
        insert_into_tree(&mut node.children, &components[1..], full_path);
    }
}

/// Formata um nó da árvore recursivamente
fn format_tree_node_with_limits(
    tree: &BTreeMap<std::ffi::OsString, TreeNode>,
    result: &mut String,
    prefix: &str,
    is_root: bool,
    max_symbols_per_file: usize,
) {
    const MAX_DIRS_TO_SHOW: usize = 50; // Limite de diretórios a mostrar

    let entries: Vec<_> = tree.iter().take(MAX_DIRS_TO_SHOW).collect();

    for (i, (name, node)) in entries.iter().enumerate() {
        let is_last = i == entries.len() - 1;
        let current_prefix = if is_root {
            ""
        } else if is_last {
            "└── "
        } else {
            "├── "
        };

        let name_str = name.to_string_lossy();

        if let Some(file_path) = &node.file_path {
            // É um ficheiro - mostrar símbolos limitados
            result.push_str(&format!("{}{}{}\n", prefix, current_prefix, name_str));

            // Extrair e mostrar símbolos (limitados)
            match extract_symbols_from_file(file_path) {
                Ok(symbols) => {
                    let symbols_prefix = if is_root {
                        ""
                    } else if is_last {
                        "    "
                    } else {
                        "│   "
                    };

                    if !symbols.is_empty() {
                        let symbols_to_show = symbols.iter().take(max_symbols_per_file);
                        let total_symbols = symbols.len();

                        for (j, symbol) in symbols_to_show.enumerate() {
                            let symbol_marker = if j == max_symbols_per_file - 1
                                || (j == total_symbols - 1 && node.children.is_empty())
                            {
                                "└─ "
                            } else {
                                "├─ "
                            };
                            result.push_str(&format!(
                                "{}{}  {}{}\n",
                                prefix,
                                symbols_prefix,
                                symbol_marker,
                                format_symbol(&symbol)
                            ));
                        }

                        // Mostrar se há mais símbolos
                        if total_symbols > max_symbols_per_file {
                            result.push_str(&format!(
                                "{}{}  └─ ... ({} more symbols)\n",
                                prefix,
                                symbols_prefix,
                                total_symbols - max_symbols_per_file
                            ));
                        }
                    }
                }
                Err(_) => {
                    let error_prefix = if is_root {
                        ""
                    } else if is_last {
                        "    "
                    } else {
                        "│   "
                    };
                    result.push_str(&format!(
                        "{}{}  └─ ⚠️  (parsing error)\n",
                        prefix, error_prefix
                    ));
                }
            }
        } else {
            // É uma diretoria
            result.push_str(&format!("{}{}📁 {}/\n", prefix, current_prefix, name_str));
        }

        // Processar filhos (subdiretórias e ficheiros)
        if !node.children.is_empty() {
            let child_prefix = if is_root {
                ""
            } else if is_last {
                &format!("{}    ", prefix)
            } else {
                &format!("{}│   ", prefix)
            };
            format_tree_node_with_limits(
                &node.children,
                result,
                child_prefix,
                false,
                max_symbols_per_file,
            );
        }
    }

    // Mostrar se há mais diretórios/arquivos
    if tree.len() > MAX_DIRS_TO_SHOW {
        result.push_str(&format!(
            "{}... ({} more items not shown)\n",
            prefix,
            tree.len() - MAX_DIRS_TO_SHOW
        ));
    }
}

/// Formata um símbolo com ícones apropriados
fn format_symbol(symbol: &str) -> String {
    if symbol.starts_with("fn ") || symbol.contains("function") {
        format!("🔧 {}", symbol)
    } else if symbol.starts_with("struct ") || symbol.starts_with("class ") {
        format!("🏗️  {}", symbol)
    } else if symbol.starts_with("enum ") {
        format!("🔢 {}", symbol)
    } else if symbol.starts_with("trait ") || symbol.starts_with("interface ") {
        format!("🎭 {}", symbol)
    } else if symbol.starts_with("impl ") {
        format!("⚙️  {}", symbol)
    } else if symbol.starts_with("mod ") || symbol.starts_with("module ") {
        format!("📦 {}", symbol)
    } else if symbol.starts_with("const ") || symbol.starts_with("static ") {
        format!("📌 {}", symbol)
    } else if symbol.starts_with("let ") || symbol.starts_with("var ") {
        format!("📊 {}", symbol)
    } else {
        format!("🔍 {}", symbol)
    }
}

/// Coleta ficheiros de código fonte do projeto, ignorando diretorias e ficheiros irrelevantes
fn collect_project_files_with_limits(
    path: &Path,
    max_files: usize,
    max_depth: usize,
) -> Vec<PathBuf> {
    let mut files = Vec::new();

    // Diretorias a ignorar
    let ignored_dirs = [
        ".git",
        "target",
        "node_modules",
        ".next",
        "dist",
        "build",
        "coverage",
        ".nyc_output",
        "vendor",
        "__pycache__",
        ".pytest_cache",
        ".vscode",
        ".idea",
        "tmp",
        "temp",
        ".cache",
        ".DS_Store",
    ];

    // Priorizar extensões principais de código
    let priority_extensions = ["rs", "js", "ts", "tsx", "jsx", "py", "go", "java"];
    let secondary_extensions = [
        "c",
        "cpp",
        "h",
        "hpp",
        "cs",
        "php",
        "rb",
        "kt",
        "swift",
        "scala",
        "sh",
        "bash",
        "zsh",
        "sql",
        "vue",
        "svelte",
        "md",
        "yaml",
        "yml",
        "json",
        "toml",
        "xml",
        "makefile",
        "dockerfile",
    ];

    // Ficheiros específicos a ignorar (padrões)
    let ignored_file_patterns = [
        ".lock",
        ".log",
        ".tmp",
        ".cache",
        ".DS_Store",
        "thumbs.db",
        ".min.js",
        ".min.css",
        ".bundle.js",
        ".bundle.css",
        "package-lock.json",
        "yarn.lock",
        "Cargo.lock",
    ];

    let mut priority_files = Vec::new();
    let mut secondary_files = Vec::new();

    for entry in WalkDir::new(path)
        .max_depth(max_depth) // Usar profundidade configurável
        .into_iter()
        .filter_entry(|e| {
            // Filtrar diretorias ignoradas
            if e.file_type().is_dir() {
                let dir_name = e.file_name().to_string_lossy();
                !ignored_dirs.iter().any(|&ignored| dir_name == ignored)
            } else {
                true
            }
        })
    {
        if let Ok(entry) = entry {
            let path = entry.path();

            // Parar se já temos muitos arquivos
            if priority_files.len() + secondary_files.len() >= max_files {
                break;
            }

            // Apenas processar ficheiros (não diretorias)
            if path.is_file() {
                // Verificar se tem extensão válida
                if let Some(extension) = path.extension() {
                    let ext = extension.to_string_lossy().to_lowercase();

                    // Verificar se não é um ficheiro a ser ignorado
                    let file_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_lowercase();

                    let should_ignore = ignored_file_patterns
                        .iter()
                        .any(|&pattern| file_name.contains(pattern));

                    if !should_ignore {
                        if priority_extensions.contains(&ext.as_str()) {
                            priority_files.push(path.to_path_buf());
                        } else if secondary_extensions.contains(&ext.as_str()) {
                            secondary_files.push(path.to_path_buf());
                        }
                    }
                }
                // Incluir ficheiros sem extensão mas com nomes específicos
                else if let Some(file_name) = path.file_name() {
                    let name = file_name.to_string_lossy().to_lowercase();
                    if name == "makefile" || name == "dockerfile" || name == "rakefile" {
                        secondary_files.push(path.to_path_buf());
                    }
                }
            }
        }
    }

    // Combinar arquivos priorizando os principais
    priority_files.sort();
    secondary_files.sort();

    files.extend(priority_files);

    // Adicionar arquivos secundários até o limite
    let remaining_capacity = max_files.saturating_sub(files.len());
    files.extend(secondary_files.into_iter().take(remaining_capacity));

    files
}

/// Extrai símbolos de código de um ficheiro usando tree-sitter
fn extract_symbols_from_file(file_path: &Path) -> Result<Vec<String>, anyhow::Error> {
    // Ler o conteúdo do ficheiro
    let content = fs::read_to_string(file_path)?;

    // Determinar a linguagem pela extensão
    let language = match file_path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => Some(tree_sitter_rust::language()),
        Some("js") | Some("jsx") => Some(tree_sitter_javascript::language()),
        Some("ts") | Some("tsx") => Some(tree_sitter_typescript::language_typescript()),
        Some("py") => Some(tree_sitter_python::language()),
        _ => None,
    };

    let language = match language {
        Some(lang) => lang,
        None => return Ok(vec![]), // Linguagem não suportada, retornar lista vazia
    };

    // Criar parser e definir a linguagem
    let mut parser = Parser::new();
    parser.set_language(language)?;

    // Parse do código
    let tree = parser
        .parse(&content, None)
        .ok_or_else(|| anyhow::anyhow!("Falha ao fazer parse do ficheiro"))?;

    // Definir queries para extrair símbolos baseado na linguagem
    let query_source = match file_path.extension().and_then(|ext| ext.to_str()) {
        Some("rs") => get_rust_query(),
        Some("js") | Some("jsx") => get_javascript_query(),
        Some("ts") | Some("tsx") => get_typescript_query(),
        Some("py") => get_python_query(),
        _ => return Ok(vec![]),
    };

    // Criar e executar a query
    let query = Query::new(language, &query_source)?;
    let mut cursor = QueryCursor::new();
    let matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut symbols = Vec::new();

    for mat in matches {
        for capture in mat.captures {
            let node = capture.node;
            let capture_name = &query.capture_names()[capture.index as usize];

            if let Ok(symbol_name) = node.utf8_text(content.as_bytes()) {
                // Adicionar prefixo baseado no tipo de símbolo
                let formatted_symbol = match capture_name {
                    name if name.contains("function") => format!("fn {}", symbol_name),
                    name if name.contains("struct") => format!("struct {}", symbol_name),
                    name if name.contains("class") => format!("class {}", symbol_name),
                    name if name.contains("enum") => format!("enum {}", symbol_name),
                    name if name.contains("trait") => format!("trait {}", symbol_name),
                    name if name.contains("interface") => format!("interface {}", symbol_name),
                    name if name.contains("type") => format!("type {}", symbol_name),
                    name if name.contains("impl") => format!("impl {}", symbol_name),
                    name if name.contains("mod") => format!("mod {}", symbol_name),
                    name if name.contains("const") => format!("const {}", symbol_name),
                    name if name.contains("static") => format!("static {}", symbol_name),
                    name if name.contains("method") => format!("method {}", symbol_name),
                    name if name.contains("variable") => format!("var {}", symbol_name),
                    name if name.contains("import") => format!("import {}", symbol_name),
                    _ => symbol_name.to_string(),
                };
                symbols.push(formatted_symbol);
            }
        }
    }

    // Remover duplicados e ordenar
    symbols.sort();
    symbols.dedup();

    Ok(symbols)
}

/// Query para extrair símbolos do Rust
fn get_rust_query() -> String {
    r#"
    (function_item
      name: (identifier) @function.name)

    (struct_item
      name: (type_identifier) @struct.name)

    (enum_item
      name: (type_identifier) @enum.name)

    (trait_item
      name: (type_identifier) @trait.name)

    (impl_item
      type: (type_identifier) @impl.name)

    (mod_item
      name: (identifier) @mod.name)

    (const_item
      name: (identifier) @const.name)

    (static_item
      name: (identifier) @static.name)
    "#
    .to_string()
}

/// Query para extrair símbolos do JavaScript
fn get_javascript_query() -> String {
    r#"
    (function_declaration
      name: (identifier) @function.name)

    (class_declaration
      name: (identifier) @class.name)

    (method_definition
      name: (property_identifier) @method.name)

    (variable_declarator
      name: (identifier) @variable.name)
    "#
    .to_string()
}

/// Query para extrair símbolos do TypeScript
fn get_typescript_query() -> String {
    r#"
    (function_declaration
      name: (identifier) @function.name)

    (class_declaration
      name: (type_identifier) @class.name)

    (interface_declaration
      name: (type_identifier) @interface.name)

    (type_alias_declaration
      name: (type_identifier) @type.name)

    (enum_declaration
      name: (identifier) @enum.name)

    (method_definition
      name: (property_identifier) @method.name)

    (variable_declarator
      name: (identifier) @variable.name)
    "#
    .to_string()
}

/// Query para extrair símbolos do Python
fn get_python_query() -> String {
    r#"
    (function_definition
      name: (identifier) @function.name)

    (class_definition
      name: (identifier) @class.name)

    (assignment
      left: (identifier) @variable.name)

    (import_statement
      name: (dotted_name
        (identifier) @import.name))

    (import_from_statement
      name: (dotted_name
        (identifier) @import.name))
    "#
    .to_string()
}

fn main() -> Result<()> {
    // Print startup information to stderr so it doesn't interfere with JSON-RPC
    eprintln!("🚀 MCP Workspace Context Server");
    eprintln!("═══════════════════════════════");
    eprintln!("📡 Protocol: JSON-RPC over stdin/stdout");
    eprintln!("🔧 Tools available:");
    eprintln!("   - get_workspace_context: Analyze workspace structure and code symbols");
    eprintln!("🏗️  Supported languages: Rust, JavaScript, TypeScript, Python");
    eprintln!(
        "📁 Working directory: {:?}",
        std::env::current_dir().unwrap_or_default()
    );
    eprintln!("✅ Server ready - waiting for MCP client connection...");
    eprintln!("");

    // Print Zed configuration example
    eprintln!("📋 To use with Zed, add this to your settings.json:");
    eprintln!("{{");
    eprintln!("  \"context_servers\": {{");
    eprintln!("    \"workspace-context\": {{");
    eprintln!("      \"source\": \"custom\",");
    eprintln!(
        "      \"command\": \"{}\",",
        std::env::current_exe().unwrap_or_default().display()
    );
    eprintln!("      \"args\": [],");
    eprintln!("      \"env\": {{}}");
    eprintln!("    }}");
    eprintln!("  }}");
    eprintln!("}}");
    eprintln!("════════════════════════════════════════════════════════════");
    eprintln!("");

    // Criar o handler RPC
    let rpc_handler = RpcHandler::new();

    // Configurar o servidor de IO
    let mut io = IoHandler::new();

    // Registar o método initialize
    io.add_sync_method("initialize", move |params| rpc_handler.initialize(params));

    // Registar o método list_tools
    let rpc_handler_tools = RpcHandler::new();
    io.add_sync_method("tools/list", move |params| {
        rpc_handler_tools.list_tools(params)
    });

    // Registar o método execute_tool
    let rpc_handler_clone = RpcHandler::new();
    io.add_sync_method("tools/call", move |params| {
        rpc_handler_clone.execute_tool(params)
    });

    // Criar reader/writer para stdin/stdout
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = BufReader::new(stdin);

    // Loop principal do servidor
    eprintln!("🔄 Starting JSON-RPC message loop...");
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Log incoming request to stderr (for debugging)
        eprintln!("📨 Received request: {}", line.trim());

        // Parse da requisição JSON-RPC
        match serde_json::from_str::<Value>(&line) {
            Ok(request) => {
                // Log method name if available
                if let Some(method) = request.get("method").and_then(|m| m.as_str()) {
                    eprintln!("🎯 Processing method: {}", method);
                }

                // Processar a requisição
                let response = io.handle_request_sync(&line);

                if let Some(response_str) = response {
                    eprintln!(
                        "📤 Sending response: {}",
                        response_str.chars().take(100).collect::<String>() + "..."
                    );
                    writeln!(stdout, "{}", response_str)?;
                    stdout.flush()?;
                }
            }
            Err(parse_error) => {
                eprintln!("❌ JSON parse error: {}", parse_error);
                // Erro de parsing - retornar erro JSON-RPC
                let error_response = json!({
                    "jsonrpc": "2.0",
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    },
                    "id": null
                });
                writeln!(stdout, "{}", error_response)?;
                stdout.flush()?;
            }
        }
    }

    eprintln!("🔚 Server shutting down...");

    Ok(())
}
