fn main() {
    // 仅在目标平台为 Windows 时执行
    // CARGO_CFG_TARGET_OS 是主机系统，交叉编译时永远是 linux/mac
    // 用 CARGO_CFG_TARGET_ENV 判断：gnu = MinGW (Linux->Windows), msvc = MSVC (Windows->Windows)
    if std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "gnu" {
        let mut res = winres::WindowsResource::new();

        // 设置图标路径（相对于项目根目录）
        res.set_icon("assets/icon.ico");

        // 添加 Windows 文件属性信息
        res.set("ProductName", "批量ping工具");
        res.set("FileDescription", "高性能多目标ping工具，支持1000+IP同时ping");
        res.set("LegalCopyright", "Copyright 2025 byff");

        // 交叉编译时通过 WINDRES 环境变量指定 windres 路径
        if let Ok(windres_path) = std::env::var("WINDRES") {
            res.set_windres_path(&windres_path);
        }

        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=winres failed: {}", e);
        }
    }
}
