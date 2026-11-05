$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw 'Cargo não encontrado. Instale Rust em https://rustup.rs/ e reinicie o PowerShell.'
}
Write-Host 'Validando código Rust...'
cargo fmt --all
if ($LASTEXITCODE -ne 0) { throw 'cargo fmt falhou.' }
cargo test --release
if ($LASTEXITCODE -ne 0) { throw 'Testes falharam.' }
Write-Host 'Compilando executável nativo para Windows...'
cargo build --release
if ($LASTEXITCODE -ne 0) { throw 'Compilação falhou.' }
New-Item -ItemType Directory -Force -Path 'dist' | Out-Null
Copy-Item 'target\release\rtime.exe' 'dist\Rtime.exe' -Force
Write-Host 'Concluído: dist\Rtime.exe' -ForegroundColor Green
