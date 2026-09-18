fn main() {
    tauri_build::build();

    // 版本数据 JSON 位于 crate 目录之外（前端 src/data/），
    // include_str! 引用的文件 Cargo 默认不跟踪变更，显式声明以保证编辑后重新编译
    println!("cargo:rerun-if-changed=../src/data/mysql_versions.json");
    println!("cargo:rerun-if-changed=../src/data/postgresql_versions.json");
    println!("cargo:rerun-if-changed=../src/data/software.json");
}
