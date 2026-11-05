<h1><img src="./assets/rtime.png" alt="Ícone do RTIME" width="44"> RTIME</h1>

[![Linguagem: Rust](https://img.shields.io/badge/LANGUAGE-Rust-B7410E?style=flat-square&labelColor=353535)](https://www.rust-lang.org/) ![Interface: Win32](https://img.shields.io/badge/INTERFACE-Win32-697789?style=flat-square&labelColor=353535) ![Plataforma: Windows 10/11](https://img.shields.io/badge/PLATFORM-Windows%2010%20%2F%2011-3776AB?style=flat-square&labelColor=353535) ![Versão: 0.3.20](https://img.shields.io/badge/VERSION-0.3.20-697789?style=flat-square&labelColor=353535) ![Licença: MIT](https://img.shields.io/badge/LICENSE-MIT-4C9A2A?style=flat-square&labelColor=353535)

**Aplicativo nativo de monitoramento do sistema para Windows, com widgets configuráveis na área de trabalho.**

O RTIME é desenvolvido em Rust com a API Win32. Reúne informações de desempenho e atividade do computador em uma interface compacta e personalizável, sem depender de WebView ou de um navegador.

## Funcionalidades

### Monitoramento do sistema

- **CPU:** percentual de utilização e histórico visual.
- **Memória:** consumo e capacidade de RAM.
- **Disco:** utilização e espaço disponível da unidade selecionada.
- **Rede:** taxas de transferência e dados acumulados da interface selecionada.
- **Atividade:** tempo de funcionamento do Windows.
- **Processos:** quantidade de processos ativos e processos com maior uso de memória.

### Interface e personalização

- Estilos de widgets *Fluent Light*, *Midnight Minimal* e *Compact Dashboard*.
- Temas claro, escuro e automático, de acordo com o Windows.
- Ajustes de cores, tamanhos, opacidade, gráficos e tipografia.
- Personalização individual dos widgets por meio de um editor integrado.

### Organização e controle

- Organização em grade, linhas ou colunas, com opções de alinhamento.
- Posicionamento manual, alinhamento magnético e arraste em grupo.
- Opções para bloquear, ocultar e recuperar widgets fora da tela.
- Controle por ícone na área de notificação do Windows.
- Seleção de disco, adaptador de rede e intervalo de atualização.

## Tecnologias

| Tecnologia | Aplicação |
| --- | --- |
| Rust | Implementação e lógica do aplicativo |
| API Win32 | Janelas, controles, desenho e integração com o Windows |
| `windows-sys` | Acesso às APIs nativas do sistema |
| Registro do Windows | Persistência das preferências do usuário |
| `winresource` | Incorporação do ícone ao executável |
| GitHub Actions | Testes e compilação automatizada para Windows |

## Arquitetura

O código está organizado em módulos responsáveis por interface, métricas, configurações e posicionamento dos widgets.

```text
RTIME/
├── .github/workflows/
├── assets/
│   ├── rtime.ico
│   └── rtime.png
├── src/
│   ├── native/
│   │   ├── card_layout.rs
│   │   ├── config.rs
│   │   ├── desktop.rs
│   │   ├── editor_layout.rs
│   │   ├── layout.rs
│   │   ├── metrics.rs
│   │   └── settings_layout.rs
│   ├── main.rs
│   └── native.rs
├── build.rs
├── build-windows.ps1
├── Cargo.lock
└── Cargo.toml
```

## Persistência e privacidade

As métricas são consultadas pelas APIs do próprio Windows. As preferências do RTIME são armazenadas localmente no Registro do usuário, em `HKEY_CURRENT_USER\Software\Rtime`.

O aplicativo é destinado ao Windows 10 e ao Windows 11.

## Autoria

Desenvolvido e mantido por **[@joaorizzo0112](https://github.com/joaorizzo0112)**.

## Licença

O projeto declara a licença MIT em seu `Cargo.toml`.
