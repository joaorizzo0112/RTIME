// Inclui o ícone dentro do executável, para Explorer, barra de tarefas e bandeja.
fn main() {
    println!("cargo:rerun-if-changed=assets/rtime.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon_with_id("assets/rtime.ico", "1")
            .set("FileDescription", "Rtime - Monitoramento do sistema")
            .set("ProductName", "Rtime")
            .set("OriginalFilename", "rtime.exe");
        resource.compile().expect(
            "Falha ao incorporar icone: instale o Windows SDK (rc.exe) ou MinGW (windres.exe).",
        );
    }
}
