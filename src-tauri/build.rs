fn main() {
    tauri_build::build();

    // tauri-winres/embed-resource 只把生成的 Windows 资源（含
    // Microsoft.Windows.Common-Controls 6.0 清单）链接进 *bins*，
    // `cargo test` 产出的集成测试可执行文件因此没有该清单。
    // Windows 系统目录下的 comctl32.dll 是 5.82，不导出
    // TaskDialogIndirect；只有加载 comctl32 v6 才有该入口。
    // 一旦测试二进制拉入 Tauri 命令表（触发 comctl32!TaskDialogIndirect），
    // 进程会在加载阶段就以 STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139) 退出。
    // 这里把同一份 resource.lib 也链接进 tests/ 下的集成测试目标。
    // 注意：必须用 -tests 而非兜底的 cargo:rustc-link-arg，否则 bins 会
    // 收到两份相同资源并以 CVT1100 duplicate resource 链接失败。
    if let Ok(out_dir) = std::env::var("OUT_DIR") {
        let resource = std::path::Path::new(&out_dir).join("resource.lib");
        if resource.exists() {
            println!("cargo:rustc-link-arg-tests={}", resource.display());
        }
    }

    // 版本数据 JSON 位于 crate 目录之外（前端 src/data/），
    // include_str! 引用的文件 Cargo 默认不跟踪变更，显式声明以保证编辑后重新编译
    println!("cargo:rerun-if-changed=../src/data/mysql_versions.json");
    println!("cargo:rerun-if-changed=../src/data/postgresql_versions.json");
    println!("cargo:rerun-if-changed=../src/data/software.json");
}
