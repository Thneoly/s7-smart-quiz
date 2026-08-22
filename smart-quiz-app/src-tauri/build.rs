fn main() {
    // tauri-build 不追踪图标文件变化（只盯 tauri.conf.json/capabilities），
    // 换图标后若 build script 不重跑，exe 仍嵌入旧图标资源——这里显式追踪
    println!("cargo:rerun-if-changed=icons/icon.ico");
    tauri_build::build()
}
