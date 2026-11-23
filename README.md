 <div align="center">
  <img src="./assets/rtime.png" alt="RTIME Icon" width="100">

  <h1>RTIME</h1>

  [![Language: Rust](https://img.shields.io/badge/LANGUAGE-Rust-B7410E?style=flat-square&labelColor=353535)](https://www.rust-lang.org/) ![Interface: Win32](https://img.shields.io/badge/INTERFACE-Win32-697789?style=flat-square&labelColor=353535) ![Platform: Windows 10/11](https://img.shields.io/badge/PLATFORM-Windows%2010%20%2F%2011-3776AB?style=flat-square&labelColor=353535) ![Version: 0.3.20](https://img.shields.io/badge/VERSION-0.3.20-697789?style=flat-square&labelColor=353535) ![License: MIT](https://img.shields.io/badge/LICENSE-MIT-4C9A2A?style=flat-square&labelColor=353535)

  **Aplicativo nativo de monitoramento do sistema para Windows, com widgets configuráveis na área de trabalho.**

  <a href="https://github.com/joaorizzo0112/RTIME/releases/latest">
    <img src="https://img.shields.io/badge/BAIXAR-RTIME%20PARA%20WINDOWS-697789?style=for-the-badge&labelColor=353535" alt="Baixar RTIME">
  </a>

  <sub>Windows 10/11 · Rust · Win32 · Sem WebView</sub>
</div>

O RTIME reúne informações de desempenho e atividade do computador em seis widgets independentes. Desenvolvido em Rust com a API Win32, utiliza recursos nativos do Windows para apresentar métricas em tempo real, com opções de personalização visual e organização na área de trabalho.

## Funcionalidades

### Monitoramento do sistema

- **CPU:** percentual de utilização e histórico visual do processamento.
- **Memória:** utilização atual e capacidade total de RAM.
- **Disco:** percentual de utilização e espaço disponível da unidade selecionada.
- **Rede:** velocidade de transferência e contadores acumulados da interface selecionada.
- **Atividade:** tempo de funcionamento do Windows.
- **Processos:** quantidade de processos ativos e três processos com maior consumo de memória física.

### Interface e personalização

- Três estilos: *Fluent Light*, *Midnight Minimal* e *Compact Dashboard*.
- Temas escuro, claro e automático, seguindo o Windows.
- Cores de destaque personalizáveis.
- Quatro tamanhos de widgets.
- Ajustes de opacidade, tipografia, bordas e gráficos.
- Editor individual para cada widget.
- Perfis visuais rápidos: Minimalista, Trabalho e Monitoramento.

### Organização e controle

- Organização automática em grade, linhas ou colunas.
- Nove posições de alinhamento para o conjunto de widgets.
- Seleção de monitor de destino e espaçamento.
- Alinhamento magnético entre widgets.
- Arraste individual ou em grupo.
- Bloqueio e desbloqueio das posições.
- Opções para ocultar, mostrar e recuperar widgets.
- Integração com a área de notificação do Windows.

## Instalação

O RTIME é distribuído como um executável para Windows.

1. Acesse a [página de Releases](https://github.com/joaorizzo0112/RTIME/releases/latest).
2. Abra a versão mais recente e localize `Rtime.exe` em Assets.
3. Baixe o executável para o computador.
4. Coloque o arquivo em uma pasta de sua preferência.
5. Execute `Rtime.exe`.

Após a inicialização, os widgets ficam disponíveis na área de trabalho e o ícone do aplicativo aparece na área de notificação do Windows.

**Para abrir as configurações:** clique com o botão esquerdo no ícone do RTIME na área de notificação.

**Para acessar os comandos:** clique com o botão direito no mesmo ícone.

**Para encerrar:** abra o menu contextual e selecione *Sair*.

### Atualização

1. Encerre o RTIME pelo menu da área de notificação.
2. Acesse novamente a [página de Releases](https://github.com/joaorizzo0112/RTIME/releases/latest).
3. Baixe o novo `Rtime.exe`.
4. Substitua o executável anterior pelo novo arquivo.
5. Inicie o aplicativo novamente.

As preferências armazenadas no Registro do Windows são preservadas durante a substituição do executável.

## Configurações

As configurações são acessadas pelo ícone do RTIME na área de notificação. A janela principal possui cinco seções.

### Aparência

Define o estilo visual dos widgets e suas características gerais.

**Estilo visual**

- **Fluent Light:** aparência baseada no Fluent Design.
- **Midnight Minimal:** apresentação escura e minimalista.
- **Compact Dashboard:** disposição compacta das informações.

**Modo de cor**

- Escuro.
- Claro.
- Automático (Windows), acompanhando o tema do sistema.

**Cor de destaque**

- Padrão do estilo.
- Azul.
- Violeta.
- Esmeralda.
- Âmbar.
- Coral.

**Tamanho dos widgets**

- Compacto.
- Padrão.
- Expandido.
- Horizontal.

**Opacidade dos cartões**

- 100%.
- 94%.
- 86%.

Também é possível controlar globalmente a exibição de gráficos históricos, barras de progresso e textos secundários.

O tema da própria janela de configurações possui um seletor independente, com opções Claro e Escuro.

### Widgets

Permite controlar individualmente os seis indicadores apresentados na área de trabalho.

Cada widget possui:

- Controle de ativação e visibilidade.
- Seleção individual de tamanho.
- Pré-visualização do indicador.
- Acesso ao editor de personalização.

A desativação de um widget não exclui suas preferências ou configurações.

### Layout

Controla a organização e o posicionamento dos widgets.

**Modos de organização**

- Grade.
- Linhas.
- Colunas.

**Espaçamento**

- 8 px: justo.
- 14 px: confortável.
- 22 px: amplo.
- 32 px: espaçado.

**Monitor de destino**

Permite selecionar em qual monitor o conjunto será organizado.

**Posição do conjunto**

Oferece nove posições de alinhamento:

- Superior esquerdo.
- Superior centro.
- Superior direito.
- Centro esquerdo.
- Centro.
- Centro direito.
- Inferior esquerdo.
- Inferior centro.
- Inferior direito.

**Controles adicionais**

- **Mover widgets em grupo:** permite deslocar vários widgets mantendo suas posições relativas.
- **Alinhamento magnético:** aproxima e alinha os widgets durante o arraste.
- **Aplicar organização:** reorganiza os cartões conforme as opções selecionadas.

Para arrastar os widgets manualmente, suas posições precisam estar destravadas.

### Comportamento

Controla a interação dos widgets com a área de trabalho e outros aplicativos.

**Manter widgets atrás das janelas**

Mantém os cartões atrás das demais janelas do Windows. A opção também pode ser desativada.

**Posição dos cartões**

Alterna entre posições travadas e destravadas, evitando movimentações acidentais.

**Visibilidade**

Permite exibir ou ocultar todos os widgets simultaneamente.

**Recuperação**

Reposiciona os widgets quando ficam fora da área visível, inclusive após alterações na configuração dos monitores.

### Monitoramento

Permite escolher as fontes utilizadas para a coleta de informações.

**Unidade de disco**

Seleciona a unidade que será monitorada, incluindo a unidade do sistema e outras unidades disponíveis.

**Adaptador de rede**

Permite selecionar a interface de rede utilizada para calcular as taxas de transferência.

**Atualização dos dados**

Disponibiliza quatro intervalos:

- 0,5 segundo.
- 1 segundo.
- 2 segundos.
- 5 segundos.

**Atualizar lista de dispositivos**

Atualiza as unidades e os adaptadores disponíveis quando um dispositivo é conectado, removido ou alterado.

O intervalo de atualização geral não altera os intervalos internos das consultas de disco e processos, que utilizam períodos próprios para reduzir leituras desnecessárias.

## Editor de widgets

Além das configurações gerais, o RTIME possui um editor integrado para personalizar cada indicador separadamente.

Para acessar:

1. Abra as configurações do RTIME.
2. Entre na seção *Aparência*.
3. Clique em *Personalizar widgets*.
4. Selecione o indicador que deseja modificar.

O editor oferece as seguintes opções:

| Configuração | Opções |
|---|---|
| Estilo visual | Seguir estilo global, Fluent Light, Midnight Minimal e Compact Dashboard |
| Cor de destaque | Seguir cor global, padrão do estilo, Azul, Violeta, Esmeralda, Âmbar e Coral |
| Gráfico | Seguir configuração global, Linha, Área suave, Barras verticais e Desligado |
| Números | Padrão, Números maiores e Números discretos |
| Borda | Sutil, Cor de destaque e Sem borda |
| Opacidade | Seguir opacidade global, 100%, 94% e 86% |
| Barra de progresso | Seguir configuração global, Sempre exibir e Ocultar |
| Textos secundários | Seguir configuração global, Sempre exibir e Ocultar |
| Intervalo de atualização | 0,5, 1, 2 e 5 segundos |

### Perfis rápidos

O editor também oferece três perfis visuais globais:

- **Minimalista:** utiliza Midnight Minimal, tema escuro e reduz a quantidade de elementos visuais.
- **Trabalho:** utiliza Fluent Light, tema claro e mantém as barras e os textos secundários.
- **Monitoramento:** utiliza Compact Dashboard, tema escuro e mantém gráficos, barras e textos secundários.

Os perfis alteram as preferências visuais globais. As personalizações individuais dos widgets permanecem preservadas.

### Ações adicionais

- **Anterior / Próximo:** alterna entre os seis widgets.
- **Restaurar este widget:** recupera suas configurações visuais padrão.
- **Copiar aparência para todos:** replica as opções visuais do widget selecionado para os outros cinco, preservando tamanhos, posições e estados de ativação.
- **Arraste em grupo:** ativa ou desativa a movimentação conjunta.
- **Organizar conjunto agora:** aplica o modo de organização atualmente selecionado.

## Menu da área de notificação

O menu contextual do RTIME oferece acesso rápido às principais operações.

| Comando | Função |
|---|---|
| Configurações | Abre a janela de preferências |
| Mostrar / ocultar widgets | Controla a visibilidade de todos os cartões |
| Travar / destravar posição | Controla a movimentação |
| Ligar / desligar alinhamento magnético | Alterna o encaixe automático |
| Ativar / desativar arraste em grupo | Controla a movimentação conjunta |
| Organizar: grade por monitor | Organiza os widgets em grade |
| Organizar: linhas por monitor | Organiza os widgets horizontalmente |
| Organizar: colunas por monitor | Organiza os widgets verticalmente |
| Recuperar posições fora da tela | Reposiciona cartões não visíveis |
| Sair | Encerra o aplicativo |

## Arquitetura técnica

O RTIME utiliza uma arquitetura nativa para Windows, organizada em torno de duas responsabilidades principais: a interface Win32, executada na thread principal, e a coleta periódica de métricas, executada em uma thread de trabalho independente.

Essa separação mantém a atualização dos indicadores fora do loop de mensagens da interface.

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
│           Metrics             │
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

### Responsabilidade dos módulos

| Arquivo | Responsabilidade |
|---|---|
| `src/main.rs` | Ponto de entrada da aplicação. |
| `src/native.rs` | Coordenação da interface Win32, janelas, widgets e comunicação entre threads. |
| `src/native/metrics.rs` | Coleta e cálculo das métricas do sistema. |
| `src/native/config.rs` | Gerenciamento e persistência das configurações. |
| `src/native/desktop.rs` | Identificação dos monitores, áreas úteis e DPI. |
| `src/native/layout.rs` | Algoritmos de posicionamento, alinhamento e organização dos widgets. |
| `src/native/card_layout.rs` | Geometria e hierarquia visual dos cartões. |
| `src/native/editor_layout.rs` | Organização do editor individual de widgets. |
| `src/native/settings_layout.rs` | Organização dos painéis e controles de configurações. |

### APIs nativas utilizadas

| Métrica | API |
|---|---|
| CPU | `GetSystemTimes` |
| Memória RAM | `GlobalMemoryStatusEx` |
| Disco | `GetDiskFreeSpaceExW` |
| Rede | `GetIfTable2` |
| Tempo de atividade | `GetTickCount64` |
| Processos | `CreateToolhelp32Snapshot` |
| Memória dos processos | `K32GetProcessMemoryInfo` |

O histórico visual utiliza uma estrutura fixa de 60 amostras em memória. A transferência das amostras entre as threads é realizada por mensagens Win32, utilizando `PostMessageW`.

O posicionamento leva em consideração as áreas úteis dos monitores, o DPI e as coordenadas da área de trabalho virtual.

## Compilar a partir do código-fonte

Caso prefira gerar o executável diretamente do código, é possível utilizar o script de compilação incluído no repositório.

<details>
<summary><b>Ver o passo a passo</b></summary>

### Requisitos

- Windows 10 ou Windows 11.
- Rust instalado por meio do [rustup](https://rustup.rs/).
- Cargo disponível no terminal.
- Ambiente de compilação Windows compatível com o target MSVC do Rust.
- Git.

### 1. Clonar o repositório

Abra o PowerShell e execute:

```powershell
git clone https://github.com/joaorizzo0112/RTIME.git
```

### 2. Entrar na pasta do projeto

```powershell
cd RTIME
```

### 3. Executar a compilação

```powershell
.\build-windows.ps1
```

O script executa automaticamente:

1. Formatação do código com `cargo fmt --all`.
2. Testes automatizados com `cargo test --release`.
3. Compilação de produção com `cargo build --release`.
4. Cópia do executável para a pasta `dist`.

### 4. Localizar o executável

Após a compilação, o arquivo estará disponível em:

```text
dist/
└── Rtime.exe
```

### Compilação manual

As operações também podem ser executadas separadamente:

```powershell
cargo fmt --all
cargo test --release
cargo build --release
```

O executável gerado ficará em:

```text
target/release/rtime.exe
```

</details>

## Persistência e privacidade

As métricas são coletadas localmente por meio das APIs nativas do Windows.

As preferências do RTIME são armazenadas no Registro do usuário:

```text
HKEY_CURRENT_USER\Software\Rtime
```

As configurações incluem aparência, dimensões, posições, visibilidade dos widgets, organização e fontes de monitoramento.

O projeto também possui compatibilidade de leitura com a chave de Registro de uma versão anterior, permitindo migrar as preferências para o caminho atual.

## Estrutura do projeto

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
├── README.md
└── SHA256SUMS.txt
```

## Créditos

Desenvolvido e mantido por [@joaorizzo0112](https://github.com/joaorizzo0112).

## Licença

Distribuído sob a licença MIT. Consulte o arquivo [LICENSE](LICENSE) para conhecer os termos de utilização, modificação e distribuição.

---

<a id="english"></a>

<details>
<summary><b>English version</b></summary>

# RTIME

**Native Windows system monitoring application with configurable desktop widgets.**

RTIME brings computer performance and activity information together in six independent widgets. Built with Rust and the Win32 API, it uses native Windows resources to display real-time metrics with visual customization and desktop layout options.

## Features

### System monitoring

- **CPU:** usage percentage and visual history.
- **Memory:** current RAM usage and total capacity.
- **Disk:** usage percentage and available space for the selected drive.
- **Network:** transfer rates and accumulated counters for the selected interface.
- **Activity:** Windows system uptime.
- **Processes:** active process count and the three processes using the most physical memory.

### Interface and customization

- Three widget styles: *Fluent Light*, *Midnight Minimal* and *Compact Dashboard*.
- Dark, light and automatic Windows themes.
- Customizable accent colors.
- Four widget sizes.
- Opacity, typography, borders and graph settings.
- Individual widget editor.
- Quick visual profiles: Minimalist, Work and Monitoring.

### Layout and control

- Automatic grid, row and column arrangements.
- Nine alignment positions for the widget group.
- Target monitor and spacing selection.
- Magnetic widget alignment.
- Individual and group dragging.
- Position locking and unlocking.
- Hide, show and recover widgets.
- Windows notification area integration.

## Installation

RTIME is distributed as a Windows executable.

1. Visit the [Releases page](https://github.com/joaorizzo0112/RTIME/releases/latest).
2. Open the latest release and find `Rtime.exe` under Assets.
3. Download the executable.
4. Place it in a folder of your choice.
5. Run `Rtime.exe`.

After startup, the widgets become available on the desktop and the application icon appears in the Windows notification area.

**To open settings:** left-click the RTIME icon in the notification area.

**To access commands:** right-click the same icon.

**To exit:** open the context menu and select *Exit*.

### Updating

1. Exit RTIME from the notification area menu.
2. Visit the [latest Releases page](https://github.com/joaorizzo0112/RTIME/releases/latest).
3. Download the new `Rtime.exe`.
4. Replace the previous executable.
5. Start the application again.

Preferences stored in the Windows Registry are preserved when replacing the executable.

## Settings

RTIME settings are accessed through the notification area icon. The main settings window contains five sections.

### Appearance

Controls the visual style and general appearance of the widgets.

**Visual style**

- **Fluent Light:** Fluent Design-inspired appearance.
- **Midnight Minimal:** dark and minimal presentation.
- **Compact Dashboard:** compact information layout.

**Color mode**

- Dark.
- Light.
- Automatic (Windows), following the system theme.

**Accent color**

- Style default.
- Blue.
- Violet.
- Emerald.
- Amber.
- Coral.

**Widget size**

- Compact.
- Standard.
- Expanded.
- Horizontal.

**Card opacity**

- 100%.
- 94%.
- 86%.

Global controls are also available for historical graphs, progress bars and secondary text.

The settings window has its own independent theme selector, with Light and Dark options.

### Widgets

Controls the six indicators displayed on the desktop individually.

Each widget has:

- Individual activation and visibility.
- Individual size selection.
- Indicator preview.
- Access to its customization editor.

Disabling a widget does not delete its preferences or settings.

### Layout

Controls widget arrangement and positioning.

**Arrangement modes**

- Grid.
- Rows.
- Columns.

**Spacing**

- 8 px: tight.
- 14 px: comfortable.
- 22 px: wide.
- 32 px: spacious.

**Target monitor**

Selects the monitor where the widget group will be arranged.

**Group position**

Provides nine alignment positions:

- Top left.
- Top center.
- Top right.
- Center left.
- Center.
- Center right.
- Bottom left.
- Bottom center.
- Bottom right.

**Additional controls**

- **Group widget movement:** moves multiple widgets while preserving their relative positions.
- **Magnetic alignment:** automatically aligns widgets during dragging.
- **Apply arrangement:** rearranges the cards using the selected options.

Widget positions must be unlocked before manual dragging.

### Behavior

Controls how widgets interact with the desktop and other applications.

**Keep widgets behind windows**

Keeps the cards behind other Windows applications. This option can also be disabled.

**Widget positions**

Locks or unlocks widget positions to prevent accidental movement.

**Visibility**

Shows or hides all widgets simultaneously.

**Recovery**

Repositions widgets that are outside the visible desktop area, including after monitor configuration changes.

### Monitoring

Selects the sources used to collect system information.

**Disk drive**

Selects the monitored drive, including the system drive and other available drives.

**Network adapter**

Selects the network interface used to calculate transfer rates.

**Data update interval**

Provides four intervals:

- 0.5 seconds.
- 1 second.
- 2 seconds.
- 5 seconds.

**Refresh device list**

Refreshes the available drives and network adapters when a device is connected, removed or changed.

The general update interval does not change the internal disk and process sampling intervals, which use their own periods to reduce unnecessary readings.

## Widget editor

In addition to the general settings, RTIME includes an integrated editor for customizing each indicator individually.

To open it:

1. Open RTIME settings.
2. Go to *Appearance*.
3. Click *Customize widgets*.
4. Select the indicator you want to modify.

The editor provides the following options:

| Setting | Options |
|---|---|
| Visual style | Follow global style, Fluent Light, Midnight Minimal and Compact Dashboard |
| Accent color | Follow global color, style default, Blue, Violet, Emerald, Amber and Coral |
| Graph | Follow global setting, Line, Smooth area, Vertical bars and Off |
| Numbers | Default, Larger numbers and Subtle numbers |
| Border | Subtle, Accent color and No border |
| Opacity | Follow global opacity, 100%, 94% and 86% |
| Progress bar | Follow global setting, Always show and Hide |
| Secondary text | Follow global setting, Always show and Hide |
| Update interval | 0.5, 1, 2 and 5 seconds |

### Quick profiles

The editor also provides three global visual profiles:

- **Minimalist:** uses Midnight Minimal, Dark mode and reduces visual elements.
- **Work:** uses Fluent Light, Light mode and keeps progress bars and secondary text.
- **Monitoring:** uses Compact Dashboard, Dark mode and enables graphs, progress bars and secondary text.

Profiles modify global visual preferences. Individual widget customizations remain preserved.

### Additional actions

- **Previous / Next:** switches between the six widgets.
- **Reset this widget:** restores its default visual settings.
- **Copy appearance to all:** copies the selected widget's visual options to the other five, preserving sizes, positions and activation states.
- **Group dragging:** enables or disables group movement.
- **Arrange group now:** applies the currently selected arrangement mode.

## Notification area menu

The RTIME context menu provides quick access to its main operations.

| Command | Function |
|---|---|
| Settings | Opens the preferences window |
| Show / hide widgets | Controls visibility of all cards |
| Lock / unlock position | Controls movement |
| Enable / disable magnetic alignment | Toggles automatic snapping |
| Enable / disable group dragging | Controls group movement |
| Arrange: grid per monitor | Arranges widgets in a grid |
| Arrange: rows per monitor | Arranges widgets horizontally |
| Arrange: columns per monitor | Arranges widgets vertically |
| Recover off-screen positions | Repositions invisible cards |
| Exit | Closes the application |

## Technical architecture

RTIME uses a native Windows architecture organized around two main responsibilities: the Win32 interface, running on the main thread, and periodic metric collection, running on an independent worker thread.

This separation keeps metric collection outside the interface message loop.

### System overview

```text
┌───────────────────────────────┐
│        WORKER THREAD          │
│                               │
│       Sampler::read()         │
│              │                │
│              ▼                │
│       Native Windows APIs     │
│                               │
│      ├── CPU                  │
│      ├── RAM                  │
│      ├── Disk                 │
│      ├── Network              │
│      ├── Uptime               │
│      └── Processes            │
│              │                │
│              ▼                │
│           Metrics             │
└──────────────┬────────────────┘
               │
               │ PostMessageW
               │ EVT_METRICS
               ▼
┌───────────────────────────────┐
│          MAIN THREAD          │
│                               │
│           Win32 API           │
│                               │
│      ├── Widgets              │
│      ├── GDI drawing          │
│      ├── Settings             │
│      ├── Widget editor        │
│      ├── System tray          │
│      └── Message loop         │
│              │                │
│              ▼                │
│       Visual updates          │
│              │                │
│              ▼                │
│       In-memory history       │
└───────────────────────────────┘
```

### Module responsibilities

| File | Responsibility |
|---|---|
| `src/main.rs` | Application entry point. |
| `src/native.rs` | Win32 interface, windows, widgets and communication between threads. |
| `src/native/metrics.rs` | System metric collection and calculations. |
| `src/native/config.rs` | Configuration management and persistence. |
| `src/native/desktop.rs` | Monitor detection, work areas and DPI. |
| `src/native/layout.rs` | Widget positioning, alignment and arrangement algorithms. |
| `src/native/card_layout.rs` | Card geometry and visual hierarchy. |
| `src/native/editor_layout.rs` | Individual widget editor layout. |
| `src/native/settings_layout.rs` | Settings panels and controls layout. |

### Native Windows APIs

| Metric | API |
|---|---|
| CPU | `GetSystemTimes` |
| RAM | `GlobalMemoryStatusEx` |
| Disk | `GetDiskFreeSpaceExW` |
| Network | `GetIfTable2` |
| System uptime | `GetTickCount64` |
| Processes | `CreateToolhelp32Snapshot` |
| Process memory | `K32GetProcessMemoryInfo` |

Visual history uses a fixed in-memory structure containing 60 samples. Samples are transferred between threads through Win32 messages using `PostMessageW`.

Widget positioning takes monitor work areas, DPI and virtual desktop coordinates into account.

## Building from source

If you prefer to build the executable directly from the source code, use the build script included in the repository.

<details>
<summary><b>View the step-by-step guide</b></summary>

### Requirements

- Windows 10 or Windows 11.
- Rust installed through [rustup](https://rustup.rs/).
- Cargo available in the terminal.
- Windows build environment compatible with the Rust MSVC target.
- Git.

### 1. Clone the repository

Open PowerShell and run:

```powershell
git clone https://github.com/joaorizzo0112/RTIME.git
```

### 2. Enter the project folder

```powershell
cd RTIME
```

### 3. Run the build

```powershell
.\build-windows.ps1
```

The script automatically performs:

1. Rust formatting with `cargo fmt --all`.
2. Automated tests with `cargo test --release`.
3. Production compilation with `cargo build --release`.
4. Executable copying to the `dist` folder.

### 4. Locate the executable

After compilation, the file will be available at:

```text
dist/
└── Rtime.exe
```

### Manual build

The commands can also be executed separately:

```powershell
cargo fmt --all
cargo test --release
cargo build --release
```

The generated executable will be located at:

```text
target/release/rtime.exe
```

</details>

## Persistence and privacy

Metrics are collected locally through native Windows APIs.

RTIME preferences are stored in the user's Windows Registry:

```text
HKEY_CURRENT_USER\Software\Rtime
```

Settings include appearance, dimensions, positions, widget visibility, layout and monitoring sources.

The project also supports reading a legacy Registry key from a previous application version, allowing preferences to migrate to the current location.

## Project structure

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
├── README.md
└── SHA256SUMS.txt
```

## Credits

Developed and maintained by [@joaorizzo0112](https://github.com/joaorizzo0112).

## License

Distributed under the MIT License. See [LICENSE](LICENSE) for the terms of use, modification and distribution.

</details>
