//! P1-8：插件命令名一致性断言
//!
//! 每个插件手写两份清单——`tauri::generate_handler![...]`（运行时路由）与
//! `command_names()`（O(1) 路由表）——二者一旦漂移，IPC 会被判定为
//! "未知命令"。这里同时用「源码解析」与「真实 PluginManager」两条路径
//! 断言它们永远一致。
//!
//! 放在 `tests/` 而非 `src/plugin.rs`：构建脚本通过
//! `cargo:rustc-link-arg-tests` 把 Common-Controls 6 清单链接进集成测试，
//! 实例化插件才会引入 comctl32!TaskDialogIndirect；`--lib` 单元测试拿不到
//! 该清单，链接进来会以 STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139) 启动失败。

use dev_tools_lib::plugin::{PluginManager, ToolPlugin};
use dev_tools_lib::{env, java, jetbrains, mysql, node, postgresql, python, software};
use dev_tools_lib::{global_invoke_handler, GLOBAL_COMMANDS};
use std::collections::HashSet;

const PLUGIN_SOURCES: [(&str, &str); 8] = [
    ("mysql", include_str!("../src/mysql/plugin.rs")),
    ("postgresql", include_str!("../src/postgresql/plugin.rs")),
    ("python", include_str!("../src/python/plugin.rs")),
    ("node", include_str!("../src/node/plugin.rs")),
    ("jetbrains", include_str!("../src/jetbrains/plugin.rs")),
    ("java", include_str!("../src/java/plugin.rs")),
    ("env", include_str!("../src/env/plugin.rs")),
    ("software", include_str!("../src/software/plugin.rs")),
];

const LIB_SOURCE: &str = include_str!("../src/lib.rs");

/// 按方括号配对截取 `marker` 所指位置开始的方括号内容（不含最外层 `[]`）。
fn bracket_body<'a>(src: &'a str, marker: &str, from: usize) -> &'a str {
    let rel = src[from..]
        .find(marker)
        .unwrap_or_else(|| panic!("找不到 {}", marker));
    let open = from + rel + marker.len() - 1;
    assert_eq!(src.as_bytes()[open], b'[', "标记末字符必须是 '['");
    let mut depth = 0usize;
    for (i, ch) in src[open..].char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return &src[open + 1..open + i];
                }
            }
            _ => {}
        }
    }
    panic!("方括号未闭合: {}", marker)
}

/// 把 `commands::detect_mysql` / `is_running_as_admin` 统一成最后一段。
fn last_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

/// 解析 `tauri::generate_handler![a::b, c::d]` → ["b", "d"]
fn generate_handler_names(src: &str, from: usize) -> Vec<String> {
    bracket_body(src, "generate_handler![", from)
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| last_segment(s).to_string())
        .collect()
}

/// 提取某个方括号块里的全部字符串字面量。
fn quoted_items(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(q) = rest.find('"') {
        rest = &rest[q + 1..];
        let end = rest.find('"').expect("字符串字面量未闭合");
        out.push(rest[..end].to_string());
        rest = &rest[end + 1..];
    }
    out
}

/// 解析 `fn command_names(&self) -> &'static [&'static str] { &[ "a", .. ] }`
fn declared_command_names(src: &str) -> Vec<String> {
    let at = src
        .find("fn command_names")
        .expect("插件未实现 command_names()");
    quoted_items(bracket_body(src, "&[", at))
}

/// 解析 `pub const GLOBAL_COMMANDS: &[&str] = &[ "a", .. ];`
fn global_commands_declared(src: &str) -> Vec<String> {
    let at = src.find("GLOBAL_COMMANDS").expect("找不到 GLOBAL_COMMANDS");
    quoted_items(bracket_body(src, "= &[", at))
}

fn manager() -> PluginManager {
    let mut m = PluginManager::new();
    m.register(Box::new(mysql::MySqlPlugin));
    m.register(Box::new(postgresql::PostgresqlPlugin));
    m.register(Box::new(python::PythonPlugin));
    m.register(Box::new(node::NodePlugin));
    m.register(Box::new(jetbrains::JetBrainsPlugin));
    m.register(Box::new(java::JavaPlugin));
    m.register(Box::new(env::EnvPlugin));
    m.register(Box::new(software::SoftwarePlugin));
    m
}

fn plugin_objects() -> Vec<&'static dyn ToolPlugin> {
    vec![
        &mysql::MySqlPlugin,
        &postgresql::PostgresqlPlugin,
        &python::PythonPlugin,
        &node::NodePlugin,
        &jetbrains::JetBrainsPlugin,
        &java::JavaPlugin,
        &env::EnvPlugin,
        &software::SoftwarePlugin,
    ]
}

fn sorted(v: Vec<String>) -> Vec<String> {
    let mut v = v;
    v.sort();
    v
}

#[test]
fn command_names_match_generate_handler_for_every_plugin() {
    for (id, src) in PLUGIN_SOURCES {
        let declared = sorted(declared_command_names(src));
        let handler_at = src
            .find("fn invoke_handler")
            .unwrap_or_else(|| panic!("{} 未实现 invoke_handler()", id));
        let generated = sorted(generate_handler_names(src, handler_at));
        assert_eq!(
            declared, generated,
            "插件 {} 的 command_names() 与 generate_handler! 不一致",
            id
        );
    }
}

#[test]
fn global_commands_match_global_generate_handler() {
    let declared = global_commands_declared(LIB_SOURCE);
    let at = LIB_SOURCE
        .find("fn global_invoke_handler")
        .expect("找不到 global_invoke_handler");
    let generated = generate_handler_names(LIB_SOURCE, at);
    assert_eq!(
        sorted(declared.clone()),
        sorted(generated),
        "GLOBAL_COMMANDS 与 global_invoke_handler 的 generate_handler! 不一致"
    );
    assert_eq!(
        sorted(declared),
        sorted(GLOBAL_COMMANDS.iter().map(|s| s.to_string()).collect()),
        "源码里的 GLOBAL_COMMANDS 与运行时常量不一致"
    );
}

#[test]
fn global_invoke_handler_is_constructible() {
    let _handler = global_invoke_handler();
}

#[test]
fn command_names_are_globally_unique() {
    let mut plugin_names: HashSet<&str> = HashSet::new();
    for plugin in plugin_objects() {
        for name in plugin.command_names() {
            assert!(plugin_names.insert(name), "插件间命令名重复: {}", name);
        }
    }

    let mut global_names: HashSet<&str> = HashSet::new();
    for name in GLOBAL_COMMANDS {
        assert!(global_names.insert(name), "全局命令名重复: {}", name);
        assert!(
            !plugin_names.contains(name),
            "全局命令与插件命令冲突: {}",
            name
        );
    }

    let (route, _) = manager().build_router();
    assert_eq!(
        route.len(),
        plugin_names.len(),
        "路由表条目数与声明的插件命令总数不符（存在重复键）"
    );
}

#[test]
fn router_resolves_every_declared_command() {
    let (route, handlers) = manager().build_router();
    let mut checked = 0usize;
    for plugin in plugin_objects() {
        for name in plugin.command_names() {
            let idx = route
                .get(name)
                .unwrap_or_else(|| panic!("路由表缺少命令: {}", name));
            assert!(*idx < handlers.len(), "路由索引越界: {}", name);
            checked += 1;
        }
    }
    assert!(checked > 0, "没有检查到任何命令");
}

#[test]
fn router_route_size_equals_declared_command_count() {
    let declared: usize = plugin_objects()
        .iter()
        .map(|p| p.command_names().len())
        .sum();
    let (route, handlers) = manager().build_router();
    assert_eq!(route.len(), declared, "路由表大小与命令声明总数不符");
    assert_eq!(
        handlers.len(),
        PLUGIN_SOURCES.len(),
        "handler 数应等于插件数（每插件一个 handler）"
    );
}

#[test]
fn tool_ids_and_names_are_unique() {
    let tools = manager().get_tool_list();
    assert_eq!(tools.len(), PLUGIN_SOURCES.len(), "插件数量不符");
    let ids: HashSet<&str> = tools.iter().map(|t| t.id.as_str()).collect();
    let names: HashSet<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    assert_eq!(ids.len(), tools.len(), "插件 id 存在重复");
    assert_eq!(names.len(), tools.len(), "插件 name 存在重复");
}

#[test]
fn every_plugin_exposes_at_least_one_command() {
    for plugin in plugin_objects() {
        assert!(
            !plugin.command_names().is_empty(),
            "插件 {} 没有声明任何命令",
            plugin.id()
        );
        assert!(!plugin.icon().is_empty(), "插件 {} 缺少图标", plugin.id());
        assert!(!plugin.name().is_empty(), "插件 {} 缺少名称", plugin.id());
    }
}
