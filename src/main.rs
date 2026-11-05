#![cfg_attr(all(windows, not(test)), windows_subsystem = "windows")]
// O harness de testes deve usar o console para exibir falhas e resultados.

#[cfg(windows)]
mod native;

#[cfg(windows)]
fn main() {
    native::run();
}

#[cfg(not(windows))]
fn main() {
    eprintln!("Rtime é um aplicativo exclusivo para Windows 10/11.");
}
