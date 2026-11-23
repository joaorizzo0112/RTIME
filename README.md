<div align="center">

<img src="./assets/rtime.png" alt="Ícone do RTIME" width="180">

<h1>RTIME</h1>

[![Linguagem: Rust](https://img.shields.io/badge/LANGUAGE-Rust-B7410E?style=flat-square&labelColor=353535)](https://www.rust-lang.org/) ![Interface: Win32](https://img.shields.io/badge/INTERFACE-Win32-697789?style=flat-square&labelColor=353535) ![Plataforma: Windows 10/11](https://img.shields.io/badge/PLATFORM-Windows%2010%20%2F%2011-3776AB?style=flat-square&labelColor=353535) ![Versão: 0.3.20](https://img.shields.io/badge/VERSION-0.3.20-697789?style=flat-square&labelColor=353535) ![Licença: MIT](https://img.shields.io/badge/LICENSE-MIT-4C9A2A?style=flat-square&labelColor=353535)

**Aplicativo nativo de monitoramento do sistema para Windows, com widgets configuráveis na área de trabalho.**

</div>

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
|---|---|
| Rust | Implementação e lógica do aplicativo |
| API Win32 | Janelas, controles, desenho e integração com o Windows |
| `windows-sys` | Acesso às APIs nativas do sistema |
| Registro do Windows | Persistência das preferências do usuário |
| `winresource` | Incorporação do ícone ao executável |
| GitHub Actions | Testes e compilação automatizada para Windows |

## Arquitetura técnica

O RTIME utiliza uma arquitetura nativa para Windows, organizada em torno de duas responsabilidades principais: a interface Win32, executada na thread principal, e a coleta periódica de métricas, executada em uma thread de trabalho independente.

Essa separação mantém a atualização dos indicadores fora do loop de mensagens da interface. Os dados coletados são enviados à thread principal por mensagens Win32, que atualiza os widgets e o histórico visual.

### Visão geral do funcionamento

```text
┌───────────────────────────────┐
│       THREAD DE COLETA        │
│                               │
│       Sampler::read()         │
│              │                │
│              ▼                │
│      APIs nativas do Windows  │
│                               │
│      ├── CPU                  │
│      ├── Memória RAM          │
│      ├── Disco                │
│      ├── Rede                 │
│      ├── Atividade            │
│      └── Processos            │
│              │                │
│              ▼                │
│          Metrics              │
└──────────────┬────────────────┘
               │
               │ PostMessageW
               │ EVT_METRICS
               ▼
┌───────────────────────────────┐
│      THREAD PRINCIPAL         │
│                               │
│          API Win32            │
│                               │
│      ├── Widgets              │
│      ├── Desenho GDI          │
│      ├── Configurações        │
│      ├── Editor de widgets    │
│      ├── System Tray          │
│      └── Loop de mensagens    │
│              │                │
│              ▼                │
│      Atualização visual       │
│              │                │
│              ▼                │
│      Histórico em memória     │
└───────────────────────────────┘
```

### Fluxo de atualização das métricas

1. A aplicação inicia a interface Win32 e cria as janelas utilizadas pelos widgets, configurações e área de notificação.
2. Uma thread de trabalho executa o coletor `Sampler`, responsável por consultar as APIs nativas do Windows.
3. As amostras são agrupadas na estrutura `Metrics` e enviadas à thread principal por meio de `PostMessageW`.
4. Ao receber a mensagem `EVT_METRICS`, a interface atualiza os valores, registra a amostra no histórico em memória e solicita o redesenho dos componentes.
5. O intervalo de coleta pode ser configurado entre 0,5, 1, 2 e 5 segundos.

Algumas consultas possuem intervalos próprios para evitar leituras desnecessárias: disco a cada 10 segundos e processos a cada 5 segundos.

O coletor mantém no máximo uma mensagem de métricas pendente na fila da interface, evitando o acúmulo de atualizações quando a thread principal ainda está processando uma amostra.

### Responsabilidade dos módulos

| Arquivo | Responsabilidade |
|---|---|
| `src/main.rs` | Ponto de entrada da aplicação. Em Windows, encaminha a execução para `native::run()`. |
| `src/native.rs` | Coordena a aplicação Win32: ciclo de vida, janelas, widgets, área de notificação, loop de mensagens e comunicação com a thread de coleta. |
| `src/native/metrics.rs` | Coleta e calcula as métricas do sistema por meio das APIs do Windows. |
| `src/native/config.rs` | Define as preferências, valores padrão e leitura e gravação das configurações no Registro do Windows. |
| `src/native/desktop.rs` | Identifica os monitores conectados, suas áreas úteis e o DPI efetivo de cada tela. |
| `src/native/layout.rs` | Implementa os algoritmos de posicionamento, alinhamento, organização, recuperação e encaixe magnético dos widgets. |
| `src/native/card_layout.rs` | Calcula a geometria e a hierarquia visual dos cartões de métricas, considerando conteúdo e tamanho do widget. |
| `src/native/editor_layout.rs` | Organiza os campos, seções e áreas de pré-visualização do editor individual de widgets. |
| `src/native/settings_layout.rs` | Calcula a disposição dos painéis e controles da janela de configurações, adaptando o layout ao espaço disponível. |

### Coleta de dados do Windows

O módulo `metrics.rs` utiliza APIs nativas para obter os dados diretamente do sistema operacional.

| Métrica | API ou mecanismo |
|---|---|
| CPU | `GetSystemTimes` |
| Memória RAM | `GlobalMemoryStatusEx` |
| Espaço e utilização do disco | `GetDiskFreeSpaceExW` |
| Tráfego e contadores da rede | `GetIfTable2` |
| Tempo de atividade do sistema | `GetTickCount64` |
| Quantidade de processos | `CreateToolhelp32Snapshot` |
| Memória dos processos | `K32GetProcessMemoryInfo` |

A velocidade de transferência da rede é calculada pela diferença entre as leituras sucessivas dos contadores da interface selecionada.

Na área de processos, o aplicativo apresenta os três maiores consumidores de memória física (*Working Set*).

### Interface, posicionamento e configurações

A camada de interface utiliza janelas e controles Win32, com desenho nativo por GDI. O módulo central coordena os eventos e a apresentação dos dados, enquanto os módulos de layout calculam as dimensões e posições dos elementos.

O posicionamento considera a área útil de cada monitor, o DPI e as coordenadas da área de trabalho virtual. Isso permite organizar os widgets em grade, linhas ou colunas, aplicar âncoras de alinhamento, utilizar encaixe magnético e recuperar janelas que tenham ficado fora da área visível.

As preferências são carregadas na inicialização e gravadas no Registro do Windows. A configuração também contempla a leitura da chave legada utilizada por uma versão anterior do aplicativo, permitindo migrar as preferências para o caminho atual.

### Estrutura do código

```text
RTIME/
├── .github/
│   └── workflows/
│       └── windows.yml
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
├── Cargo.toml
├── LICENSE
└── SHA256SUMS.txt
```

## Persistência e privacidade

As métricas são consultadas pelas APIs do próprio Windows. As preferências do RTIME são armazenadas localmente no Registro do usuário, em `HKEY_CURRENT_USER\Software\Rtime`.

O aplicativo é destinado ao Windows 10 e ao Windows 11.

## Autoria

Desenvolvido e mantido por **[@joaorizzo0112](https://github.com/joaorizzo0112)**.

## Licença

O projeto utiliza a licença MIT, conforme declarada no arquivo `Cargo.toml` e disponibilizada em `LICENSE`.
