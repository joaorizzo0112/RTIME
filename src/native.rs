//! Interface Win32 nativa. Nenhuma WebView, `egui` ou polling da bandeja.
mod card_layout;
mod config;
mod desktop;
mod editor_layout;
mod layout;
mod metrics;
mod settings_layout;

use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::sync::{
    atomic::{AtomicBool, AtomicU32, Ordering},
    Arc,
};
use std::thread;
use std::time::Duration;

use card_layout::{BoxRect, CardTextLayout, MetricKind};
use editor_layout::{EditorLayout, EditorRect};
use layout::{
    arrange_anchored, nearest_monitor, recover, scaled, snap_moving, Anchor, Arrangement, Rect,
};
use settings_layout::{UiLayout, UiRect};
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::CreateMutexW;
use windows_sys::Win32::UI::Controls::{SetScrollInfo, DRAWITEMSTRUCT, ODS_FOCUS, ODS_SELECTED};
use windows_sys::Win32::UI::HiDpi::{
    GetDpiForWindow, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetCapture, ReleaseCapture, SetCapture, SetFocus,
};
use windows_sys::Win32::UI::Shell::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use config::{
    dimensions, legacy_positions_for, positions_for, positions_for_sizes, system_light_theme,
    update_interval_ms, Config, ACCENTS, BORDER_MODES, FONT_SCALES, GRAPH_MODES,
    INDIVIDUAL_ACCENTS, INDIVIDUAL_OPACITIES, INDIVIDUAL_STYLES, INTERFACE_THEMES, LAYOUT_GAPS,
    LAYOUT_MODES, N, OPACITIES, SIZES, STYLES, THEMES, UPDATE_INTERVALS, VISIBILITY_CHOICES,
};
use metrics::{
    available_adapters, available_drives, disk_label, fmt_network, fmt_network_total, fmt_uptime,
    Metrics, Sampler,
};

pub(crate) fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

// O build.rs grava o ICO como recurso RT_GROUP_ICON de identificador 1.
const RTIME_ICON_RESOURCE: u16 = 1;

const HOST_CLASS: &str = "Rtime.HiddenHost";
const LEGACY_HOST_CLASS: &str = "RustFetchNative.HiddenHost";
const WIDGET_CLASS: &str = "Rtime.Widget";
const SETTINGS_CLASS: &str = "Rtime.Settings";
const SETTINGS_PAGE_CLASS: &str = "Rtime.SettingsViewport";
const SETTINGS_SIDEBAR_WIDTH: i32 = 245;
const SETTINGS_FOOTER_HEIGHT: i32 = 66;
// O conteúdo central agora é refluído por settings_layout.rs, sem canvas fixo.
// Tamanho padrão de Configurações (janela RESTAURADA, não maximizada).
const SETTINGS_DEFAULT_WIDTH: i32 = 1160;
const SETTINGS_DEFAULT_HEIGHT: i32 = 724;
const EDITOR_CLASS: &str = "Rtime.WidgetEditor";
// Dimensões de abertura do editor. Em telas pequenas, só são limitadas pela
// área útil do monitor; depois de aberto, o usuário não pode redimensioná-lo.
const EDITOR_DEFAULT_WIDTH: i32 = 1190;
const EDITOR_DEFAULT_HEIGHT: i32 = 855;
// Estilo de janela fixa para Configurações e Editor: sem maximizar e sem
// bordas de redimensionamento. Minimizar e fechar continuam disponíveis.
const FIXED_DIALOG_STYLE: u32 = WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;

const SETTINGS_CHECK_CLASS: &str = "Rtime.SettingsCheck";
const NAMES: [&str; N] = ["CPU", "MEMÓRIA", "DISCO", "REDE", "ATIVIDADE", "PROCESSOS"];

const EVT_TRAY: u32 = WM_APP + 1;
const EVT_METRICS: u32 = WM_APP + 2;
const EVT_OPEN: u32 = WM_APP + 3;
const EVT_HIDE_AUX: u32 = WM_APP + 4; // Ocultacao adiada para depois do WM_SIZE do Windows.
                                      // Estados do controle BUTTON enviados via BM_SETCHECK (Win32).
const BST_CHECKED: usize = 1;
const BST_UNCHECKED: usize = 0;
const VK_ESCAPE: usize = 0x1B;
const TIMER_SAVE: usize = 3;
const TIMER_SETTINGS_HOVER: usize = 4;
const SETTINGS_HOVER_POLL_MS: u32 = 90;
const ID_TRAY: u32 = 1;
const CMD_SETTINGS: u32 = 1001;
const CMD_VISIBILITY: u32 = 1002;
const CMD_LOCK: u32 = 1003;
const CMD_EXIT: u32 = 1004;
const CMD_BEHIND: u32 = 1005;
const CMD_SNAP: u32 = 1006;
const CMD_ARRANGE_GRID: u32 = 1007;
const CMD_ARRANGE_ROW: u32 = 1008;
const CMD_ARRANGE_COLUMN: u32 = 1009;
const CMD_RECOVER: u32 = 1010;
const CMD_GROUP_MOVE: u32 = 1011;
const CMD_LAYOUT_MODE: u32 = 1510;
const CMD_LAYOUT_MONITOR: u32 = 1512;
const CMD_LAYOUT_GAP: u32 = 1513;
const CMD_APPLY_LAYOUT: u32 = 1514;
const CMD_TAB_BASE: u32 = 1520;
const CMD_SETTINGS_CLOSE: u32 = 1530;
// Fontes de monitoramento: escolha persistida por ID, nao pela linha do ComboBox.
const CMD_MONITOR_DISK: u32 = 1550;
const CMD_MONITOR_ADAPTER: u32 = 1551;
const CMD_MONITOR_INTERVAL: u32 = 1552;
const CMD_MONITOR_REFRESH: u32 = 1553;
const CMD_LAYOUT_ANCHOR_TILE_BASE: u32 = 1560;
const CMD_CARD_BASE: u32 = 1100;
const CMD_STYLE: u32 = 1200;
const CMD_THEME: u32 = 1201;
const CMD_ACCENT: u32 = 1202;
const CMD_SIZE: u32 = 1203;
const CMD_OPACITY: u32 = 1204;
const CMD_RESET_POSITIONS: u32 = 1210;
const CMD_SHOW_GRAPHS: u32 = 1211;
const CMD_SHOW_BARS: u32 = 1212;
const CMD_SHOW_SUBTITLES: u32 = 1213;
const CMD_UI_THEME: u32 = 1214;
const CMD_CARD_SIZE_BASE: u32 = 1300;
const CMD_OPEN_EDITOR: u32 = 1400;
const CMD_EDIT_CARD: u32 = 1410;
const CMD_EDIT_STYLE: u32 = 1411;
const CMD_EDIT_ACCENT: u32 = 1412;
const CMD_EDIT_GRAPH: u32 = 1413;
const CMD_EDIT_FONT: u32 = 1414;
const CMD_EDIT_BORDER: u32 = 1415;
const CMD_EDIT_OPACITY: u32 = 1416;
const CMD_EDIT_BARS: u32 = 1417;
const CMD_EDIT_SUBTITLES: u32 = 1418;
const CMD_EDIT_INTERVAL: u32 = 1419;
const CMD_EDIT_RESET: u32 = 1420;
const CMD_EDIT_CLOSE: u32 = 1421;
const CMD_EDIT_PROFILE: u32 = 1422;
const CMD_EDIT_PREVIOUS: u32 = 1423;
const CMD_EDIT_NEXT: u32 = 1424;
const CMD_EDIT_COPY_ALL: u32 = 1425;
const CMD_EDIT_GROUP_TOGGLE: u32 = 1426;
const CMD_EDIT_ARRANGE_NOW: u32 = 1427;
const PROFILES: [&str; 4] = [
    "Escolha um perfil",
    "Minimalista",
    "Trabalho",
    "Monitoramento",
];

/// Foto do início do arraste: cada cartão mantém seu deslocamento relativo.
struct GroupDrag {
    leader: usize,
    leader_start: Rect,
    members: Vec<(usize, Rect)>,
}

// Histórico fixo: 60 amostras de Metrics, sem crescimento de memória.
#[derive(Clone)]
struct History {
    entries: [Metrics; 60],
    next: usize,
    len: usize,
}

impl Default for History {
    fn default() -> Self {
        Self {
            entries: [Metrics::default(); 60],
            next: 0,
            len: 0,
        }
    }
}

impl History {
    fn push(&mut self, sample: Metrics) {
        self.entries[self.next] = sample;
        self.next = (self.next + 1) % self.entries.len();
        self.len = (self.len + 1).min(self.entries.len());
    }

    fn get(&self, index: usize) -> Metrics {
        let oldest = (self.next + self.entries.len() - self.len) % self.entries.len();
        self.entries[(oldest + index) % self.entries.len()]
    }
}

// Acesso a App restrito à thread Win32 que executa o loop de mensagens.
static mut APP: *mut App = null_mut();

// Geometria da interface responsiva centralizada em settings_layout.rs.
// Retângulos de desenho e controles usam o MESMO cálculo para evitar sobreposição.
fn settings_geometry(view_w: i32, view_h: i32, page: usize) -> UiLayout {
    UiLayout::new(view_w, view_h, page)
}

struct App {
    instance: HINSTANCE,
    host: HWND,
    settings: HWND,
    settings_viewport: HWND,
    settings_scroll_x: i32,
    settings_scroll_y: i32,
    // Retângulos originais, independentes de resize e posição do scroll.
    settings_control_rects: Vec<(HWND, RECT)>,
    settings_font: HFONT,
    settings_rate_font: HFONT,
    settings_ui_scale: i32,
    editor: HWND,
    editor_index: usize,
    editor_combos: [HWND; 11],
    editor_scroll_y: i32,
    editor_content_h: i32,
    editor_control_font: HFONT,
    editor_font_px: i32,
    cards: [HWND; N],
    checkboxes: [HWND; N],
    show_button: HWND,
    lock_button: HWND,
    behind_checkbox: HWND,
    snap_checkbox: HWND,
    group_checkbox: HWND,
    layout_combos: [HWND; 4],
    monitor_combos: [HWND; 3],
    disk_choice_ids: Vec<u32>,
    adapter_choice_ids: Vec<u32>,
    settings_tabs: [HWND; 5],
    settings_pages: [Vec<HWND>; 5],
    settings_page: usize,
    settings_main_brush: HBRUSH,
    settings_card_brush: HBRUSH,
    settings_heading_font: HFONT,
    settings_hover: HWND,
    editor_background_brush: HBRUSH,
    drag: Option<GroupDrag>,
    appearance_combos: [HWND; 5],
    ui_theme_combo: HWND,
    card_size_combos: [HWND; N],
    graph_checkbox: HWND,
    bar_checkbox: HWND,
    subtitle_checkbox: HWND,
    system_light: bool,
    font: HFONT,
    label_font: HFONT,
    value_font: HFONT,
    rate_font: HFONT,
    large_font: HFONT,
    compact_font: HFONT,
    taskbar_created: u32,
    monitor_areas: Vec<layout::Monitor>,
    tray: NOTIFYICONDATAW,
    tray_added: bool,
    config: Config,
    metrics: Metrics,
    history: History,
    alive: Arc<AtomicBool>,
    sampling: Arc<AtomicBool>,
    refresh_interval: Arc<AtomicU32>,
    selected_disk: Arc<AtomicU32>,
    selected_adapter: Arc<AtomicU32>,
    pending: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    closing: bool,
}

impl App {
    fn new(instance: HINSTANCE) -> Self {
        let config = Config::load();
        let refresh_interval = Arc::new(AtomicU32::new(update_interval_ms(config.update_interval)));
        let selected_disk = Arc::new(AtomicU32::new(config.monitor_disk));
        let selected_adapter = Arc::new(AtomicU32::new(config.monitor_adapter));
        Self {
            instance,
            host: null_mut(),
            settings: null_mut(),
            settings_viewport: null_mut(),
            settings_scroll_x: 0,
            settings_scroll_y: 0,
            settings_control_rects: Vec::new(),
            settings_font: null_mut(),
            settings_rate_font: null_mut(),
            settings_ui_scale: 1000,
            editor: null_mut(),
            editor_index: 0,
            editor_combos: [null_mut(); 11],
            editor_scroll_y: 0,
            editor_content_h: 0,
            editor_control_font: null_mut(),
            editor_font_px: 0,
            cards: [null_mut(); N],
            checkboxes: [null_mut(); N],
            show_button: null_mut(),
            lock_button: null_mut(),
            behind_checkbox: null_mut(),
            snap_checkbox: null_mut(),
            group_checkbox: null_mut(),
            layout_combos: [null_mut(); 4],
            monitor_combos: [null_mut(); 3],
            disk_choice_ids: Vec::new(),
            adapter_choice_ids: Vec::new(),
            settings_tabs: [null_mut(); 5],
            settings_pages: std::array::from_fn(|_| Vec::new()),
            settings_page: 0,
            settings_main_brush: null_mut(),
            settings_card_brush: null_mut(),
            settings_heading_font: null_mut(),
            settings_hover: null_mut(),
            editor_background_brush: null_mut(),
            drag: None,
            appearance_combos: [null_mut(); 5],
            ui_theme_combo: null_mut(),
            card_size_combos: [null_mut(); N],
            graph_checkbox: null_mut(),
            bar_checkbox: null_mut(),
            subtitle_checkbox: null_mut(),
            system_light: system_light_theme(),
            font: null_mut(),
            label_font: null_mut(),
            value_font: null_mut(),
            rate_font: null_mut(),
            large_font: null_mut(),
            compact_font: null_mut(),
            taskbar_created: 0,
            monitor_areas: Vec::new(),
            tray: unsafe { std::mem::zeroed() },
            tray_added: false,
            config,
            metrics: Metrics::default(),
            history: History::default(),
            alive: Arc::new(AtomicBool::new(true)),
            sampling: Arc::new(AtomicBool::new(true)),
            refresh_interval,
            selected_disk,
            selected_adapter,
            pending: Arc::new(AtomicBool::new(false)),
            worker: None,
            closing: false,
        }
    }
}

// Todas as operações abaixo só são chamadas pelo loop de mensagens, não pelo worker.
unsafe fn app() -> &'static mut App {
    &mut *APP
}

// Recupera o ícone incorporado no próprio .exe, respeitando o tamanho do Windows.
// LR_SHARED evita vazamento de HICON; os recursos compartilhados não devem ser destruídos.
unsafe fn rtime_icon(size: i32) -> HICON {
    let name = RTIME_ICON_RESOURCE as usize as *const u16;
    let icon = LoadImageW(app().instance, name, IMAGE_ICON, size, size, LR_SHARED) as HICON;
    if icon.is_null() {
        LoadIconW(null_mut(), IDI_APPLICATION)
    } else {
        icon
    }
}

unsafe fn register_class(name: &str, proc: WNDPROC, background: HBRUSH, style: u32) -> bool {
    let class_name = wide(name); // mantém o buffer UTF-16 vivo até RegisterClassW.
    let wc = WNDCLASSW {
        style,
        lpfnWndProc: proc,
        cbClsExtra: 0,
        cbWndExtra: 0,
        hInstance: app().instance,
        hIcon: rtime_icon(GetSystemMetrics(SM_CXICON)),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        hbrBackground: background,
        lpszMenuName: null(),
        lpszClassName: class_name.as_ptr(),
    };
    RegisterClassW(&wc) != 0
}

unsafe fn window(
    ex: u32,
    class: &str,
    title: &str,
    style: u32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    parent: HWND,
    id: usize,
    user: *const c_void,
) -> HWND {
    let hwnd = CreateWindowExW(
        ex,
        wide(class).as_ptr(),
        wide(title).as_ptr(),
        style,
        x,
        y,
        w,
        h,
        parent,
        id as HMENU,
        app().instance,
        user,
    );
    // Painéis com ícone próprio nas barras de título, Alt+Tab e barra de tarefas.
    if !hwnd.is_null()
        && parent.is_null()
        && (class == SETTINGS_CLASS || class == EDITOR_CLASS || class == HOST_CLASS)
    {
        SendMessageW(
            hwnd,
            WM_SETICON,
            ICON_BIG as usize,
            rtime_icon(GetSystemMetrics(SM_CXICON)) as isize,
        );
        SendMessageW(
            hwnd,
            WM_SETICON,
            ICON_SMALL as usize,
            rtime_icon(GetSystemMetrics(SM_CXSMICON)) as isize,
        );
    }
    hwnd
}

unsafe fn start_sampler() {
    let host = app().host as usize;
    let alive = app().alive.clone();
    let sampling = app().sampling.clone();
    let refresh_interval = app().refresh_interval.clone();
    let selected_disk = app().selected_disk.clone();
    let selected_adapter = app().selected_adapter.clone();
    let pending = app().pending.clone();
    app().worker = Some(thread::spawn(move || {
        let mut sampler = Sampler::new();
        let mut was_sampling = true;
        while alive.load(Ordering::Acquire) {
            if sampling.load(Ordering::Acquire) {
                if !was_sampling {
                    sampler.reset_network_baseline();
                }
                let disk = selected_disk.load(Ordering::Acquire);
                let adapter = selected_adapter.load(Ordering::Acquire);
                let result = sampler.read(disk, adapter);
                // No máximo uma mensagem de métricas em trânsito: sem acumular backlog.
                if pending
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    let boxed = Box::into_raw(Box::new(result));
                    if unsafe { PostMessageW(host as HWND, EVT_METRICS, 0, boxed as isize) } == 0 {
                        unsafe {
                            drop(Box::from_raw(boxed));
                        }
                        pending.store(false, Ordering::Release);
                    }
                }
                was_sampling = true;
            } else {
                was_sampling = false;
            }
            let total_ms = refresh_interval.load(Ordering::Acquire).clamp(500, 5000);
            let mut elapsed_ms = 0;
            while elapsed_ms < total_ms && alive.load(Ordering::Acquire) {
                let step = (total_ms - elapsed_ms).min(100);
                thread::sleep(Duration::from_millis(step as u64));
                elapsed_ms += step;
            }
        }
    }));
}

unsafe fn save_later() {
    KillTimer(app().host, TIMER_SAVE);
    SetTimer(app().host, TIMER_SAVE, 650, None);
}

unsafe fn tray_add() {
    if app().tray_added {
        Shell_NotifyIconW(NIM_DELETE, &app().tray);
    }
    let mut data: NOTIFYICONDATAW = std::mem::zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = app().host;
    data.uID = ID_TRAY;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = EVT_TRAY;
    data.hIcon = rtime_icon(GetSystemMetrics(SM_CXSMICON));
    for (dst, src) in data
        .szTip
        .iter_mut()
        .zip("Rtime".encode_utf16().chain(Some(0)))
    {
        *dst = src;
    }
    let added = Shell_NotifyIconW(NIM_ADD, &data) != 0;
    if added {
        // A estrutura da versão 4 altera os códigos de clique, mas dá suporte ao teclado.
        data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
        Shell_NotifyIconW(NIM_SETVERSION, &data);
    }
    app().tray = data;
    app().tray_added = added;
}

unsafe fn tray_menu() {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return;
    }
    let cfg = &app().config;
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_SETTINGS as usize,
        wide("Configurações").as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_VISIBILITY as usize,
        wide(if cfg.visible {
            "Ocultar widgets"
        } else {
            "Mostrar widgets"
        })
        .as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_LOCK as usize,
        wide(if cfg.locked {
            "Destravar posição"
        } else {
            "Travar posição"
        })
        .as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_SNAP as usize,
        wide(if cfg.snap_enabled {
            "Desligar alinhamento magnético"
        } else {
            "Ligar alinhamento magnético"
        })
        .as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_GROUP_MOVE as usize,
        wide(if cfg.group_move {
            "Desativar arraste em grupo"
        } else {
            "Ativar arraste em grupo"
        })
        .as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_ARRANGE_GRID as usize,
        wide("Organizar: grade por monitor").as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_ARRANGE_ROW as usize,
        wide("Organizar: linhas por monitor").as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_ARRANGE_COLUMN as usize,
        wide("Organizar: colunas por monitor").as_ptr(),
    );
    AppendMenuW(
        menu,
        MF_STRING,
        CMD_RECOVER as usize,
        wide("Recuperar posições fora da tela").as_ptr(),
    );
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, CMD_EXIT as usize, wide("Sair").as_ptr());
    let mut pos: POINT = std::mem::zeroed();
    GetCursorPos(&mut pos);
    SetForegroundWindow(app().host);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
        pos.x,
        pos.y,
        0,
        app().host,
        null(),
    );
    DestroyMenu(menu);
    PostMessageW(app().host, WM_NULL, 0, 0);
    if cmd > 0 {
        PostMessageW(app().host, WM_COMMAND, cmd as usize, 0);
    }
}

unsafe fn update_settings() {
    if app().settings.is_null() {
        return;
    }
    if !app().ui_theme_combo.is_null() {
        SendMessageW(
            app().ui_theme_combo,
            CB_SETCURSEL,
            app().config.interface_theme as usize,
            0,
        );
    }
    if !app().show_button.is_null() {
        SetWindowTextW(
            app().show_button,
            wide(if app().config.visible {
                "Ocultar widgets"
            } else {
                "Mostrar widgets"
            })
            .as_ptr(),
        );
    }
    if !app().lock_button.is_null() {
        SetWindowTextW(
            app().lock_button,
            wide(if app().config.locked {
                "Destravar posição"
            } else {
                "Travar posição"
            })
            .as_ptr(),
        );
    }
    if !app().snap_checkbox.is_null() {
        SendMessageW(
            app().snap_checkbox,
            BM_SETCHECK,
            if app().config.snap_enabled {
                BST_CHECKED
            } else {
                BST_UNCHECKED
            },
            0,
        );
    }
    if !app().group_checkbox.is_null() {
        SendMessageW(
            app().group_checkbox,
            BM_SETCHECK,
            if app().config.group_move {
                BST_CHECKED
            } else {
                BST_UNCHECKED
            },
            0,
        );
    }
    let layout_values = [
        app().config.layout_mode,
        app().config.layout_anchor,
        app().config.layout_monitor,
        app().config.layout_gap,
    ];
    for (slot, selected) in layout_values.into_iter().enumerate() {
        let combo = app().layout_combos[slot];
        if !combo.is_null() {
            SendMessageW(combo, CB_SETCURSEL, selected as usize, 0);
        }
    }
    if !app().behind_checkbox.is_null() {
        SendMessageW(
            app().behind_checkbox,
            BM_SETCHECK,
            if app().config.behind {
                BST_CHECKED
            } else {
                BST_UNCHECKED
            },
            0,
        );
    }
    // Atualiza o realce dos três modos de organização após alterações por menu/teclado.
    for id in [CMD_ARRANGE_GRID, CMD_ARRANGE_ROW, CMD_ARRANGE_COLUMN] {
        let button = GetDlgItem(app().settings_viewport, id as i32);
        if !button.is_null() {
            InvalidateRect(button, null(), 0);
        }
    }
    for i in 0..N {
        if !app().checkboxes[i].is_null() {
            SendMessageW(
                app().checkboxes[i],
                BM_SETCHECK,
                if app().config.enabled[i] {
                    BST_CHECKED
                } else {
                    BST_UNCHECKED
                },
                0,
            );
        }
    }
    for (hwnd, checked) in [
        (app().graph_checkbox, app().config.show_graphs),
        (app().bar_checkbox, app().config.show_bars),
        (app().subtitle_checkbox, app().config.show_subtitles),
    ] {
        if !hwnd.is_null() {
            SendMessageW(
                hwnd,
                BM_SETCHECK,
                if checked { BST_CHECKED } else { BST_UNCHECKED },
                0,
            );
        }
    }
    let selections = [
        app().config.style,
        app().config.theme,
        app().config.accent,
        app().config.size,
        app().config.opacity,
    ];
    for (i, selection) in selections.into_iter().enumerate() {
        let hwnd = app().appearance_combos[i];
        if !hwnd.is_null() {
            SendMessageW(hwnd, CB_SETCURSEL, selection as usize, 0);
        }
    }
    for i in 0..N {
        let hwnd = app().card_size_combos[i];
        if !hwnd.is_null() {
            SendMessageW(hwnd, CB_SETCURSEL, app().config.card_sizes[i] as usize, 0);
        }
    }
    sync_monitor_combos();
}

// Travas Win32 compartilhadas pelas DUAS janelas do aplicativo.
// O bloqueio de SC_MAXIMIZE/SC_SIZE impede também o menu Alt+Espaço e
// atalhos do Windows que solicitam maximização/redimensionamento.
fn forbidden_resize_command(command: usize) -> bool {
    matches!((command as u32) & 0xFFF0, SC_MAXIMIZE | SC_SIZE)
}

// Os paineis de configuracao e edicao nao precisam permanecer como uma
// janela iconica na area de trabalho: minimizar = ocultar (bandeja).
fn minimize_auxiliary_command(command: usize) -> bool {
    ((command as u32) & 0xFFF0) == SC_MINIMIZE
}

// Sem WS_THICKFRAME, WS_MAXIMIZEBOX e SC_SIZE/SC_MAXIMIZE, as janelas
// ja sao fixas. Nao modificar MINMAXINFO a partir de GetWindowRect():
// durante a minimizacao o Windows pode reportar retangulos iconizados e
// transformar a dimensao minima/maxima em uma pequena faixa de titulo.
// Tambem nao ocultar reentrantemente dentro do WM_SIZE: use EVT_HIDE_AUX.
fn dialog_geometry_incorrect(rect: RECT, expected_w: i32, expected_h: i32) -> bool {
    let current_w = rect.right.saturating_sub(rect.left);
    let current_h = rect.bottom.saturating_sub(rect.top);
    current_w <= 0
        || current_h <= 0
        || (current_w - expected_w).abs() > 2
        || (current_h - expected_h).abs() > 2
}

unsafe fn dialog_work_area() -> RECT {
    let mut work: RECT = std::mem::zeroed();
    if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0) == 0 {
        work.right = GetSystemMetrics(SM_CXSCREEN).max(1);
        work.bottom = GetSystemMetrics(SM_CYSCREEN).max(1);
    }
    work
}

// Abre SEMPRE a janela no estado restaurado, reparando qualquer geometria
// corrompida de uma minimizacao anterior. Um movimento manual valido fica
// preservado: so redefinimos x/y quando a janela esta iconizada ou danificada.
unsafe fn reveal_auxiliary_window(hwnd: HWND, width: i32, height: i32) {
    if hwnd.is_null() {
        return;
    }
    let was_iconic = IsIconic(hwnd) != 0;
    ShowWindow(hwnd, if was_iconic { SW_RESTORE } else { SW_SHOW });

    let work = dialog_work_area();
    let wanted_w = width.min((work.right - work.left).max(1));
    let wanted_h = height.min((work.bottom - work.top).max(1));
    let mut current: RECT = std::mem::zeroed();
    let valid = GetWindowRect(hwnd, &mut current) != 0;
    if !valid || dialog_geometry_incorrect(current, wanted_w, wanted_h) {
        let collapsed = !valid
            || current.left < -10000
            || current.top < -10000
            || (current.right - current.left) < wanted_w / 2
            || (current.bottom - current.top) < wanted_h / 2;
        let x = if collapsed {
            work.left + ((work.right - work.left) - wanted_w) / 2
        } else {
            current.left
        };
        let y = if collapsed {
            work.top + ((work.bottom - work.top) - wanted_h) / 2
        } else {
            current.top
        };
        SetWindowPos(
            hwnd,
            null_mut(),
            x,
            y,
            wanted_w,
            wanted_h,
            SWP_NOZORDER | SWP_NOACTIVATE | SWP_FRAMECHANGED | SWP_SHOWWINDOW,
        );
    }
    SetForegroundWindow(hwnd);
}

unsafe fn is_key_for_fixed_window(target: HWND, settings: HWND, editor: HWND) -> bool {
    (!settings.is_null() && (target == settings || IsChild(settings, target) != 0))
        || (!editor.is_null() && (target == editor || IsChild(editor, target) != 0))
}

unsafe fn show_settings() {
    if app().settings.is_null() {
        // Respeita a área útil do monitor principal (taskbar inclusive), em vez
        // de criar uma janela maior que a tela disponível.
        let mut work: RECT = std::mem::zeroed();
        if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0) == 0
        {
            work.right = GetSystemMetrics(SM_CXSCREEN);
            work.bottom = GetSystemMetrics(SM_CYSCREEN);
        }
        let available_w = (work.right - work.left).max(480);
        let available_h = (work.bottom - work.top).max(320);
        let win_w = SETTINGS_DEFAULT_WIDTH.min(available_w);
        let win_h = SETTINGS_DEFAULT_HEIGHT.min(available_h);
        let win_x = work.left + (available_w - win_w) / 2;
        let win_y = work.top + (available_h - win_h) / 2;
        app().settings = window(
            WS_EX_APPWINDOW,
            SETTINGS_CLASS,
            "Rtime - Configurações",
            FIXED_DIALOG_STYLE,
            win_x,
            win_y,
            win_w,
            win_h,
            null_mut(),
            0,
            null(),
        );
    }
    if !app().settings.is_null() {
        refill_monitor_sources();
        update_settings();
        reveal_auxiliary_window(
            app().settings,
            SETTINGS_DEFAULT_WIDTH,
            SETTINGS_DEFAULT_HEIGHT,
        );
    }
}

unsafe fn update_visibility() {
    let mut any = false;
    for i in 0..N {
        let hwnd = app().cards[i];
        if hwnd.is_null() {
            continue;
        }
        let show = app().config.visible && app().config.enabled[i];
        if show {
            any = true;
            if IsWindowVisible(hwnd) == 0 {
                ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                let place = if app().config.behind {
                    HWND_BOTTOM
                } else {
                    HWND_TOP
                };
                SetWindowPos(
                    hwnd,
                    place,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                );
            }
        } else if IsWindowVisible(hwnd) != 0 {
            ShowWindow(hwnd, SW_HIDE);
        }
    }
    app().sampling.store(any, Ordering::Release);
    update_settings();
}

// Posições persistidas permanecem em coordenadas físicas da área de trabalho virtual.
// As três formas de organização preservam o monitor atual de cada cartão.
unsafe fn screen_rect(hwnd: HWND) -> Option<Rect> {
    if hwnd.is_null() {
        return None;
    }
    let mut rect: RECT = std::mem::zeroed();
    if GetWindowRect(hwnd, &mut rect) == 0 {
        return None;
    }
    Some(Rect {
        left: rect.left,
        top: rect.top,
        right: rect.right,
        bottom: rect.bottom,
    })
}

unsafe fn physical_size(i: usize, monitors: &[layout::Monitor], reference: Rect) -> (i32, i32) {
    let logical = dimensions(app().config.card_sizes[i]);
    let dpi = nearest_monitor(monitors, reference)
        .map(|idx| monitors[idx].dpi)
        .unwrap_or(96);
    (scaled(logical.0, dpi), scaled(logical.1, dpi))
}

unsafe fn set_card_region(index: usize) {
    let hwnd = app().cards[index];
    if hwnd.is_null() {
        return;
    }
    let (w, h) = screen_rect(hwnd)
        .map(|rect| (rect.width(), rect.height()))
        .unwrap_or(dimensions(app().config.card_sizes[index]));
    let individual_style = visual_for(&app().config, index, app().system_light).style;
    let rounding = if individual_style == 1 { 18 } else { 26 };
    let dpi = GetDpiForWindow(hwnd).max(96);
    let rounded = scaled(rounding, dpi);
    let region = CreateRoundRectRgn(0, 0, w + 1, h + 1, rounded, rounded);
    if !region.is_null() && SetWindowRgn(hwnd, region, 1) == 0 {
        DeleteObject(region);
    }
}

unsafe fn recovery_positions() {
    let monitors = app().monitor_areas.clone();
    for i in 0..N {
        let hwnd = app().cards[i];
        let Some(current) = screen_rect(hwnd) else {
            continue;
        };
        let target = recover(current, &monitors);
        if target.left != current.left || target.top != current.top {
            SetWindowPos(
                hwnd,
                null_mut(),
                target.left,
                target.top,
                0,
                0,
                SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
        }
        app().config.positions[i] = (target.left, target.top);
    }
    save_later();
}

unsafe fn reorganize(mode: Arrangement) {
    let monitors = app().monitor_areas.clone();
    let current: Vec<Rect> = (0..N)
        .map(|i| {
            screen_rect(app().cards[i]).unwrap_or_else(|| {
                let (x, y) = app().config.positions[i];
                let (w, h) = dimensions(app().config.card_sizes[i]);
                Rect::new(x, y, w, h)
            })
        })
        .collect();
    let sizes: Vec<(i32, i32)> = current
        .iter()
        .map(|rect| (rect.width(), rect.height()))
        .collect();
    let enabled: Vec<bool> = (0..N).map(|i| app().config.enabled[i]).collect();
    let monitor_target = if app().config.layout_monitor == 0 {
        None
    } else {
        Some((app().config.layout_monitor - 1) as usize).filter(|&i| i < monitors.len())
    };
    let targets = arrange_anchored(
        &current,
        &enabled,
        &sizes,
        &monitors,
        mode,
        Anchor::from_index(app().config.layout_anchor),
        monitor_target,
        app().config.layout_gap,
    );
    for i in 0..N {
        let hwnd = app().cards[i];
        if hwnd.is_null() || !enabled[i] {
            continue;
        }
        let next = targets[i];
        SetWindowPos(
            hwnd,
            null_mut(),
            next.left,
            next.top,
            0,
            0,
            SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        app().config.positions[i] = (next.left, next.top);
    }
    save_later();
}

unsafe fn rescale_and_recover() {
    app().monitor_areas = desktop::monitors();
    let monitors = app().monitor_areas.clone();
    for i in 0..N {
        let hwnd = app().cards[i];
        let Some(current) = screen_rect(hwnd) else {
            continue;
        };
        let (w, h) = physical_size(i, &monitors, current);
        let new_rect = recover(Rect::new(current.left, current.top, w, h), &monitors);
        SetWindowPos(
            hwnd,
            null_mut(),
            new_rect.left,
            new_rect.top,
            w,
            h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        set_card_region(i);
        app().config.positions[i] = (new_rect.left, new_rect.top);
        InvalidateRect(hwnd, null(), 0);
    }
    save_later();
}

unsafe fn command(cmd: u32) {
    match cmd {
        CMD_SETTINGS => show_settings(),
        CMD_OPEN_EDITOR => show_editor(0),
        CMD_VISIBILITY => {
            app().config.visible = !app().config.visible;
            update_visibility();
            save_later();
        }
        CMD_LOCK => {
            app().config.locked = !app().config.locked;
            update_settings();
            save_later();
        }
        CMD_SNAP => {
            app().config.snap_enabled = !app().config.snap_enabled;
            update_settings();
            save_later();
        }
        CMD_GROUP_MOVE => {
            app().config.group_move = !app().config.group_move;
            update_settings();
            save_later();
        }
        CMD_ARRANGE_GRID | CMD_RESET_POSITIONS => {
            app().config.layout_mode = 0;
            reorganize(Arrangement::Grid);
            update_settings();
        }
        CMD_ARRANGE_ROW => {
            app().config.layout_mode = 1;
            reorganize(Arrangement::Row);
            update_settings();
        }
        CMD_ARRANGE_COLUMN => {
            app().config.layout_mode = 2;
            reorganize(Arrangement::Column);
            update_settings();
        }
        CMD_APPLY_LAYOUT => match app().config.layout_mode {
            1 => reorganize(Arrangement::Row),
            2 => reorganize(Arrangement::Column),
            _ => reorganize(Arrangement::Grid),
        },
        CMD_RECOVER => recovery_positions(),
        CMD_BEHIND => {
            app().config.behind = !app().config.behind;
            for &hwnd in &app().cards {
                if !hwnd.is_null() && IsWindowVisible(hwnd) != 0 {
                    let place = if app().config.behind {
                        HWND_BOTTOM
                    } else {
                        HWND_TOP
                    };
                    SetWindowPos(
                        hwnd,
                        place,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
            update_settings();
            save_later();
        }
        CMD_EXIT => quit(),
        _ if (CMD_CARD_BASE..CMD_CARD_BASE + N as u32).contains(&cmd) => {
            let index = (cmd - CMD_CARD_BASE) as usize;
            app().config.enabled[index] = !app().config.enabled[index];
            update_visibility();
            save_later();
        }
        _ => {}
    }
}

unsafe fn quit() {
    if app().closing {
        return;
    }
    app().closing = true;
    KillTimer(app().host, TIMER_SAVE);
    app().config.save();
    app().alive.store(false, Ordering::Release);
    if let Some(worker) = app().worker.take() {
        let _ = worker.join();
    }
    if app().tray_added {
        Shell_NotifyIconW(NIM_DELETE, &app().tray);
        app().tray_added = false;
    }
    for index in 0..N {
        if !app().cards[index].is_null() {
            DestroyWindow(app().cards[index]);
            app().cards[index] = null_mut();
        }
    }
    if !app().editor.is_null() {
        DestroyWindow(app().editor);
        app().editor = null_mut();
    }
    if !app().settings.is_null() {
        DestroyWindow(app().settings);
        app().settings = null_mut();
    }
    DestroyWindow(app().host);
}

unsafe extern "system" fn host_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    if !APP.is_null() && app().taskbar_created != 0 && message == app().taskbar_created {
        tray_add();
        return 0;
    }
    match message {
        EVT_TRAY => {
            let event = (l as u32) & 0xFFFF;
            match event {
                WM_RBUTTONUP | WM_CONTEXTMENU => tray_menu(),
                WM_LBUTTONUP | WM_LBUTTONDBLCLK | NIN_SELECT => show_settings(),
                _ => {}
            }
            0
        }
        EVT_METRICS => {
            if l != 0 {
                let metrics = *Box::from_raw(l as *mut Metrics);
                app().metrics = metrics;
                app().history.push(metrics);
                app().pending.store(false, Ordering::Release);
                if !app().editor.is_null() && IsWindowVisible(app().editor) != 0 {
                    InvalidateRect(app().editor, null(), 0);
                }
                // Atualiza também os valores reais na aba Widgets.
                if !app().settings.is_null()
                    && app().settings_page == 1
                    && IsWindowVisible(app().settings) != 0
                {
                    if !app().settings_viewport.is_null() {
                        InvalidateRect(app().settings_viewport, null(), 0);
                    }
                }
                if app().config.visible {
                    for i in 0..N {
                        if app().config.enabled[i] && !app().cards[i].is_null() {
                            InvalidateRect(app().cards[i], null(), 0);
                        }
                    }
                }
            }
            0
        }
        EVT_OPEN => {
            show_settings();
            0
        }
        WM_DISPLAYCHANGE => {
            rescale_and_recover();
            if !app().settings.is_null() {
                refill_monitor_combo();
                update_settings();
            }
            0
        }
        WM_DEVICECHANGE => {
            if !app().settings.is_null() && IsWindowVisible(app().settings) != 0 {
                refill_monitor_sources();
            }
            0
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED => {
            app().system_light = system_light_theme();
            if app().config.theme == 2 {
                update_appearance();
            }
            // A área útil pode mudar quando a barra de tarefas é movida.
            if message == WM_SETTINGCHANGE {
                rescale_and_recover();
            }
            0
        }
        WM_COMMAND => {
            command((w & 0xFFFF) as u32);
            0
        }
        WM_TIMER if w == TIMER_SAVE => {
            KillTimer(hwnd, TIMER_SAVE);
            app().config.save();
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, message, w, l),
    }
}

unsafe fn begin_group_drag(index: usize) {
    app().drag = None;
    if !app().config.group_move || app().config.locked {
        return;
    }
    let Some(leader_start) = screen_rect(app().cards[index]) else {
        return;
    };
    let members: Vec<(usize, Rect)> = (0..N)
        .filter(|&j| {
            app().config.enabled[j]
                && !app().cards[j].is_null()
                && IsWindowVisible(app().cards[j]) != 0
        })
        .filter_map(|j| screen_rect(app().cards[j]).map(|r| (j, r)))
        .collect();
    if members.len() > 1 {
        app().drag = Some(GroupDrag {
            leader: index,
            leader_start,
            members,
        });
    }
}

unsafe fn move_group(index: usize, bounds: &mut RECT) -> bool {
    let Some(drag) = app().drag.as_ref() else {
        return false;
    };
    if drag.leader != index {
        return false;
    }
    let members = drag.members.clone();
    let start = drag.leader_start;
    let mut dx = bounds.left.saturating_sub(start.left);
    let mut dy = bounds.top.saturating_sub(start.top);
    // Alinhar todo o conjunto às bordas, não somente o cartão arrastado.
    if app().config.snap_enabled {
        let group_left = members
            .iter()
            .map(|(_, r)| r.left)
            .min()
            .unwrap_or(start.left);
        let group_top = members
            .iter()
            .map(|(_, r)| r.top)
            .min()
            .unwrap_or(start.top);
        let group_right = members
            .iter()
            .map(|(_, r)| r.right)
            .max()
            .unwrap_or(start.right);
        let group_bottom = members
            .iter()
            .map(|(_, r)| r.bottom)
            .max()
            .unwrap_or(start.bottom);
        let bounds_group = Rect {
            left: group_left,
            top: group_top,
            right: group_right,
            bottom: group_bottom,
        };
        let moved_group =
            bounds_group.moved(group_left.saturating_add(dx), group_top.saturating_add(dy));
        let monitors = &app().monitor_areas;
        if let Some(monitor_index) = nearest_monitor(monitors, moved_group) {
            let monitor = monitors[monitor_index];
            let snapped = snap_moving(moved_group, monitor, std::iter::empty());
            dx = snapped.left.saturating_sub(group_left);
            dy = snapped.top.saturating_sub(group_top);
            // Evita perder o conjunto inteiro fora da área útil quando ele cabe na tela.
            if moved_group.width() <= monitor.work.width()
                && moved_group.height() <= monitor.work.height()
            {
                let aligned = recover(
                    bounds_group.moved(group_left.saturating_add(dx), group_top.saturating_add(dy)),
                    &[monitor],
                );
                dx = aligned.left.saturating_sub(group_left);
                dy = aligned.top.saturating_sub(group_top);
            }
        }
    }
    let leader = start.moved(start.left.saturating_add(dx), start.top.saturating_add(dy));
    *bounds = RECT {
        left: leader.left,
        top: leader.top,
        right: leader.right,
        bottom: leader.bottom,
    };
    for (member_index, original) in members {
        if member_index == index {
            continue;
        }
        let next = original.moved(
            original.left.saturating_add(dx),
            original.top.saturating_add(dy),
        );
        SetWindowPos(
            app().cards[member_index],
            null_mut(),
            next.left,
            next.top,
            0,
            0,
            SWP_NOZORDER | SWP_NOSIZE | SWP_NOACTIVATE,
        );
        app().config.positions[member_index] = (next.left, next.top);
    }
    true
}

unsafe extern "system" fn card_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match message {
        WM_NCCREATE => {
            let cs = &*(l as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, (cs.lpCreateParams as isize) - 1);
            1
        }
        WM_CREATE => {
            let index = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
            let (width, height) = dimensions(app().config.card_sizes[index.min(N - 1)]);
            let individual_style =
                visual_for(&app().config, index.min(N - 1), app().system_light).style;
            let rounding = if individual_style == 1 { 18 } else { 26 };
            let region = CreateRoundRectRgn(0, 0, width + 1, height + 1, rounding, rounding);
            if !region.is_null() && SetWindowRgn(hwnd, region, 1) == 0 {
                DeleteObject(region);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let i = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
            if i < N {
                paint_card(hwnd, i);
            }
            0
        }
        WM_NCHITTEST => {
            if app().config.locked {
                HTCLIENT as isize
            } else {
                HTCAPTION as isize
            }
        }
        WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
        WM_ENTERSIZEMOVE => {
            let index = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
            if index < N {
                begin_group_drag(index);
            }
            0
        }
        WM_MOVING => {
            let index = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
            if index < N
                && !app().config.locked
                && l != 0
                && move_group(index, &mut *(l as *mut RECT))
            {
                return 1;
            }
            if !app().config.locked && app().config.snap_enabled && l != 0 {
                let i = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
                if i < N {
                    let bounds = &mut *(l as *mut RECT);
                    let moving = Rect {
                        left: bounds.left,
                        top: bounds.top,
                        right: bounds.right,
                        bottom: bounds.bottom,
                    };
                    let monitors = app().monitor_areas.clone();
                    if let Some(index) = nearest_monitor(&monitors, moving) {
                        let others: Vec<Rect> = (0..N)
                            .filter(|&j| {
                                j != i
                                    && app().config.enabled[j]
                                    && IsWindowVisible(app().cards[j]) != 0
                            })
                            .filter_map(|j| screen_rect(app().cards[j]))
                            .collect();
                        let target = snap_moving(moving, monitors[index], others.into_iter());
                        *bounds = RECT {
                            left: target.left,
                            top: target.top,
                            right: target.right,
                            bottom: target.bottom,
                        };
                        return 1;
                    }
                }
            }
            0
        }
        WM_DPICHANGED => {
            let i = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
            if i < N {
                let dpi = ((w & 0xFFFF) as u32).clamp(72, 384);
                let (base_w, base_h) = dimensions(app().config.card_sizes[i]);
                let suggested = &*(l as *const RECT);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    scaled(base_w, dpi),
                    scaled(base_h, dpi),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                app().config.positions[i] = (suggested.left, suggested.top);
                set_card_region(i);
                save_later();
            }
            0
        }
        WM_EXITSIZEMOVE => {
            let i = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
            if app().drag.is_some() {
                for j in 0..N {
                    if let Some(rect) = screen_rect(app().cards[j]) {
                        app().config.positions[j] = (rect.left, rect.top);
                    }
                }
                app().drag = None;
            }
            if i < N {
                let mut rect: RECT = std::mem::zeroed();
                GetWindowRect(hwnd, &mut rect);
                app().config.positions[i] = (rect.left, rect.top);
                if app().config.behind {
                    SetWindowPos(
                        hwnd,
                        HWND_BOTTOM,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
                save_later();
            }
            0
        }
        WM_CONTEXTMENU | WM_NCRBUTTONUP => {
            let i = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as usize;
            if i < N {
                show_editor(i);
            }
            0
        }
        WM_LBUTTONDBLCLK | WM_NCLBUTTONDBLCLK => {
            show_settings();
            0
        }
        _ => DefWindowProcW(hwnd, message, w, l),
    }
}

fn is_settings_checkbox(class: &str, style: u32) -> bool {
    const BUTTON_TYPE_MASK: u32 = 0x000F;
    class == "BUTTON" && (style & BUTTON_TYPE_MASK) == (BS_AUTOCHECKBOX as u32)
}

unsafe fn make_control(
    class: &str,
    label: &str,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: HWND,
    id: u32,
) -> HWND {
    let focusable = if class == "BUTTON" || class == "COMBOBOX" {
        WS_TABSTOP
    } else {
        0
    };
    // Os checkboxes da janela de configurações usam um controle nativo proprio.
    // Preserva BM_GETCHECK/BM_SETCHECK e BN_CLICKED usados pela lógica existente.
    // Tipos BUTTON usam os 4 bits inferiores. BS_OWNERDRAW (0x0B) compartilha
    // bits com BS_AUTOCHECKBOX (0x03): um teste bit-a-bit classificava os
    // botoes da navegacao/acoes como checkboxes e quebrava a interface.
    let styled_checkbox = is_settings_checkbox(class, style);
    let control_class = if styled_checkbox {
        SETTINGS_CHECK_CLASS
    } else {
        class
    };
    let control_style = if styled_checkbox { 0 } else { style };
    let hwnd = window(
        0,
        control_class,
        label,
        WS_CHILD | WS_VISIBLE | focusable | control_style,
        x,
        y,
        width,
        height,
        parent,
        id as usize,
        null(),
    );
    if !hwnd.is_null() && !app().font.is_null() {
        SendMessageW(hwnd, WM_SETFONT, app().font as usize, 1);
    }
    hwnd
}

unsafe fn make_combo(
    parent: HWND,
    id: u32,
    x: i32,
    y: i32,
    width: i32,
    labels: &[&str],
    initial: u32,
    dark_owner_draw: bool,
) -> HWND {
    // O editor anterior mantém ComboBox padrão; apenas Configurações usa owner-draw.
    // CBS_HASSTRINGS obriga o ComboBox owner-draw a COPIAR os textos.
    // Sem esta flag, CB_ADDSTRING trata lParam como dados e CB_GETLBTEXT
    // devolve enderecos/dados, causando caracteres aleatorios.
    let owner_style = if dark_owner_draw {
        (CBS_OWNERDRAWFIXED as u32) | (CBS_HASSTRINGS as u32)
    } else {
        0
    };
    let style = CBS_DROPDOWNLIST as u32 | WS_VSCROLL | owner_style;
    let combo = make_control("COMBOBOX", "", style, x, y, width, 230, parent, id);
    if !combo.is_null() {
        for label in labels {
            SendMessageW(combo, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
        }
        SendMessageW(combo, CB_SETCURSEL, initial as usize, 0);
        // Selecao e itens com altura coerente com o restante da interface.
        // CB_SETITEMHEIGHT usa -1 (usize::MAX) para a area de selecao.
        SendMessageW(combo, CB_SETITEMHEIGHT, usize::MAX, 34);
        SendMessageW(combo, CB_SETITEMHEIGHT, 0, 34);
    }
    combo
}

// Navegação por páginas: mantém os controles existentes em janelas Win32 leves.
unsafe fn page_control(
    page: usize,
    class: &str,
    label: &str,
    style: u32,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    parent: HWND,
    id: u32,
) -> HWND {
    let viewport = app().settings_viewport;
    let (actual_parent, local_x) = if viewport.is_null() {
        (parent, x)
    } else {
        (viewport, x - SETTINGS_SIDEBAR_WIDTH)
    };
    let hwnd = make_control(
        class,
        label,
        style,
        local_x,
        y,
        width,
        height,
        actual_parent,
        id,
    );
    if !hwnd.is_null() {
        app().settings_pages[page].push(hwnd);
        app().settings_control_rects.push((
            hwnd,
            RECT {
                left: local_x,
                top: y,
                right: local_x + width,
                bottom: y + height,
            },
        ));
    }
    hwnd
}

unsafe fn page_label(page: usize, parent: HWND, text: &str, x: i32, y: i32, width: i32) {
    page_control(page, "STATIC", text, 0, x, y, width, 23, parent, 0);
}

// Títulos e subtítulos já são pintados em paint_settings_window.
// Não criar controles STATIC adicionais: causava duplicação e sobreposição.
unsafe fn page_heading(_page: usize, _parent: HWND, _title: &str, _description: &str) {}

unsafe fn page_combo(
    page: usize,
    parent: HWND,
    id: u32,
    x: i32,
    y: i32,
    width: i32,
    labels: &[&str],
    selection: u32,
) -> HWND {
    let viewport = app().settings_viewport;
    let (actual_parent, local_x) = if viewport.is_null() {
        (parent, x)
    } else {
        (viewport, x - SETTINGS_SIDEBAR_WIDTH)
    };
    let hwnd = make_combo(
        actual_parent,
        id,
        local_x,
        y,
        width,
        labels,
        selection,
        true,
    );
    if !hwnd.is_null() {
        app().settings_pages[page].push(hwnd);
        app().settings_control_rects.push((
            hwnd,
            RECT {
                left: local_x,
                top: y,
                right: local_x + width,
                bottom: y + 230,
            },
        ));
    }
    hwnd
}

unsafe fn page_action(
    page: usize,
    parent: HWND,
    caption: &str,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    command: u32,
) -> HWND {
    page_control(
        page,
        "BUTTON",
        caption,
        BS_OWNERDRAW as u32,
        x,
        y,
        width,
        height,
        parent,
        command,
    )
}

unsafe fn show_settings_page(hwnd: HWND, target: usize) {
    app().settings_page = target.min(4);
    for page in 0..5 {
        for &child in &app().settings_pages[page] {
            if !child.is_null() {
                ShowWindow(
                    child,
                    if page == app().settings_page {
                        SW_SHOW
                    } else {
                        SW_HIDE
                    },
                );
            }
        }
    }
    for &button in &app().settings_tabs {
        if !button.is_null() {
            InvalidateRect(button, null(), 0);
        }
    }
    // A nova página começa no topo; a barra lateral e o rodapé não rolam.
    let viewport = app().settings_viewport;
    if !viewport.is_null() {
        app().settings_scroll_x = 0;
        app().settings_scroll_y = 0;
        settings_scroll_ranges(viewport);
        InvalidateRect(viewport, null(), 0);
    }
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn refill_monitor_combo() {
    let combo = app().layout_combos[2];
    if combo.is_null() {
        return;
    }
    SendMessageW(combo, CB_RESETCONTENT, 0, 0);
    for label in ["Manter monitores atuais", "Monitor principal"] {
        SendMessageW(combo, CB_ADDSTRING, 0, wide(label).as_ptr() as isize);
    }
    for index in 1..app().monitor_areas.len() {
        let monitor = app().monitor_areas[index];
        let label = format!(
            "Monitor {}  •  {} × {}",
            index + 1,
            monitor.screen.width(),
            monitor.screen.height()
        );
        SendMessageW(combo, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
    }
    // Dois itens fixos apenas quando não há tela; durante uso normal há pelo menos uma.
    let valid = if app().monitor_areas.len() > 0 {
        app().monitor_areas.len() as u32
    } else {
        1
    };
    app().config.layout_monitor = app().config.layout_monitor.min(valid);
    SendMessageW(combo, CB_SETCURSEL, app().config.layout_monitor as usize, 0);
}

unsafe fn read_layout_controls(id: u32) {
    let values: [u32; 4] = std::array::from_fn(|i| selected_combo(app().layout_combos[i]));
    app().config.layout_mode = values[0].min(2);
    // A ancora é definida pelos nove botões, e não por um ComboBox oculto.
    if !app().layout_combos[1].is_null() {
        app().config.layout_anchor = values[1].min(8);
    }
    app().config.layout_monitor = values[2].min(app().monitor_areas.len() as u32);
    app().config.layout_gap = values[3].min(3);
    if id == CMD_GROUP_MOVE {
        app().config.group_move = checked(app().group_checkbox);
    }
    if id == CMD_SNAP {
        app().config.snap_enabled = checked(app().snap_checkbox);
    }
    save_later();
}

unsafe fn create_settings_controls(hwnd: HWND) {
    // Layout Fluent Light: coordenadas compactas, já ajustadas para a janela.
    // Os 24 px à esquerda dos textos na sidebar ficam vazios para futuros ícones PNG.
    for (i, title) in [
        "Aparência",
        "Widgets",
        "Layout",
        "Comportamento",
        "Monitoramento",
    ]
    .iter()
    .enumerate()
    {
        app().settings_tabs[i] = make_control(
            "BUTTON",
            title,
            BS_OWNERDRAW as u32,
            17,
            137 + (i as i32) * 61,
            214,
            46,
            hwnd,
            CMD_TAB_BASE + i as u32,
        );
    }

    // Aparência — opções centrais e painel de prévia à direita.
    page_heading(
        0,
        hwnd,
        "Aparência",
        "Estilo, contraste e densidade visual dos cartões.",
    );
    page_label(0, hwnd, "Estilo visual", 296, 176, 500);
    app().appearance_combos[0] = page_combo(
        0,
        hwnd,
        CMD_STYLE,
        296,
        204,
        532,
        &STYLES,
        app().config.style,
    );
    page_label(0, hwnd, "Modo de cor", 296, 268, 248);
    app().appearance_combos[1] = page_combo(
        0,
        hwnd,
        CMD_THEME,
        296,
        294,
        252,
        &THEMES,
        app().config.theme,
    );
    page_label(0, hwnd, "Tamanho dos widgets", 568, 268, 260);
    app().appearance_combos[3] =
        page_combo(0, hwnd, CMD_SIZE, 568, 294, 260, &SIZES, app().config.size);
    page_label(0, hwnd, "Cor de destaque", 296, 354, 250);
    app().appearance_combos[2] = page_combo(
        0,
        hwnd,
        CMD_ACCENT,
        296,
        380,
        252,
        &ACCENTS,
        app().config.accent,
    );
    page_label(0, hwnd, "Opacidade dos cartões", 568, 354, 260);
    app().appearance_combos[4] = page_combo(
        0,
        hwnd,
        CMD_OPACITY,
        568,
        380,
        260,
        &OPACITIES,
        app().config.opacity,
    );
    app().graph_checkbox = page_control(
        0,
        "BUTTON",
        "Gráficos históricos",
        BS_AUTOCHECKBOX as u32,
        296,
        463,
        230,
        30,
        hwnd,
        CMD_SHOW_GRAPHS,
    );
    app().bar_checkbox = page_control(
        0,
        "BUTTON",
        "Barras de progresso",
        BS_AUTOCHECKBOX as u32,
        558,
        463,
        258,
        30,
        hwnd,
        CMD_SHOW_BARS,
    );
    app().subtitle_checkbox = page_control(
        0,
        "BUTTON",
        "Textos secundários",
        BS_AUTOCHECKBOX as u32,
        296,
        509,
        240,
        30,
        hwnd,
        CMD_SHOW_SUBTITLES,
    );
    page_action(
        0,
        hwnd,
        "Personalizar widgets",
        886,
        493,
        228,
        46,
        CMD_OPEN_EDITOR,
    );
    // Este rótulo é pintado no painel com a geometria compartilhada.
    // Evita faixas de fundo do controle STATIC sobre o botão owner-draw.
    app().ui_theme_combo = page_combo(
        0,
        hwnd,
        CMD_UI_THEME,
        886,
        655,
        228,
        &INTERFACE_THEMES,
        app().config.interface_theme,
    );

    // Widgets — grade de seis cartões com fontes e tamanhos independentes.
    page_heading(
        1,
        hwnd,
        "Seus widgets",
        "Escolha os indicadores e configure o tamanho de cada cartão.",
    );
    for index in 0..N {
        let col = (index % 3) as i32;
        let row = (index / 3) as i32;
        let x = 275 + col * 289;
        let y = 145 + row * 187;
        app().checkboxes[index] = page_control(
            1,
            "BUTTON",
            NAMES[index],
            BS_AUTOCHECKBOX as u32,
            x + 17,
            y + 16,
            236,
            30,
            hwnd,
            CMD_CARD_BASE + index as u32,
        );
        // À esquerda entra uma miniatura GDI com a métrica atual; o seletor fica à direita.
        app().card_size_combos[index] = page_combo(
            1,
            hwnd,
            CMD_CARD_SIZE_BASE + index as u32,
            x + 157,
            y + 109,
            92,
            &SIZES,
            app().config.card_sizes[index],
        );
    }

    // Layout — mantém comandos de grade/linhas/colunas e seleção de âncora.
    page_heading(
        2,
        hwnd,
        "Organização do desktop",
        "Posicione e movimente seus cartões em grupo.",
    );
    // O título "Organização" é desenhado pelo layout responsivo.
    page_action(2, hwnd, "Grade", 294, 203, 125, 77, CMD_ARRANGE_GRID);
    page_action(2, hwnd, "Linhas", 429, 203, 125, 77, CMD_ARRANGE_ROW);
    page_action(2, hwnd, "Colunas", 564, 203, 125, 77, CMD_ARRANGE_COLUMN);
    page_label(2, hwnd, "Espaçamento", 295, 296, 350);
    app().layout_combos[3] = page_combo(
        2,
        hwnd,
        CMD_LAYOUT_GAP,
        294,
        324,
        396,
        &LAYOUT_GAPS,
        app().config.layout_gap,
    );
    page_label(2, hwnd, "Monitor de destino", 295, 379, 355);
    app().layout_combos[2] = page_combo(
        2,
        hwnd,
        CMD_LAYOUT_MONITOR,
        294,
        407,
        396,
        &[],
        app().config.layout_monitor,
    );
    page_label(2, hwnd, "Modo selecionado", 295, 464, 355);
    app().layout_combos[0] = page_combo(
        2,
        hwnd,
        CMD_LAYOUT_MODE,
        294,
        492,
        396,
        &LAYOUT_MODES,
        app().config.layout_mode,
    );
    // O título "Posição do conjunto" já é desenhado pelo GDI no cartão à direita.
    // O texto acessível não é pintado no quadrado, mas identifica cada âncora.
    let anchor_names = [
        "Superior esquerdo",
        "Superior centro",
        "Superior direito",
        "Centro esquerdo",
        "Centro",
        "Centro direito",
        "Inferior esquerdo",
        "Inferior centro",
        "Inferior direito",
    ];
    for row in 0..3 {
        for col in 0..3 {
            let index = row * 3 + col;
            // As posições definitivas usam UiLayout::anchor_tile no reflow.
            page_action(
                2,
                hwnd,
                anchor_names[index],
                761 + col as i32 * 63,
                218 + row as i32 * 51,
                52,
                42,
                CMD_LAYOUT_ANCHOR_TILE_BASE + index as u32,
            );
        }
    }
    app().group_checkbox = page_control(
        2,
        "BUTTON",
        "Mover widgets em grupo",
        BS_AUTOCHECKBOX as u32,
        743,
        405,
        355,
        31,
        hwnd,
        CMD_GROUP_MOVE,
    );
    app().snap_checkbox = page_control(
        2,
        "BUTTON",
        "Alinhamento magnético",
        BS_AUTOCHECKBOX as u32,
        743,
        444,
        355,
        31,
        hwnd,
        CMD_SNAP,
    );
    page_action(
        2,
        hwnd,
        "Aplicar organização",
        744,
        509,
        354,
        48,
        CMD_APPLY_LAYOUT,
    );
    page_label(2, hwnd, "Destrave os cartões para arrastar.", 744, 578, 355);

    // Comportamento — filas horizontais com ação independente à direita.
    page_heading(
        3,
        hwnd,
        "Comportamento",
        "Controles do desktop, da bandeja e das posições.",
    );
    // Checkbox nativo personalizado, em formato de interruptor à direita da primeira linha.
    app().behind_checkbox = page_control(
        3,
        "BUTTON",
        "",
        BS_AUTOCHECKBOX as u32,
        1014,
        184,
        82,
        36,
        hwnd,
        CMD_BEHIND,
    );
    app().lock_button = page_action(3, hwnd, "Travar / destravar", 855, 282, 245, 44, CMD_LOCK);
    app().show_button = page_action(
        3,
        hwnd,
        "Mostrar / ocultar",
        855,
        393,
        245,
        44,
        CMD_VISIBILITY,
    );
    page_action(3, hwnd, "Recuperar widgets", 855, 512, 245, 44, CMD_RECOVER);

    // Monitoramento — coleta permanece na mesma thread que a v0.3.10.
    page_heading(
        4,
        hwnd,
        "Fontes e atualização",
        "Selecione o volume e o adaptador de rede monitorados.",
    );
    page_label(4, hwnd, "Unidade de disco", 296, 227, 350);
    app().monitor_combos[0] = page_combo(4, hwnd, CMD_MONITOR_DISK, 295, 263, 362, &[], 0);
    page_label(4, hwnd, "Adaptador de rede", 296, 357, 350);
    app().monitor_combos[1] = page_combo(4, hwnd, CMD_MONITOR_ADAPTER, 295, 393, 362, &[], 0);
    page_label(4, hwnd, "Atualização dos dados", 738, 227, 356);
    app().monitor_combos[2] = page_combo(
        4,
        hwnd,
        CMD_MONITOR_INTERVAL,
        737,
        263,
        364,
        &UPDATE_INTERVALS,
        app().config.update_interval,
    );
    page_action(
        4,
        hwnd,
        "Atualizar lista de dispositivos",
        737,
        351,
        364,
        48,
        CMD_MONITOR_REFRESH,
    );
    refill_monitor_sources();

    // Botão persistente — o rodapé é desenhado em paint_settings_window.
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    make_control(
        "BUTTON",
        "Fechar",
        BS_OWNERDRAW as u32,
        (bounds.right - 130).max(SETTINGS_SIDEBAR_WIDTH),
        (bounds.bottom - SETTINGS_FOOTER_HEIGHT + 12).max(0),
        112,
        42,
        hwnd,
        CMD_SETTINGS_CLOSE,
    );
    refill_monitor_combo();
    show_settings_page(hwnd, app().settings_page);
}

// Reconstroi os ComboBoxes mantendo as selecoes por codigo da unidade/InterfaceIndex.
unsafe fn refill_monitor_sources() {
    let disk = app().monitor_combos[0];
    let adapter = app().monitor_combos[1];
    if disk.is_null() || adapter.is_null() {
        return;
    }

    app().disk_choice_ids.clear();
    SendMessageW(disk, CB_RESETCONTENT, 0, 0);
    let system_name = disk_label(0);
    let system_label = format!("Sistema ({system_name}) - automatico");
    SendMessageW(disk, CB_ADDSTRING, 0, wide(&system_label).as_ptr() as isize);
    app().disk_choice_ids.push(0);
    for (id, label) in available_drives() {
        SendMessageW(disk, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        app().disk_choice_ids.push(id);
    }
    let desired_disk = app().config.monitor_disk;
    if !app().disk_choice_ids.contains(&desired_disk) {
        let label = format!("{} (desconectado)", disk_label(desired_disk));
        SendMessageW(disk, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        app().disk_choice_ids.push(desired_disk);
    }

    app().adapter_choice_ids.clear();
    SendMessageW(adapter, CB_RESETCONTENT, 0, 0);
    SendMessageW(
        adapter,
        CB_ADDSTRING,
        0,
        wide("Todas as interfaces ativas").as_ptr() as isize,
    );
    app().adapter_choice_ids.push(0);
    for (id, label) in available_adapters() {
        SendMessageW(adapter, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        app().adapter_choice_ids.push(id);
    }
    let desired_adapter = app().config.monitor_adapter;
    if !app().adapter_choice_ids.contains(&desired_adapter) {
        let label = format!("Interface #{desired_adapter} (indisponivel; selecione novamente)");
        SendMessageW(adapter, CB_ADDSTRING, 0, wide(&label).as_ptr() as isize);
        app().adapter_choice_ids.push(desired_adapter);
    }
    sync_monitor_combos();
}

unsafe fn sync_monitor_combos() {
    let disk_index = app()
        .disk_choice_ids
        .iter()
        .position(|&id| id == app().config.monitor_disk)
        .unwrap_or(0);
    let adapter_index = app()
        .adapter_choice_ids
        .iter()
        .position(|&id| id == app().config.monitor_adapter)
        .unwrap_or(0);
    if !app().monitor_combos[0].is_null() {
        SendMessageW(app().monitor_combos[0], CB_SETCURSEL, disk_index, 0);
    }
    if !app().monitor_combos[1].is_null() {
        SendMessageW(app().monitor_combos[1], CB_SETCURSEL, adapter_index, 0);
    }
    if !app().monitor_combos[2].is_null() {
        SendMessageW(
            app().monitor_combos[2],
            CB_SETCURSEL,
            app().config.update_interval as usize,
            0,
        );
    }
}

unsafe fn read_monitor_choices(id: u32) {
    match id {
        CMD_MONITOR_DISK => {
            let index = selected_combo(app().monitor_combos[0]) as usize;
            if let Some(&choice) = app().disk_choice_ids.get(index) {
                app().config.monitor_disk = choice;
                app().selected_disk.store(choice, Ordering::Release);
            }
        }
        CMD_MONITOR_ADAPTER => {
            let index = selected_combo(app().monitor_combos[1]) as usize;
            if let Some(&choice) = app().adapter_choice_ids.get(index) {
                app().config.monitor_adapter = choice;
                app().selected_adapter.store(choice, Ordering::Release);
            }
        }
        CMD_MONITOR_INTERVAL => {
            let index = selected_combo(app().monitor_combos[2]).min(3);
            app().config.update_interval = index;
            app()
                .refresh_interval
                .store(update_interval_ms(index), Ordering::Release);
            if !app().editor.is_null() && !app().editor_combos[9].is_null() {
                SendMessageW(app().editor_combos[9], CB_SETCURSEL, index as usize, 0);
            }
        }
        _ => return,
    }
    save_later();
}

unsafe fn selected_combo(hwnd: HWND) -> u32 {
    if hwnd.is_null() {
        return 0;
    }
    let value = SendMessageW(hwnd, CB_GETCURSEL, 0, 0);
    if value < 0 {
        0
    } else {
        value as u32
    }
}

unsafe fn update_appearance() {
    for index in 0..N {
        let hwnd = app().cards[index];
        if hwnd.is_null() {
            continue;
        }
        let monitors = app().monitor_areas.clone();
        let rect = screen_rect(hwnd).unwrap_or_else(|| {
            let (x, y) = app().config.positions[index];
            let (w, h) = dimensions(app().config.card_sizes[index]);
            Rect::new(x, y, w, h)
        });
        let (width, height) = physical_size(index, &monitors, rect);
        SetWindowPos(
            hwnd,
            null_mut(),
            0,
            0,
            width,
            height,
            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
        );
        set_card_region(index);
        let visual = visual_for(&app().config, index, app().system_light);
        SetLayeredWindowAttributes(hwnd, 0, visual.alpha, LWA_ALPHA);
        InvalidateRect(hwnd, null(), 0);
    }
    if !app().settings.is_null() {
        InvalidateRect(app().settings, null(), 0);
        if !app().settings_viewport.is_null() {
            InvalidateRect(app().settings_viewport, null(), 0);
        }
    }
    if !app().editor.is_null() {
        InvalidateRect(app().editor, null(), 0);
    }
}

unsafe fn checked(hwnd: HWND) -> bool {
    !hwnd.is_null() && SendMessageW(hwnd, BM_GETCHECK, 0, 0) == BST_CHECKED as isize
}

unsafe fn read_appearance_controls() {
    let choices: [u32; 5] = std::array::from_fn(|i| selected_combo(app().appearance_combos[i]));
    let old_style = app().config.style;
    let old_size = app().config.size;
    let old_sizes = app().config.card_sizes;
    let was_default_layout = app().config.positions == positions_for_sizes(&old_sizes)
        || app().config.positions == positions_for(old_size)
        || app().config.positions == legacy_positions_for(old_size);

    app().config.style = choices[0].min(2);
    // Ao mudar de estilo, selecionamos seu modo recomendado; o usuário pode modificá-lo em seguida.
    app().config.theme = if old_style != app().config.style {
        if app().config.style == 0 {
            1
        } else {
            0
        }
    } else {
        choices[1].min(2)
    };
    app().config.accent = choices[2].min((ACCENTS.len() - 1) as u32);
    app().config.size = choices[3].min(3);
    app().config.opacity = choices[4].min(2);
    app().config.show_graphs = checked(app().graph_checkbox);
    app().config.show_bars = checked(app().bar_checkbox);
    app().config.show_subtitles = checked(app().subtitle_checkbox);

    if app().config.size != old_size {
        app().config.card_sizes = [app().config.size; N];
    } else {
        for i in 0..N {
            if !app().card_size_combos[i].is_null() {
                app().config.card_sizes[i] = selected_combo(app().card_size_combos[i]).min(3);
            }
        }
    }
    if old_sizes != app().config.card_sizes && was_default_layout {
        app().config.positions = positions_for_sizes(&app().config.card_sizes);
        for i in 0..N {
            let (x, y) = app().config.positions[i];
            if !app().cards[i].is_null() {
                SetWindowPos(
                    app().cards[i],
                    null_mut(),
                    x,
                    y,
                    0,
                    0,
                    SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
        }
    }
    update_settings();
    update_appearance();
    save_later();
}

// Verifica apenas os botoes desenhados manualmente; evita redesenhos continuos.
fn owner_button_id(id: u32) -> bool {
    (CMD_TAB_BASE..CMD_TAB_BASE + 5).contains(&id)
        || id == CMD_SETTINGS_CLOSE
        || id == CMD_OPEN_EDITOR
        || id == CMD_MONITOR_REFRESH
        || id == CMD_APPLY_LAYOUT
        || id == CMD_ARRANGE_GRID
        || id == CMD_ARRANGE_ROW
        || id == CMD_ARRANGE_COLUMN
        || id == CMD_VISIBILITY
        || id == CMD_LOCK
        || id == CMD_RECOVER
        || (CMD_LAYOUT_ANCHOR_TILE_BASE..CMD_LAYOUT_ANCHOR_TILE_BASE + 9).contains(&id)
}

unsafe fn refresh_settings_hover(hwnd: HWND) {
    let mut cursor: POINT = std::mem::zeroed();
    let mut hovered: HWND = null_mut();
    if GetCursorPos(&mut cursor) != 0 {
        let candidate = WindowFromPoint(cursor);
        if !candidate.is_null()
            && (GetParent(candidate) == hwnd || GetParent(candidate) == app().settings_viewport)
        {
            let id = GetDlgCtrlID(candidate);
            if id >= 0 && owner_button_id(id as u32) {
                hovered = candidate;
            }
        }
    }
    let previous = app().settings_hover;
    if previous != hovered {
        app().settings_hover = hovered;
        if !previous.is_null() && IsWindow(previous) != 0 {
            InvalidateRect(previous, null(), 0);
        }
        if !hovered.is_null() {
            InvalidateRect(hovered, null(), 0);
        }
    }
}

// Paleta da INTERFACE (Configurações e Editor). Não altera os temas dos widgets.
unsafe fn ui_dark() -> bool {
    app().config.interface_theme == 1
}
unsafe fn ui_bg() -> u32 {
    if ui_dark() {
        color(18, 21, 30)
    } else {
        color(247, 249, 253)
    }
}
unsafe fn ui_sidebar() -> u32 {
    if ui_dark() {
        color(25, 29, 41)
    } else {
        color(241, 244, 251)
    }
}
unsafe fn ui_panel() -> u32 {
    if ui_dark() {
        color(33, 38, 52)
    } else {
        color(255, 255, 255)
    }
}
unsafe fn ui_panel_alt() -> u32 {
    if ui_dark() {
        color(42, 39, 66)
    } else {
        color(235, 231, 250)
    }
}
unsafe fn ui_border() -> u32 {
    if ui_dark() {
        color(63, 70, 91)
    } else {
        color(217, 224, 237)
    }
}
unsafe fn ui_ink() -> u32 {
    if ui_dark() {
        color(237, 240, 251)
    } else {
        color(31, 41, 62)
    }
}
unsafe fn ui_muted() -> u32 {
    if ui_dark() {
        color(174, 183, 207)
    } else {
        color(106, 117, 138)
    }
}
unsafe fn ui_accent() -> u32 {
    if ui_dark() {
        color(167, 130, 255)
    } else {
        color(105, 58, 201)
    }
}
unsafe fn refresh_interface_theme() {
    // Recriar pincéis garante fundo correto mesmo após navegação/scroll.
    if !app().settings_main_brush.is_null() {
        DeleteObject(app().settings_main_brush);
    }
    if !app().settings_card_brush.is_null() {
        DeleteObject(app().settings_card_brush);
    }
    app().settings_main_brush = CreateSolidBrush(ui_bg());
    app().settings_card_brush = CreateSolidBrush(ui_panel());
    if !app().editor_background_brush.is_null() {
        DeleteObject(app().editor_background_brush);
        app().editor_background_brush = CreateSolidBrush(ui_bg());
    }
    if !app().settings.is_null() {
        InvalidateRect(app().settings, null(), 0);
        if !app().settings_viewport.is_null() {
            InvalidateRect(app().settings_viewport, null(), 0);
        }
        for &(child, _) in &app().settings_control_rects {
            if !child.is_null() {
                InvalidateRect(child, null(), 0);
            }
        }
        for &tab in &app().settings_tabs {
            if !tab.is_null() {
                InvalidateRect(tab, null(), 0);
            }
        }
        let close = GetDlgItem(app().settings, CMD_SETTINGS_CLOSE as i32);
        if !close.is_null() {
            InvalidateRect(close, null(), 0);
        }
    }
    if !app().editor.is_null() {
        InvalidateRect(app().editor, null(), 0);
        for combo in app().editor_combos {
            if !combo.is_null() {
                InvalidateRect(combo, null(), 0);
            }
        }
    }
}

// Win11 Fluent: RGB padrão claro, violeta utilizado apenas em ações e destaque.
unsafe fn settings_draw_button(info: &DRAWITEMSTRUCT) {
    let id = info.CtlID;
    let nav = id >= CMD_TAB_BASE && id < CMD_TAB_BASE + 5;
    let anchor_tile = (CMD_LAYOUT_ANCHOR_TILE_BASE..CMD_LAYOUT_ANCHOR_TILE_BASE + 9).contains(&id);
    let mode_index = match id {
        CMD_ARRANGE_GRID => Some(0u32),
        CMD_ARRANGE_ROW => Some(1u32),
        CMD_ARRANGE_COLUMN => Some(2u32),
        _ => None,
    };
    let mode_selected = mode_index == Some(app().config.layout_mode);
    let active = (nav && id - CMD_TAB_BASE == app().settings_page as u32)
        || (anchor_tile && id - CMD_LAYOUT_ANCHOR_TILE_BASE == app().config.layout_anchor)
        || mode_selected;
    let primary = id == CMD_OPEN_EDITOR || id == CMD_APPLY_LAYOUT || id == CMD_SETTINGS_CLOSE;
    let hovered = info.hwndItem == app().settings_hover;
    let pressed = (info.itemState & ODS_SELECTED as u32) != 0;
    let disabled = (info.itemState & 0x0004) != 0;
    let fill = if ui_dark() {
        if disabled {
            color(48, 52, 65)
        } else if primary && pressed {
            color(95, 49, 184)
        } else if primary && hovered {
            color(143, 100, 233)
        } else if primary {
            color(111, 63, 210)
        } else if active {
            color(65, 49, 104)
        } else if hovered {
            color(47, 53, 72)
        } else if nav {
            ui_sidebar()
        } else {
            ui_panel()
        }
    } else {
        if disabled {
            color(235, 237, 243)
        } else if primary && pressed {
            color(77, 37, 174)
        } else if primary && hovered {
            color(119, 73, 214)
        } else if primary {
            color(105, 58, 201)
        } else if active {
            color(229, 221, 252)
        } else if hovered {
            color(230, 234, 245)
        } else if nav {
            color(242, 245, 252)
        } else {
            color(248, 250, 254)
        }
    };
    let border = if active {
        if ui_dark() {
            color(124, 88, 203)
        } else {
            color(207, 190, 248)
        }
    } else if primary {
        fill
    } else if nav && !hovered {
        fill
    } else {
        ui_border()
    };
    let ink = if primary {
        color(255, 255, 255)
    } else if disabled {
        ui_muted()
    } else {
        ui_ink()
    };

    let rc = info.rcItem;
    // Pinta os cantos ANTES do retângulo arredondado: impede halo cinza
    // deixado pelo tema padrão do controle Win32 após foco/hover.
    let backdrop = if nav {
        ui_sidebar()
    } else if id == CMD_SETTINGS_CLOSE {
        ui_bg()
    } else if anchor_tile {
        ui_panel_alt()
    } else {
        ui_panel()
    };
    settings_fill(info.hDC, rc, backdrop);
    let brush = CreateSolidBrush(fill);
    let pen = CreatePen(PS_SOLID, 1, border);
    let old_brush = SelectObject(info.hDC, brush);
    let old_pen = SelectObject(info.hDC, pen);
    RoundRect(info.hDC, rc.left, rc.top, rc.right, rc.bottom, 15, 15);
    SelectObject(info.hDC, old_brush);
    SelectObject(info.hDC, old_pen);
    DeleteObject(brush);
    DeleteObject(pen);

    if nav {
        // SLOT VAZIO de 24 × 24 à esquerda. Futuramente receberá PNG fornecido pelo usuário.
        let mut buf = [0u16; 160];
        GetWindowTextW(info.hwndItem, buf.as_mut_ptr(), buf.len() as i32);
        let len = buf.iter().position(|v| *v == 0).unwrap_or(buf.len());
        let caption = String::from_utf16_lossy(&buf[..len]);
        draw_settings_fit_text(
            info.hDC,
            &caption,
            RECT {
                left: rc.left + 50,
                top: rc.top,
                right: rc.right - 9,
                bottom: rc.bottom,
            },
            16,
            false,
            ink,
            false,
        );
        if active {
            draw_rounded_fill(
                info.hDC,
                RECT {
                    left: rc.left + 2,
                    top: rc.top + 10,
                    right: rc.left + 6,
                    bottom: rc.bottom - 10,
                },
                4,
                ui_accent(),
            );
        }
    } else if anchor_tile {
        // Mini quadrado central: revela visualmente a posição selecionada.
        let mid_x = (rc.left + rc.right) / 2;
        let mid_y = (rc.top + rc.bottom) / 2;
        draw_rounded_fill(
            info.hDC,
            RECT {
                left: mid_x - 7,
                top: mid_y - 7,
                right: mid_x + 7,
                bottom: mid_y + 7,
            },
            6,
            if active { ui_accent() } else { ui_muted() },
        );
    } else if let Some(which) = mode_index {
        let middle_x = (rc.left + rc.right) / 2;
        let tone = if mode_selected {
            ui_accent()
        } else {
            ui_muted()
        };
        match which {
            0 => {
                for row in 0..2 {
                    for col in 0..3 {
                        let x = middle_x - 25 + col * 19;
                        let y = rc.top + 14 + row * 17;
                        draw_rounded_fill(
                            info.hDC,
                            RECT {
                                left: x,
                                top: y,
                                right: x + 14,
                                bottom: y + 12,
                            },
                            4,
                            tone,
                        );
                    }
                }
            }
            1 => {
                for row in 0..3 {
                    let y = rc.top + 13 + row * 12;
                    draw_rounded_fill(
                        info.hDC,
                        RECT {
                            left: middle_x - 25,
                            top: y,
                            right: middle_x + 25,
                            bottom: y + 6,
                        },
                        4,
                        tone,
                    );
                }
            }
            _ => {
                for col in 0..3 {
                    let x = middle_x - 20 + col * 16;
                    draw_rounded_fill(
                        info.hDC,
                        RECT {
                            left: x,
                            top: rc.top + 12,
                            right: x + 10,
                            bottom: rc.top + 41,
                        },
                        4,
                        tone,
                    );
                }
            }
        }
        let label = match which {
            0 => "Grade",
            1 => "Linhas",
            _ => "Colunas",
        };
        draw_settings_fit_text(
            info.hDC,
            label,
            RECT {
                left: rc.left + 5,
                top: rc.bottom - 28,
                right: rc.right - 5,
                bottom: rc.bottom - 4,
            },
            16,
            false,
            ink,
            true,
        );
    } else {
        let mut buf = [0u16; 160];
        GetWindowTextW(info.hwndItem, buf.as_mut_ptr(), buf.len() as i32);
        let len = buf.iter().position(|v| *v == 0).unwrap_or(buf.len());
        let caption = String::from_utf16_lossy(&buf[..len]);
        let mut target = rc;
        InflateRect(&mut target, -7, -2);
        draw_settings_fit_text(
            info.hDC,
            &caption,
            target,
            if primary { 18 } else { 16 },
            primary,
            ink,
            true,
        );
    }
    if (info.itemState & ODS_FOCUS as u32) != 0 {
        // Contorno de foco Fluent: sem cinza/pontilhado legado do Windows.
        let pen = CreatePen(PS_SOLID, 1, ui_accent());
        let prev_pen = SelectObject(info.hDC, pen);
        let prev_brush = SelectObject(info.hDC, GetStockObject(NULL_BRUSH));
        RoundRect(
            info.hDC,
            rc.left + 2,
            rc.top + 2,
            rc.right - 2,
            rc.bottom - 2,
            12,
            12,
        );
        SelectObject(info.hDC, prev_brush);
        SelectObject(info.hDC, prev_pen);
        DeleteObject(pen);
    }
}

unsafe fn settings_panel(dc: HDC, rect: RECT, radius: i32) {
    draw_rounded_fill(dc, rect, radius, ui_panel());
    let pen = CreatePen(PS_SOLID, 1, ui_border());
    let prev_pen = SelectObject(dc, pen);
    let prev_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    RoundRect(
        dc,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        radius,
        radius,
    );
    SelectObject(dc, prev_brush);
    SelectObject(dc, prev_pen);
    DeleteObject(pen);
}

unsafe fn settings_fill(dc: HDC, rect: RECT, rgb: u32) {
    let brush = CreateSolidBrush(rgb);
    FillRect(dc, &rect, brush);
    DeleteObject(brush);
}

unsafe fn settings_header(dc: HDC, bounds: RECT) {
    settings_fill(
        dc,
        RECT {
            left: 0,
            top: 0,
            right: 245,
            bottom: bounds.bottom,
        },
        ui_sidebar(),
    );
    settings_fill(
        dc,
        RECT {
            left: 245,
            top: 0,
            right: bounds.right,
            bottom: bounds.bottom,
        },
        ui_bg(),
    );
    settings_fill(
        dc,
        RECT {
            left: 244,
            top: 0,
            right: 245,
            bottom: bounds.bottom,
        },
        ui_border(),
    );

    // Cabeçalho simplificado: sem ícone e sem a marca antiga.
    // Mantém apenas o nome novo do app e a versão, como solicitado.
    draw_card_text(
        dc,
        "Rtime",
        RECT {
            left: 26,
            top: 28,
            right: 220,
            bottom: 58,
        },
        app().rate_font,
        ui_ink(),
        false,
    );
    draw_card_text(
        dc,
        "v0.3.20",
        RECT {
            left: 26,
            top: 58,
            right: 220,
            bottom: 80,
        },
        app().label_font,
        ui_muted(),
        false,
    );

    // Não sobrepor o último item de navegação em resoluções baixas.
    if bounds.bottom >= 630 {
        let divider_y = bounds.bottom - 89;
        settings_fill(
            dc,
            RECT {
                left: 22,
                top: divider_y,
                right: 224,
                bottom: divider_y + 1,
            },
            ui_border(),
        );
        draw_card_text(
            dc,
            "GITHUB",
            RECT {
                left: 25,
                top: divider_y + 12,
                right: 215,
                bottom: divider_y + 32,
            },
            app().compact_font,
            ui_muted(),
            false,
        );
        draw_card_text(
            dc,
            "@joaorizzo0112",
            RECT {
                left: 25,
                top: divider_y + 36,
                right: 225,
                bottom: divider_y + 64,
            },
            app().label_font,
            ui_accent(),
            false,
        );
    }
}

unsafe fn paint_settings_window(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let screen_dc = BeginPaint(hwnd, &mut ps);
    if screen_dc.is_null() {
        return;
    }
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    let buffer_dc = CreateCompatibleDC(screen_dc);
    let bitmap = if buffer_dc.is_null() {
        null_mut()
    } else {
        CreateCompatibleBitmap(screen_dc, bounds.right.max(1), bounds.bottom.max(1))
    };
    let previous_bitmap = if bitmap.is_null() {
        null_mut()
    } else {
        SelectObject(buffer_dc, bitmap)
    };
    let dc = if bitmap.is_null() {
        screen_dc
    } else {
        buffer_dc
    };
    settings_fill(dc, bounds, ui_bg());
    settings_header(dc, bounds);

    let footer_y = bounds.bottom - 66;
    settings_fill(
        dc,
        RECT {
            left: 245,
            top: footer_y,
            right: bounds.right,
            bottom: bounds.bottom,
        },
        ui_bg(),
    );
    settings_fill(
        dc,
        RECT {
            left: 245,
            top: footer_y,
            right: bounds.right,
            bottom: footer_y + 1,
        },
        ui_border(),
    );
    draw_rounded_fill(
        dc,
        RECT {
            left: 278,
            top: footer_y + 25,
            right: 288,
            bottom: footer_y + 35,
        },
        10,
        color(52, 169, 110),
    );
    if bounds.right >= 680 {
        draw_card_text(
            dc,
            "As alterações são aplicadas automaticamente.",
            RECT {
                left: 298,
                top: footer_y + 18,
                right: bounds.right - 140,
                bottom: footer_y + 46,
            },
            app().label_font,
            ui_muted(),
            false,
        );
    }

    // Os cartões pertencem ao viewport rolável, não à janela principal.
    if !bitmap.is_null() {
        BitBlt(
            screen_dc,
            0,
            0,
            bounds.right,
            bounds.bottom,
            dc,
            0,
            0,
            SRCCOPY,
        );
        SelectObject(buffer_dc, previous_bitmap);
        DeleteObject(bitmap);
    }
    if !buffer_dc.is_null() {
        DeleteDC(buffer_dc);
    }
    EndPaint(hwnd, &ps);
}

// Editor independente; cada troca e refletida imediatamente no card real.
unsafe fn update_editor_controls() {
    if app().editor.is_null() {
        return;
    }
    let i = app().editor_index.min(N - 1);
    let cfg = &app().config;
    let selections = [
        i as u32,
        cfg.widget_styles[i],
        cfg.widget_accents[i],
        cfg.widget_graphs[i],
        cfg.widget_fonts[i],
        cfg.widget_borders[i],
        cfg.widget_opacities[i],
        cfg.widget_bars[i],
        cfg.widget_subtitles[i],
        cfg.update_interval,
        0, // Um perfil e uma acao, nao um estado que substitua escolhas individuais.
    ];
    for (index, value) in selections.iter().enumerate() {
        let hwnd = app().editor_combos[index];
        if !hwnd.is_null() {
            SendMessageW(hwnd, CB_SETCURSEL, *value as usize, 0);
        }
    }
    update_editor_action_labels();
    InvalidateRect(app().editor, null(), 0);
}

unsafe fn update_editor_action_labels() {
    if app().editor.is_null() {
        return;
    }
    let toggle = GetDlgItem(app().editor, CMD_EDIT_GROUP_TOGGLE as i32);
    if !toggle.is_null() {
        let caption = if app().config.group_move {
            "Arraste em grupo: ativado"
        } else {
            "Arraste em grupo: desativado"
        };
        SetWindowTextW(toggle, wide(caption).as_ptr());
    }
}

// Editor Fluent responsivo preservado. Rótulos são pintados pelo GDI, com
// retângulos idênticos aos usados pelos ComboBoxes nativos. Isso evita cortes,
// sobreposição de STATIC e a diferença de escala entre controles e desenho.
fn editor_geometry(client_width: i32) -> EditorLayout {
    EditorLayout::new(client_width)
}

unsafe fn editor_reflow(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);
    let layout = editor_geometry(client.right.max(1));
    let available_height = client.bottom.max(1);
    let maximum = (layout.content_height - available_height).max(0);
    app().editor_scroll_y = app().editor_scroll_y.clamp(0, maximum);
    app().editor_content_h = layout.content_height;
    let scroll = SCROLLINFO {
        cbSize: std::mem::size_of::<SCROLLINFO>() as u32,
        fMask: SIF_RANGE | SIF_PAGE | SIF_POS,
        nMin: 0,
        nMax: layout.content_height.saturating_sub(1),
        nPage: available_height as u32,
        nPos: app().editor_scroll_y,
        nTrackPos: 0,
    };
    SetScrollInfo(hwnd, SB_VERT, &scroll, 1);
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let font_px = (15 * dpi / 96).clamp(14, 18);
    if app().editor_control_font.is_null() || app().editor_font_px != font_px {
        // Recalcular a fonte quando a janela muda de DPI. Primeiro trocar a
        // referência de todos os HWNDs; depois liberar a fonte anterior.
        let new_font = CreateFontW(
            -font_px,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        if !new_font.is_null() {
            let previous = app().editor_control_font;
            app().editor_control_font = new_font;
            app().editor_font_px = font_px;
            for &combo in app().editor_combos.iter() {
                if !combo.is_null() {
                    SendMessageW(combo, WM_SETFONT, new_font as usize, 0);
                }
            }
            for id in [
                CMD_EDIT_PREVIOUS,
                CMD_EDIT_NEXT,
                CMD_EDIT_RESET,
                CMD_EDIT_CLOSE,
                CMD_EDIT_COPY_ALL,
                CMD_EDIT_GROUP_TOGGLE,
                CMD_EDIT_ARRANGE_NOW,
            ] {
                let button = GetDlgItem(hwnd, id as i32);
                if !button.is_null() {
                    SendMessageW(button, WM_SETFONT, new_font as usize, 0);
                }
            }
            if !previous.is_null() {
                DeleteObject(previous);
            }
        }
    }
    let shift = app().editor_scroll_y;
    // A rolagem move 11 ComboBoxes e 7 botões. Se cada controle for
    // reposicionado/redesenhado individualmente, o GDI pode copiar pixels da
    // posição anterior (SWP usa copy-bits por padrão) e deixar rastros dos
    // botões owner-draw sobre a prévia. Preparar todos os movimentos primeiro
    // e aplicar de uma só vez evita estados intermediários visíveis.
    let mut moves: Vec<(HWND, EditorRect)> = Vec::with_capacity(18);
    for (index, rect) in layout.combos.iter().enumerate() {
        let control = app().editor_combos[index];
        if !control.is_null() {
            // A altura total do ComboBox inclui a lista suspensa (230 px).
            moves.push((
                control,
                EditorRect::new(rect.x, rect.y - shift, rect.w, 240),
            ));
        }
    }
    for (id, rect) in [
        (CMD_EDIT_PREVIOUS, layout.previous),
        (CMD_EDIT_NEXT, layout.next),
        (CMD_EDIT_RESET, layout.reset),
        (CMD_EDIT_CLOSE, layout.close),
        (CMD_EDIT_COPY_ALL, layout.copy_all),
        (CMD_EDIT_GROUP_TOGGLE, layout.group_toggle),
        (CMD_EDIT_ARRANGE_NOW, layout.arrange),
    ] {
        let button = GetDlgItem(hwnd, id as i32);
        if !button.is_null() {
            moves.push((button, rect.translated(-shift)));
        }
    }

    // SWP_NOCOPYBITS: nenhum pixel da posição velha é reutilizado.
    // SWP_NOREDRAW: o desenho acontece só depois que TODOS os HWNDs
    // estiverem em suas posições definitivas, em vez de redesenhos parciais.
    let flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS | SWP_NOREDRAW;
    let mut batch = BeginDeferWindowPos(moves.len() as i32);
    let mut applied = false;
    if !batch.is_null() {
        let mut ok = true;
        for &(control, rect) in &moves {
            let next = DeferWindowPos(
                batch,
                control,
                null_mut(),
                rect.x,
                rect.y,
                rect.w,
                rect.h,
                flags,
            );
            if next.is_null() {
                ok = false;
                break;
            }
            batch = next;
        }
        if ok {
            applied = EndDeferWindowPos(batch) != 0;
        }
    }
    // Fallback de compatibilidade se o sistema não puder reservar o lote.
    if !applied {
        for &(control, rect) in &moves {
            SetWindowPos(control, null_mut(), rect.x, rect.y, rect.w, rect.h, flags);
        }
    }

    // O editor usa desenho GDI com buffer duplo e botões Win32 owner-draw.
    // Invalidar pai E filhos após reposicionar (e atualizar imediatamente)
    // é indispensável para limpar áreas antigas, sobretudo após vários
    // eventos consecutivos da roda do mouse.
    RedrawWindow(
        hwnd,
        null(),
        null_mut(),
        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
    );
}

unsafe fn editor_scroll_to(hwnd: HWND, next: i32) {
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);
    let maximum = (app().editor_content_h - client.bottom.max(1)).max(0);
    let position = next.clamp(0, maximum);
    if position != app().editor_scroll_y {
        app().editor_scroll_y = position;
        editor_reflow(hwnd);
    }
}

unsafe fn editor_scroll_event(hwnd: HWND, w: WPARAM) {
    let command = (w & 0xffff) as u32;
    let pos = app().editor_scroll_y;
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);
    let page = client.bottom.max(150) - 90;
    let target = match command {
        x if x == SB_LINEUP as u32 => pos - 42,
        x if x == SB_LINEDOWN as u32 => pos + 42,
        x if x == SB_PAGEUP as u32 => pos - page,
        x if x == SB_PAGEDOWN as u32 => pos + page,
        x if x == SB_TOP as u32 => 0,
        x if x == SB_BOTTOM as u32 => app().editor_content_h,
        x if x == SB_THUMBPOSITION as u32 || x == SB_THUMBTRACK as u32 => {
            ((w >> 16) & 0xffff) as i32
        }
        _ => pos,
    };
    editor_scroll_to(hwnd, target);
}

// A navegação por Tab deve levar automaticamente o campo para a área visível.
unsafe fn editor_ensure_focused_visible(hwnd: HWND, control: HWND) {
    if hwnd.is_null() || control.is_null() {
        return;
    }
    let mut window: RECT = std::mem::zeroed();
    let mut client: RECT = std::mem::zeroed();
    if GetWindowRect(control, &mut window) == 0 {
        return;
    }
    GetClientRect(hwnd, &mut client);
    let mut origin = POINT {
        x: window.left,
        y: window.top,
    };
    ScreenToClient(hwnd, &mut origin);
    let current = app().editor_scroll_y;
    let height = 38;
    if origin.y < 12 {
        editor_scroll_to(hwnd, current + origin.y - 12);
    } else if origin.y + height > client.bottom - 12 {
        editor_scroll_to(hwnd, current + origin.y + height - client.bottom + 12);
    }
}

unsafe fn create_editor_controls(hwnd: HWND) {
    app().editor_scroll_y = 0;
    app().editor_combos[0] = make_combo(
        hwnd,
        CMD_EDIT_CARD,
        0,
        0,
        180,
        &NAMES,
        app().editor_index as u32,
        true,
    );
    for (index, id, labels) in [
        (1, CMD_EDIT_STYLE, &INDIVIDUAL_STYLES[..]),
        (2, CMD_EDIT_ACCENT, &INDIVIDUAL_ACCENTS[..]),
        (3, CMD_EDIT_GRAPH, &GRAPH_MODES[..]),
        (4, CMD_EDIT_FONT, &FONT_SCALES[..]),
        (5, CMD_EDIT_BORDER, &BORDER_MODES[..]),
        (6, CMD_EDIT_OPACITY, &INDIVIDUAL_OPACITIES[..]),
        (7, CMD_EDIT_BARS, &VISIBILITY_CHOICES[..]),
        (8, CMD_EDIT_SUBTITLES, &VISIBILITY_CHOICES[..]),
        (9, CMD_EDIT_INTERVAL, &UPDATE_INTERVALS[..]),
        (10, CMD_EDIT_PROFILE, &PROFILES[..]),
    ] {
        app().editor_combos[index] = make_combo(hwnd, id, 0, 0, 160, labels, 0, true);
    }
    for (id, label) in [
        (CMD_EDIT_PREVIOUS, "← Anterior"),
        (CMD_EDIT_NEXT, "Próximo →"),
        (CMD_EDIT_RESET, "Restaurar este widget"),
        (CMD_EDIT_CLOSE, "Fechar"),
        (CMD_EDIT_COPY_ALL, "Copiar aparência para todos"),
        (CMD_EDIT_GROUP_TOGGLE, "Arraste em grupo: desativado"),
        (CMD_EDIT_ARRANGE_NOW, "Organizar conjunto agora"),
    ] {
        make_control(
            "BUTTON",
            label,
            BS_OWNERDRAW as u32,
            0,
            0,
            100,
            36,
            hwnd,
            id,
        );
    }
    editor_reflow(hwnd);
}

unsafe fn show_editor(index: usize) {
    app().editor_index = index.min(N - 1);
    if !app().editor.is_null() {
        update_editor_controls();
        reveal_auxiliary_window(app().editor, EDITOR_DEFAULT_WIDTH, EDITOR_DEFAULT_HEIGHT);
        return;
    }
    let mut work: RECT = std::mem::zeroed();
    if SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut work as *mut RECT as *mut c_void, 0) == 0 {
        work.right = GetSystemMetrics(SM_CXSCREEN);
        work.bottom = GetSystemMetrics(SM_CYSCREEN);
    }
    let available_w = (work.right - work.left).max(1);
    let available_h = (work.bottom - work.top).max(1);
    // Tamanho fixo por sessão, sem resize / maximizar. Em monitores menores,
    // usar a área útil para não deixar os botões de fechar fora da tela.
    let width = EDITOR_DEFAULT_WIDTH.min(available_w);
    let height = EDITOR_DEFAULT_HEIGHT.min(available_h);
    let x = work.left + (available_w - width) / 2;
    let y = work.top + (available_h - height) / 2;
    let editor_style = FIXED_DIALOG_STYLE | WS_VSCROLL;
    let hwnd = window(
        WS_EX_APPWINDOW,
        EDITOR_CLASS,
        "Rtime - Editor de widgets",
        editor_style,
        x,
        y,
        width,
        height,
        null_mut(),
        0,
        null(),
    );
    if !hwnd.is_null() {
        app().editor = hwnd;
        update_editor_controls();
        reveal_auxiliary_window(hwnd, EDITOR_DEFAULT_WIDTH, EDITOR_DEFAULT_HEIGHT);
    }
}

// Presets alteram apenas a aparência global. Preferências individuais permanecem intactas.
fn apply_visual_profile(cfg: &mut Config, selection: u32) {
    match selection {
        1 => {
            // Minimalista
            cfg.style = 1;
            cfg.theme = 0;
            cfg.accent = 0;
            cfg.show_graphs = false;
            cfg.show_bars = false;
            cfg.show_subtitles = false;
        }
        2 => {
            // Trabalho
            cfg.style = 0;
            cfg.theme = 1;
            cfg.accent = 1;
            cfg.show_graphs = false;
            cfg.show_bars = true;
            cfg.show_subtitles = true;
        }
        3 => {
            // Monitoramento
            cfg.style = 2;
            cfg.theme = 0;
            cfg.accent = 2;
            cfg.show_graphs = true;
            cfg.show_bars = true;
            cfg.show_subtitles = true;
        }
        _ => {}
    }
}

unsafe fn apply_editor_controls(id: u32) {
    if id == CMD_EDIT_PROFILE {
        let profile = selected_combo(app().editor_combos[10]);
        apply_visual_profile(&mut app().config, profile);
        update_settings();
        update_appearance();
        save_later();
        return;
    }
    if id == CMD_EDIT_CARD {
        app().editor_index = selected_combo(app().editor_combos[0]).min(N as u32 - 1) as usize;
        update_editor_controls();
        return;
    }
    if app().editor.is_null() {
        return;
    }
    let i = app().editor_index;
    let values: [u32; 10] = std::array::from_fn(|slot| selected_combo(app().editor_combos[slot]));
    app().config.widget_styles[i] = values[1].min(3);
    app().config.widget_accents[i] = values[2].min(6);
    app().config.widget_graphs[i] = values[3].min(4);
    app().config.widget_fonts[i] = values[4].min(2);
    app().config.widget_borders[i] = values[5].min(2);
    app().config.widget_opacities[i] = values[6].min(3);
    app().config.widget_bars[i] = values[7].min(2);
    app().config.widget_subtitles[i] = values[8].min(2);
    app().config.update_interval = values[9].min(3);
    let interval_ms = update_interval_ms(app().config.update_interval);
    app().refresh_interval.store(interval_ms, Ordering::Release);
    sync_monitor_combos();
    update_appearance();
    save_later();
}

// Os botões do editor seguem as mesmas cores Fluent das Configurações,
// mas preservam todos os IDs e ações existentes.
unsafe fn draw_editor_button(info: &DRAWITEMSTRUCT) {
    let primary = info.CtlID == CMD_EDIT_COPY_ALL;
    let pressed = (info.itemState & ODS_SELECTED as u32) != 0;
    let focused = (info.itemState & ODS_FOCUS as u32) != 0;
    let fill = if primary {
        if pressed {
            color(81, 39, 167)
        } else {
            color(105, 58, 201)
        }
    } else if pressed {
        ui_panel_alt()
    } else {
        ui_panel()
    };
    let border = if primary {
        color(105, 58, 201)
    } else {
        ui_border()
    };
    let rect = info.rcItem;
    settings_fill(info.hDC, rect, ui_bg());
    let brush = CreateSolidBrush(fill);
    let pen = CreatePen(PS_SOLID, 1, border);
    let old_brush = SelectObject(info.hDC, brush);
    let old_pen = SelectObject(info.hDC, pen);
    RoundRect(
        info.hDC,
        rect.left,
        rect.top,
        rect.right,
        rect.bottom,
        13,
        13,
    );
    SelectObject(info.hDC, old_pen);
    SelectObject(info.hDC, old_brush);
    DeleteObject(pen);
    DeleteObject(brush);
    let mut buf = [0u16; 160];
    GetWindowTextW(info.hwndItem, buf.as_mut_ptr(), buf.len() as i32);
    let n = buf.iter().position(|&v| v == 0).unwrap_or(buf.len());
    let label = String::from_utf16_lossy(&buf[..n]);
    draw_settings_fit_text(
        info.hDC,
        &label,
        RECT {
            left: rect.left + 8,
            top: rect.top,
            right: rect.right - 8,
            bottom: rect.bottom,
        },
        16,
        primary,
        if primary {
            color(255, 255, 255)
        } else {
            ui_ink()
        },
        true,
    );
    if focused {
        let mut focus = rect;
        InflateRect(&mut focus, -4, -4);
        let pen = CreatePen(PS_SOLID, 1, ui_accent());
        let old_pen = SelectObject(info.hDC, pen);
        let old_brush = SelectObject(info.hDC, GetStockObject(NULL_BRUSH));
        RoundRect(
            info.hDC,
            focus.left,
            focus.top,
            focus.right,
            focus.bottom,
            9,
            9,
        );
        SelectObject(info.hDC, old_brush);
        SelectObject(info.hDC, old_pen);
        DeleteObject(pen);
    }
}

unsafe fn editor_outline(dc: HDC, area: EditorRect, bg: u32, border: u32) {
    let rect = area.to_rect();
    draw_rounded_fill(dc, rect, 20, bg);
    let pen = CreatePen(PS_SOLID, 1, border);
    let old_pen = SelectObject(dc, pen);
    let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
    RoundRect(dc, rect.left, rect.top, rect.right, rect.bottom, 20, 20);
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    DeleteObject(pen);
}

unsafe fn editor_text(dc: HDC, label: &str, rect: EditorRect, px: i32, bold: bool, ink: u32) {
    draw_settings_fit_text(dc, label, rect.to_rect(), px, bold, ink, false);
}

// A prévia selecionada é de fato o widget escolhido, não um desenho fictício.
unsafe fn paint_editor_metric(dc: HDC, area: EditorRect, index: usize, small: bool) {
    if area.w < 80 || area.h < 50 {
        return;
    }
    let visual = visual_for(&app().config, index, app().system_light);
    let pal = visual.palette;
    editor_outline(dc, area, pal.background, pal.border);
    draw_rounded_fill(
        dc,
        RECT {
            left: area.x + 1,
            top: area.y + 13,
            right: area.x + 5,
            bottom: area.y + area.h - 13,
        },
        5,
        pal.accent,
    );
    let x = area.x + 19;
    let right = area.x + area.w - 18;
    let y = area.y;
    editor_text(
        dc,
        NAMES[index],
        EditorRect::new(x, y + 8, area.w - 36, 22),
        if small { 12 } else { 14 },
        false,
        pal.subtle,
    );
    let data = app().metrics;
    let (value, subtitle, pct): (String, String, Option<f64>) = match index {
        0 => (
            format!("{:.0}%", data.cpu_pct),
            "Uso da CPU".into(),
            Some(data.cpu_pct),
        ),
        1 => (
            format!("{:.0}%", data.ram_pct),
            format!("{:.1} / {:.1} GiB", data.ram_used_gib, data.ram_total_gib),
            Some(data.ram_pct),
        ),
        2 if data.disk_available => (
            format!("{:.0}%", data.disk_pct),
            format!(
                "{} · {:.0} GiB livres",
                disk_label(app().config.monitor_disk),
                data.disk_free_gib
            ),
            Some(data.disk_pct),
        ),
        2 => ("N/D".into(), "Disco indisponível".into(), None),
        3 if data.network_available => (fmt_network(data.up_mib), "Envio".into(), None),
        3 => ("Sem rede".into(), "Interface indisponível".into(), None),
        4 => (fmt_uptime(data.uptime_ms), "Tempo ligado".into(), None),
        _ => (data.process_count.to_string(), "Em execução".into(), None),
    };
    let main_top = if small { y + 25 } else { y + 34 };
    let value_px = (if small { 23 } else { 36 })
        + match visual.font {
            1 => {
                if small {
                    2
                } else {
                    4
                }
            }
            2 => {
                if small {
                    -2
                } else {
                    -4
                }
            }
            _ => 0,
        };
    editor_text(
        dc,
        &value,
        EditorRect::new(x, main_top, area.w - 36, if small { 29 } else { 44 }),
        value_px,
        true,
        pal.foreground,
    );
    if index == 3 && data.network_available && visual.subtitles {
        if small {
            editor_text(
                dc,
                "Total utilizado",
                EditorRect::new(x, y + 55, area.w * 53 / 100, 16),
                11,
                false,
                pal.subtle,
            );
            editor_text(
                dc,
                &fmt_network_total(data.network_total_bytes),
                EditorRect::new(x, y + 71, area.w * 53 / 100, 19),
                16,
                true,
                pal.foreground,
            );
            if visual.graph != 0 && area.w > 210 {
                draw_history(
                    dc,
                    3,
                    RECT {
                        left: area.x + area.w * 55 / 100,
                        top: y + 51,
                        right: right,
                        bottom: y + area.h - 13,
                    },
                    pal,
                    visual.graph,
                );
            }
        } else {
            editor_text(
                dc,
                "Envio",
                EditorRect::new(x, y + 78, 140, 20),
                13,
                false,
                pal.subtle,
            );
            editor_text(
                dc,
                "Total de dados utilizados",
                EditorRect::new(x, y + 100, area.w - 36, 18),
                12,
                false,
                pal.subtle,
            );
            editor_text(
                dc,
                &fmt_network_total(data.network_total_bytes),
                EditorRect::new(x, y + 118, area.w / 2, 26),
                20,
                true,
                pal.foreground,
            );
            if visual.graph != 0 {
                draw_history(
                    dc,
                    3,
                    RECT {
                        left: area.x + area.w / 2,
                        top: y + 80,
                        right: right,
                        bottom: y + area.h - 16,
                    },
                    pal,
                    visual.graph,
                );
            }
        }
    } else {
        if visual.subtitles {
            editor_text(
                dc,
                &subtitle,
                EditorRect::new(
                    x,
                    y + if small { 54 } else { 82 },
                    area.w - 36,
                    if small { 15 } else { 20 },
                ),
                if small { 12 } else { 13 },
                false,
                pal.subtle,
            );
        }
        if index == 5 {
            if !small {
                editor_text(
                    dc,
                    "Mais RAM",
                    EditorRect::new(x, y + 102, area.w - 36, 14),
                    12,
                    true,
                    pal.subtle,
                );
            }
            let count = if small { 1 } else { 3 };
            for pos in 0..count {
                let process = data.top_processes[pos];
                if process.working_set_bytes == 0 {
                    break;
                }
                let label = format!(
                    "{} · {}",
                    process.display_name(),
                    fmt_process_ram(process.working_set_bytes)
                );
                editor_text(
                    dc,
                    &label,
                    EditorRect::new(
                        x,
                        y + if small { 74 } else { 117 + pos as i32 * 15 },
                        area.w - 36,
                        14,
                    ),
                    11,
                    false,
                    pal.foreground,
                );
            }
        } else if index == 4 && visual.bars {
            let ratio = (data.uptime_ms % 86_400_000) as f64 / 86_400_000.0;
            let bar = RECT {
                left: x,
                top: y + area.h - 11,
                right: right,
                bottom: y + area.h - 6,
            };
            draw_rounded_fill(dc, bar, 7, pal.track);
            let extent = ((right - x) as f64 * ratio).round() as i32;
            if extent > 1 {
                draw_rounded_fill(
                    dc,
                    RECT {
                        right: x + extent,
                        ..bar
                    },
                    7,
                    pal.accent,
                );
            }
            if !small && area.h >= 160 {
                editor_text(
                    dc,
                    "Ciclo atual de 24 h",
                    EditorRect::new(x, y + area.h - 38, area.w - 36, 19),
                    12,
                    false,
                    pal.subtle,
                );
            }
        } else if let Some(percent) = pct {
            if visual.graph != 0 && !small {
                draw_history(
                    dc,
                    index,
                    RECT {
                        left: x,
                        top: y + 107,
                        right: right,
                        bottom: y + area.h - 25,
                    },
                    pal,
                    visual.graph,
                );
            }
            if visual.bars {
                let track = RECT {
                    left: x,
                    top: y + area.h - 17,
                    right: right,
                    bottom: y + area.h - 11,
                };
                draw_rounded_fill(dc, track, 6, pal.track);
                let fill = ((right - x) as f64 * percent.clamp(0.0, 100.0) / 100.0) as i32;
                if fill > 1 {
                    draw_rounded_fill(
                        dc,
                        RECT {
                            right: x + fill,
                            ..track
                        },
                        6,
                        pal.accent,
                    );
                }
            }
        } else if visual.graph != 0 && index != 4 {
            draw_history(
                dc,
                index,
                RECT {
                    left: x,
                    top: y + area.h - 40,
                    right: right,
                    bottom: y + area.h - 15,
                },
                pal,
                visual.graph,
            );
        }
    }
}

unsafe fn paint_editor_preview(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let screen = BeginPaint(hwnd, &mut ps);
    if screen.is_null() {
        return;
    }
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    let width = bounds.right.max(1);
    let height = bounds.bottom.max(1);
    let buffer = CreateCompatibleDC(screen);
    if buffer.is_null() {
        EndPaint(hwnd, &ps);
        return;
    }
    let bitmap = CreateCompatibleBitmap(screen, width, height);
    if bitmap.is_null() {
        DeleteDC(buffer);
        EndPaint(hwnd, &ps);
        return;
    }
    let previous = SelectObject(buffer, bitmap);
    let brush = CreateSolidBrush(ui_bg());
    FillRect(buffer, &bounds, brush);
    DeleteObject(brush);

    let layout = editor_geometry(width);
    let offset = app().editor_scroll_y;
    let dark = ui_ink();
    let muted = ui_muted();
    // Os rótulos e cards usam a MESMA geometria que posiciona os ComboBoxes.
    editor_text(
        buffer,
        "Personalização individual",
        layout.heading.translated(-offset),
        27,
        true,
        dark,
    );
    editor_text(
        buffer,
        "As alterações são aplicadas imediatamente ao widget.",
        layout.intro.translated(-offset),
        15,
        false,
        muted,
    );
    editor_text(
        buffer,
        "Widget",
        layout.selector_label.translated(-offset),
        14,
        false,
        dark,
    );
    for (i, section) in layout.sections.iter().enumerate() {
        editor_outline(buffer, section.translated(-offset), ui_panel(), ui_border());
        // Sem símbolos decorativos: seção compacta, alinhada e com barra
        // de acento fina, deixando toda a largura livre para o título.
        draw_rounded_fill(
            buffer,
            RECT {
                left: section.x + 17,
                top: section.y + 17 - offset,
                right: section.x + 20,
                bottom: section.y + 37 - offset,
            },
            3,
            color(105, 58, 201),
        );
        editor_text(
            buffer,
            editor_layout::SECTION_TITLES[i],
            EditorRect::new(section.x + 30, section.y + 11 - offset, section.w - 45, 28),
            16,
            true,
            ui_ink(),
        );
        for j in 0..2 {
            let idx = 1 + 2 * i + j;
            let rect = layout.combos[idx];
            editor_text(
                buffer,
                editor_layout::FIELD_TITLES[idx],
                EditorRect::new(rect.x, rect.y - 25 - offset, rect.w, 22),
                14,
                false,
                dark,
            );
        }
    }
    editor_text(
        buffer,
        "Clique com o botão direito em qualquer widget para editar.",
        layout.footer_hint.translated(-offset),
        13,
        false,
        muted,
    );
    editor_outline(
        buffer,
        layout.preview_panel.translated(-offset),
        ui_panel_alt(),
        ui_panel_alt(),
    );
    editor_text(
        buffer,
        "PRÉ-VISUALIZAÇÃO AO VIVO",
        layout.preview_heading.translated(-offset),
        18,
        true,
        ui_accent(),
    );
    editor_text(
        buffer,
        "As alterações aparecem imediatamente no cartão selecionado.",
        layout.preview_help.translated(-offset),
        13,
        false,
        muted,
    );
    let index = app().editor_index.min(N - 1);
    paint_editor_metric(
        buffer,
        layout.main_preview.translated(-offset),
        index,
        false,
    );
    editor_text(
        buffer,
        "REDE · ENVIO E TOTAL",
        layout.network_heading.translated(-offset),
        12,
        true,
        ui_accent(),
    );
    paint_editor_metric(buffer, layout.network_preview.translated(-offset), 3, true);
    editor_text(
        buffer,
        "ATIVIDADE · TEMPO LIGADO",
        layout.activity_heading.translated(-offset),
        12,
        true,
        ui_accent(),
    );
    paint_editor_metric(buffer, layout.activity_preview.translated(-offset), 4, true);
    editor_text(
        buffer,
        "Copiar altera apenas a aparência; organizar usa o layout salvo.",
        layout.preview_note.translated(-offset),
        12,
        false,
        muted,
    );

    BitBlt(screen, 0, 0, width, height, buffer, 0, 0, SRCCOPY);
    SelectObject(buffer, previous);
    DeleteObject(bitmap);
    DeleteDC(buffer);
    EndPaint(hwnd, &ps);
}

unsafe extern "system" fn editor_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match message {
        WM_CREATE => {
            app().editor_background_brush = CreateSolidBrush(ui_bg());
            create_editor_controls(hwnd);
            0
        }
        WM_SIZE => {
            // Nao chamar ShowWindow dentro do WM_SIZE: ele pode interromper a
            // transicao de minimizacao e deixar apenas a barra de titulo.
            if w == SIZE_MINIMIZED as usize {
                PostMessageW(hwnd, EVT_HIDE_AUX, 0, 0);
            } else {
                editor_reflow(hwnd);
            }
            0
        }
        EVT_HIDE_AUX => {
            // Evita ocultar uma janela que ja foi restaurada pelo usuario.
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_HIDE);
            }
            0
        }
        WM_SYSCOMMAND if minimize_auxiliary_command(w) => {
            // Ocultar, sem deixar a barra de titulo reduzida sobre os widgets.
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_SYSCOMMAND if forbidden_resize_command(w) => 0,
        WM_NCLBUTTONDBLCLK if w == HTCAPTION as usize => 0,
        WM_SIZING => 0,
        WM_DPICHANGED => {
            if l != 0 {
                let suggested = &*(l as *const RECT);
                let mut current: RECT = std::mem::zeroed();
                if GetWindowRect(hwnd, &mut current) != 0 {
                    // Troca de DPI reposiciona, mas não expande o editor.
                    SetWindowPos(
                        hwnd,
                        null_mut(),
                        suggested.left,
                        suggested.top,
                        current.right - current.left,
                        current.bottom - current.top,
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
                editor_reflow(hwnd);
            }
            0
        }
        WM_VSCROLL if l == 0 => {
            editor_scroll_event(hwnd, w);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((w >> 16) as u16) as i16 as i32;
            if delta != 0 {
                editor_scroll_to(hwnd, app().editor_scroll_y - delta * 65 / 120);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX | WM_CTLCOLOREDIT => {
            let dc = w as HDC;
            SetTextColor(dc, ui_ink());
            SetBkMode(dc, TRANSPARENT as i32);
            app().editor_background_brush as LRESULT
        }
        WM_DRAWITEM => {
            if l != 0 {
                let item = &*(l as *const DRAWITEMSTRUCT);
                if item.CtlType == 3 {
                    settings_draw_combo(item);
                } else {
                    draw_editor_button(item);
                }
            }
            1
        }
        WM_PAINT => {
            paint_editor_preview(hwnd);
            0
        }
        WM_KEYDOWN if w == 0x7A => 0, // F11: fullscreen desativado no editor.
        WM_KEYDOWN if w == VK_ESCAPE as usize => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_COMMAND => {
            let id = (w & 0xFFFF) as u32;
            let notification = ((w >> 16) & 0xFFFF) as u32;
            if l != 0 && (notification == CBN_SETFOCUS as u32 || notification == BN_SETFOCUS as u32)
            {
                editor_ensure_focused_visible(hwnd, l as HWND);
            }
            if ((CMD_EDIT_CARD..=CMD_EDIT_INTERVAL).contains(&id) || id == CMD_EDIT_PROFILE)
                && notification == CBN_SELCHANGE as u32
            {
                apply_editor_controls(id);
            } else if id == CMD_EDIT_PREVIOUS || id == CMD_EDIT_NEXT {
                let current = app().editor_index;
                app().editor_index = if id == CMD_EDIT_PREVIOUS {
                    (current + N - 1) % N
                } else {
                    (current + 1) % N
                };
                update_editor_controls();
            } else if id == CMD_EDIT_COPY_ALL {
                let answer=MessageBoxW(hwnd,
                    wide("Copiar as opções visuais deste widget para os outros cinco?\n\nTamanhos, posições e estados de ativação serão preservados.").as_ptr(),
                    wide("Rtime - Confirmar aparência").as_ptr(),MB_YESNO|MB_ICONQUESTION);
                if answer == IDYES {
                    let index = app().editor_index;
                    app().config.copy_widget_visual_to_all(index);
                    update_appearance();
                    save_later();
                }
            } else if id == CMD_EDIT_GROUP_TOGGLE {
                app().config.group_move = !app().config.group_move;
                update_editor_action_labels();
                update_settings();
                save_later();
            } else if id == CMD_EDIT_ARRANGE_NOW {
                match app().config.layout_mode {
                    1 => reorganize(Arrangement::Row),
                    2 => reorganize(Arrangement::Column),
                    _ => reorganize(Arrangement::Grid),
                }
            } else if id == CMD_EDIT_RESET {
                let i = app().editor_index;
                app().config.reset_widget_visual(i);
                update_editor_controls();
                update_appearance();
                save_later();
            } else if id == CMD_EDIT_CLOSE {
                ShowWindow(hwnd, SW_HIDE);
            }
            0
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_DESTROY => {
            if !app().editor_background_brush.is_null() {
                DeleteObject(app().editor_background_brush);
            }
            app().editor_background_brush = null_mut();
            if !app().editor_control_font.is_null() {
                DeleteObject(app().editor_control_font);
            }
            app().editor_control_font = null_mut();
            app().editor_font_px = 0;
            app().editor = null_mut();
            app().editor_combos = [null_mut(); 11];
            app().editor_scroll_y = 0;
            app().editor_content_h = 0;
            0
        }
        _ => DefWindowProcW(hwnd, message, w, l),
    }
}

// Controle customizado para todos os checkboxes da janela de configurações.
// Mantém as mensagens BM_GETCHECK / BM_SETCHECK e o BN_CLICKED esperado pelo app.
unsafe extern "system" fn settings_check_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    match msg {
        BM_GETCHECK => GetWindowLongPtrW(hwnd, GWLP_USERDATA) as LRESULT,
        BM_SETCHECK => {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, if w == 0 { 0 } else { 1 });
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_LBUTTONDOWN => {
            SetCapture(hwnd);
            SetFocus(hwnd);
            0
        }
        WM_LBUTTONUP => {
            let captured = GetCapture() == hwnd;
            if captured {
                ReleaseCapture();
            }
            let mut rect: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut rect);
            let x = ((l & 0xFFFF) as u16) as i16 as i32;
            let y = (((l >> 16) & 0xFFFF) as u16) as i16 as i32;
            if captured && x >= 0 && y >= 0 && x < rect.right && y < rect.bottom {
                settings_check_toggle(hwnd);
            }
            0
        }
        WM_KEYDOWN if w == 0x20 => {
            settings_check_toggle(hwnd);
            0
        }
        WM_SETFOCUS | WM_KILLFOCUS => {
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = std::mem::zeroed();
            let dc = BeginPaint(hwnd, &mut paint);
            if !dc.is_null() {
                let mut rect: RECT = std::mem::zeroed();
                GetClientRect(hwnd, &mut rect);
                settings_fill(dc, rect, ui_panel());
                let checked = GetWindowLongPtrW(hwnd, GWLP_USERDATA) != 0;
                let mut caption_buf = [0u16; 180];
                GetWindowTextW(hwnd, caption_buf.as_mut_ptr(), caption_buf.len() as i32);
                let caption_len = caption_buf
                    .iter()
                    .position(|ch| *ch == 0)
                    .unwrap_or(caption_buf.len());
                // Comportamento: controle estreito sem rótulo funciona como interruptor Fluent.
                if caption_len == 0 && rect.right <= 90 {
                    draw_rounded_fill(
                        dc,
                        RECT {
                            left: 5,
                            top: 3,
                            right: rect.right - 5,
                            bottom: rect.bottom - 3,
                        },
                        30,
                        if checked { ui_accent() } else { ui_border() },
                    );
                    let knob_x = if checked { rect.right - 30 } else { 10 };
                    draw_rounded_fill(
                        dc,
                        RECT {
                            left: knob_x,
                            top: 8,
                            right: knob_x + 20,
                            bottom: rect.bottom - 8,
                        },
                        22,
                        color(255, 255, 255),
                    );
                    /* Foco mantém contorno do próprio controle, sem pontilhado nativo. */
                    EndPaint(hwnd, &paint);
                    return 0;
                }
                let square = RECT {
                    left: 2,
                    top: 4,
                    right: 24,
                    bottom: 26,
                };
                draw_rounded_fill(
                    dc,
                    square,
                    7,
                    if checked { ui_accent() } else { ui_border() },
                );
                if checked {
                    let pen = CreatePen(PS_SOLID, 2, color(255, 255, 255));
                    let previous = SelectObject(dc, pen);
                    let points = [
                        POINT { x: 7, y: 15 },
                        POINT { x: 12, y: 20 },
                        POINT { x: 20, y: 10 },
                    ];
                    Polyline(dc, points.as_ptr(), points.len() as i32);
                    SelectObject(dc, previous);
                    DeleteObject(pen);
                }
                let mut text_buffer = [0u16; 180];
                GetWindowTextW(hwnd, text_buffer.as_mut_ptr(), text_buffer.len() as i32);
                let len = text_buffer
                    .iter()
                    .position(|ch| *ch == 0)
                    .unwrap_or(text_buffer.len());
                let text = String::from_utf16_lossy(&text_buffer[..len]);
                draw_settings_fit_text(
                    dc,
                    &text,
                    RECT {
                        left: 34,
                        top: 0,
                        right: rect.right - 4,
                        bottom: rect.bottom,
                    },
                    16,
                    false,
                    ui_ink(),
                    false,
                );
                // A indicação de seleção já está no quadrado roxo; sem halo cinza.

                EndPaint(hwnd, &paint);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, w, l),
    }
}

unsafe fn settings_check_toggle(hwnd: HWND) {
    let checked = GetWindowLongPtrW(hwnd, GWLP_USERDATA) != 0;
    SendMessageW(hwnd, BM_SETCHECK, if checked { 0 } else { 1 }, 0);
    let parent = GetParent(hwnd);
    let id = GetDlgCtrlID(hwnd);
    if !parent.is_null() && id >= 0 {
        // BN_CLICKED = 0; é preservado o contrato do botão padrão.
        SendMessageW(parent, WM_COMMAND, id as usize, hwnd as isize);
    }
}

// Desenho do conteúdo selecionado e dos itens da lista no ComboBox.
// A moldura e a seta permanecem controles nativos do Windows.
unsafe fn settings_draw_combo(info: &DRAWITEMSTRUCT) {
    let rect = info.rcItem;
    let selected = (info.itemState & ODS_SELECTED as u32) != 0;
    settings_fill(
        info.hDC,
        rect,
        if selected { ui_panel_alt() } else { ui_panel() },
    );
    if info.itemID != u32::MAX {
        // CB_GETLBTEXT nao recebe tamanho de destino. Consultar primeiro o
        // comprimento evita overflow com nomes longos de adaptadores de rede.
        let len = SendMessageW(info.hwndItem, CB_GETLBTEXTLEN, info.itemID as usize, 0);
        if len >= 0 && len < 32768 {
            let mut value = vec![0u16; len as usize + 1];
            let copied = SendMessageW(
                info.hwndItem,
                CB_GETLBTEXT,
                info.itemID as usize,
                value.as_mut_ptr() as isize,
            );
            if copied >= 0 {
                let actual = (copied as usize).min(value.len() - 1);
                let text = String::from_utf16_lossy(&value[..actual]);
                draw_settings_fit_text(
                    info.hDC,
                    &text,
                    RECT {
                        left: rect.left + 10,
                        top: rect.top,
                        right: rect.right - 6,
                        bottom: rect.bottom,
                    },
                    16,
                    false,
                    ui_ink(),
                    false,
                );
            }
        }
    }
    if (info.itemState & ODS_FOCUS as u32) != 0 {
        let mut focus = rect;
        InflateRect(&mut focus, -2, -2);
        let pen = CreatePen(PS_SOLID, 1, ui_accent());
        let previous_pen = SelectObject(info.hDC, pen);
        let previous_brush = SelectObject(info.hDC, GetStockObject(NULL_BRUSH));
        Rectangle(info.hDC, focus.left, focus.top, focus.right, focus.bottom);
        SelectObject(info.hDC, previous_brush);
        SelectObject(info.hDC, previous_pen);
        DeleteObject(pen);
    }
}

// Reflow real: os controles nativos acompanham as mesmas regiões desenhadas por GDI.
// O tamanho de fontes é adaptado ao DPI, sem redimensionar bitmap de texto.
unsafe fn settings_reflow_controls(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    let layout = settings_geometry(bounds.right, bounds.bottom, app().settings_page);
    // O cliente do viewport já está em pixels físicos (Per-Monitor DPI Aware v2).
    // Na v0.3.15, multiplicar a fonte até 200% sem aumentar os retângulos
    // quebrava rótulos, botões e itens do ComboBox. Limitar a tipografia mantém
    // a legibilidade em 100%/125%/150% sem duplicar a escala do Windows.
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let normal_px = (15 * dpi / 96).clamp(15, 17);
    let strong_px = (17 * dpi / 96).clamp(17, 19);
    let font_key = normal_px * 100 + strong_px;
    if app().settings_ui_scale != font_key || app().settings_font.is_null() {
        let face = wide("Segoe UI");
        let next = CreateFontW(
            -normal_px,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            face.as_ptr(),
        );
        let next_rate = CreateFontW(
            -strong_px,
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            face.as_ptr(),
        );
        let old_font = app().settings_font;
        let old_rate_font = app().settings_rate_font;
        if !next.is_null() {
            app().settings_font = next;
        }
        if !next_rate.is_null() {
            app().settings_rate_font = next_rate;
        }
        app().settings_ui_scale = font_key;
        for &(child, _) in &app().settings_control_rects {
            if !child.is_null() && !app().settings_font.is_null() {
                SendMessageW(child, WM_SETFONT, app().settings_font as usize, 0);
            }
        }
        if old_font != app().settings_font && !old_font.is_null() {
            DeleteObject(old_font);
        }
        if old_rate_font != app().settings_rate_font && !old_rate_font.is_null() {
            DeleteObject(old_rate_font);
        }
    }
    for &(child, base) in &app().settings_control_rects {
        if child.is_null() {
            continue;
        }
        let id = GetDlgCtrlID(child);
        let kind = if id < 0 { 0 } else { id as u32 };
        let r = layout.control_rect(
            kind,
            UiRect::new(
                base.left,
                base.top,
                base.right - base.left,
                base.bottom - base.top,
            ),
        );
        SetWindowPos(
            child,
            null_mut(),
            r.x - app().settings_scroll_x,
            r.y - app().settings_scroll_y,
            r.w.max(1),
            r.h.max(1),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn settings_scroll_ranges(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    let layout = settings_geometry(bounds.right, bounds.bottom, app().settings_page);
    let page_w = bounds.right.max(1);
    let page_h = bounds.bottom.max(1);
    let x = app()
        .settings_scroll_x
        .clamp(0, (layout.scroll_width - page_w).max(0));
    let y = app()
        .settings_scroll_y
        .clamp(0, (layout.scroll_height - page_h).max(0));
    app().settings_scroll_x = x;
    app().settings_scroll_y = y;
    let mut info: SCROLLINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
    info.fMask = SIF_RANGE | SIF_PAGE | SIF_POS;
    info.nMin = 0;
    info.nMax = layout.scroll_width.max(1) - 1;
    info.nPage = page_w as u32;
    info.nPos = x;
    SetScrollInfo(hwnd, SB_HORZ, &info, 1);
    info.nMax = layout.scroll_height.max(1) - 1;
    info.nPage = page_h as u32;
    info.nPos = y;
    SetScrollInfo(hwnd, SB_VERT, &info, 1);
    settings_reflow_controls(hwnd);
}

unsafe fn settings_scroll_to(hwnd: HWND, requested_x: i32, requested_y: i32) {
    if hwnd.is_null() {
        return;
    }
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    let layout = settings_geometry(bounds.right, bounds.bottom, app().settings_page);
    let new_x = requested_x.clamp(0, (layout.scroll_width - bounds.right.max(1)).max(0));
    let new_y = requested_y.clamp(0, (layout.scroll_height - bounds.bottom.max(1)).max(0));
    if new_x == app().settings_scroll_x && new_y == app().settings_scroll_y {
        return;
    }
    app().settings_scroll_x = new_x;
    app().settings_scroll_y = new_y;
    let mut position: SCROLLINFO = std::mem::zeroed();
    position.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
    position.fMask = SIF_POS;
    position.nPos = new_x;
    SetScrollInfo(hwnd, SB_HORZ, &position, 1);
    position.nPos = new_y;
    SetScrollInfo(hwnd, SB_VERT, &position, 1);
    settings_reflow_controls(hwnd);
}

unsafe fn settings_scroll_event(hwnd: HWND, horizontal: bool, w: WPARAM) {
    let mut info: SCROLLINFO = std::mem::zeroed();
    info.cbSize = std::mem::size_of::<SCROLLINFO>() as u32;
    info.fMask = SIF_TRACKPOS | SIF_POS | SIF_PAGE;
    GetScrollInfo(hwnd, if horizontal { SB_HORZ } else { SB_VERT }, &mut info);
    let mut next = if horizontal {
        app().settings_scroll_x
    } else {
        app().settings_scroll_y
    };
    let request = (w & 0xFFFF) as u32;
    match request {
        x if x == SB_LINEUP as u32 => next -= 32,
        x if x == SB_LINEDOWN as u32 => next += 32,
        x if x == SB_PAGEUP as u32 => next -= info.nPage as i32,
        x if x == SB_PAGEDOWN as u32 => next += info.nPage as i32,
        x if x == SB_THUMBTRACK as u32 || x == SB_THUMBPOSITION as u32 => next = info.nTrackPos,
        x if x == SB_TOP as u32 => next = 0,
        x if x == SB_BOTTOM as u32 => next = i32::MAX,
        _ => return,
    }
    if horizontal {
        settings_scroll_to(hwnd, next, app().settings_scroll_y);
    } else {
        settings_scroll_to(hwnd, app().settings_scroll_x, next);
    }
}

fn ui_win_rect(r: UiRect) -> RECT {
    RECT {
        left: r.x,
        top: r.y,
        right: r.right(),
        bottom: r.bottom(),
    }
}
fn ui_rect(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT {
        left: x,
        top: y,
        right: x + w,
        bottom: y + h,
    }
}
unsafe fn ui_text(dc: HDC, text: &str, x: i32, y: i32, w: i32, h: i32, emphasis: bool, ink: u32) {
    draw_card_text(
        dc,
        text,
        ui_rect(x, y, w.max(1), h.max(1)),
        if emphasis {
            app().rate_font
        } else if !app().settings_font.is_null() {
            app().settings_font
        } else {
            app().font
        },
        ink,
        false,
    );
}

unsafe fn ui_paragraph(dc: HDC, text: &str, x: i32, y: i32, w: i32, h: i32, ink: u32) {
    let font = if !app().settings_font.is_null() {
        app().settings_font
    } else {
        app().font
    };
    let old = SelectObject(dc, font);
    SetTextColor(dc, ink);
    SetBkMode(dc, TRANSPARENT as i32);
    let utf16 = wide(text);
    let mut rect = ui_rect(x, y, w.max(1), h.max(1));
    DrawTextW(dc, utf16.as_ptr(), -1, &mut rect, DT_LEFT | DT_WORDBREAK);
    SelectObject(dc, old);
}

// Cada aba usa os mesmos retângulos de UiLayout que o reposicionamento dos HWNDs.
// Não há mapeamento anisotrópico: títulos, cartões e campos são renderizados 1:1.
unsafe fn paint_settings_responsive(dc: HDC, layout: &UiLayout) {
    let ink = ui_ink();
    let subtle = ui_muted();
    let page_title = match layout.page {
        0 => "Aparência",
        1 => "Seus widgets",
        2 => "Organização do desktop",
        3 => "Comportamento",
        _ => "Fontes e atualização",
    };
    let description = match layout.page {
        0 => "Estilo, contraste e densidade visual dos cartões.",
        1 => "Escolha os indicadores e configure seus tamanhos.",
        2 => "Posicione e movimente os widgets em conjunto.",
        3 => "Controles do desktop, bandeja e recuperação.",
        _ => "Selecione o disco, a rede e a atualização de dados.",
    };
    let title = if layout.view_w < 530 && layout.page == 2 {
        "Organização"
    } else {
        page_title
    };
    draw_card_text(
        dc,
        title,
        ui_rect(layout.left.x, 27, layout.view_w - 48, 42),
        app().settings_heading_font,
        ink,
        false,
    );
    ui_paragraph(
        dc,
        description,
        layout.left.x,
        76,
        layout.view_w - 48,
        58,
        subtle,
    );
    match layout.page {
        0 => {
            settings_panel(dc, ui_win_rect(layout.left), 23);
            settings_panel(dc, ui_win_rect(layout.right), 23);
            let r = layout.right;
            ui_text(
                dc,
                "Prévia do estilo",
                r.x + 18,
                r.y + 18,
                r.w - 36,
                32,
                true,
                ink,
            );
            let preview = layout.appearance_preview();
            draw_rounded_fill(dc, ui_win_rect(preview), 17, color(17, 18, 37));
            let palette = current_palette();
            let cx = preview.x + 42;
            let cy = preview.y + 55;
            draw_rounded_fill(dc, ui_rect(cx - 30, cy - 30, 60, 60), 28, color(46, 37, 77));
            let brush = CreatePen(PS_SOLID, 5, palette.accent);
            if !brush.is_null() {
                let prev = SelectObject(dc, brush);
                let old_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
                Ellipse(dc, cx - 22, cy - 22, cx + 22, cy + 22);
                SelectObject(dc, old_brush);
                SelectObject(dc, prev);
                DeleteObject(brush);
            }
            draw_settings_fit_text(
                dc,
                "32%",
                ui_rect(cx - 28, cy - 14, 56, 29),
                16,
                true,
                color(255, 255, 255),
                true,
            );
            let style = STYLES[(app().config.style as usize).min(STYLES.len() - 1)];
            // Duas linhas no título da prévia, nunca quebrar o meio da palavra.
            if style == "Compact Dashboard" {
                ui_text(
                    dc,
                    "Compact",
                    preview.x + 85,
                    preview.y + 25,
                    preview.w - 93,
                    24,
                    false,
                    color(230, 222, 243),
                );
                ui_text(
                    dc,
                    "Dashboard",
                    preview.x + 85,
                    preview.y + 49,
                    preview.w - 93,
                    25,
                    false,
                    color(230, 222, 243),
                );
            } else {
                ui_text(
                    dc,
                    style,
                    preview.x + 85,
                    preview.y + 34,
                    preview.w - 93,
                    28,
                    false,
                    color(230, 222, 243),
                );
            }
            ui_text(
                dc,
                "CPU",
                preview.x + 85,
                preview.y + 84,
                preview.w - 93,
                32,
                true,
                color(255, 255, 255),
            );
            if app().config.show_graphs {
                let pen = CreatePen(PS_SOLID, 2, palette.graph);
                if !pen.is_null() {
                    let old = SelectObject(dc, pen);
                    let mut pts: [POINT; 7] = std::array::from_fn(|_| POINT { x: 0, y: 0 });
                    for (i, (rx, ry)) in [
                        (0, 68),
                        (18, 57),
                        (37, 63),
                        (56, 29),
                        (76, 45),
                        (98, 20),
                        (122, 13),
                    ]
                    .iter()
                    .copied()
                    .enumerate()
                    {
                        pts[i] = POINT {
                            x: preview.x + 14 + rx * (preview.w - 28) / 122,
                            y: preview.bottom() - 15 - ry,
                        };
                    }
                    Polyline(dc, pts.as_ptr(), pts.len() as i32);
                    SelectObject(dc, old);
                    DeleteObject(pen);
                }
            }
            // O texto fica no mesmo canvas GDI dos cartões e nunca disputa
            // área com um STATIC transparente/owner-draw durante a rolagem.
            let theme_label = layout.appearance_theme_label();
            ui_text(
                dc,
                "Tema da interface",
                theme_label.x,
                theme_label.y,
                theme_label.w,
                theme_label.h,
                false,
                ink,
            );
        }
        1 => {
            let data = app().metrics;
            let previews = [
                format!("{:.0}%", data.cpu_pct),
                format!("{:.0}%", data.ram_pct),
                if data.disk_available {
                    format!("{:.0}%", data.disk_pct)
                } else {
                    "N/D".into()
                },
                if data.network_available {
                    fmt_network(data.up_mib)
                } else {
                    "Sem rede".into()
                },
                fmt_uptime(data.uptime_ms),
                data.process_count.to_string(),
            ];
            for index in 0..N {
                let card = layout.widget_rect(index);
                settings_panel(dc, ui_win_rect(card), 20);
                let w = card.w;
                let mini = UiRect::new(
                    card.x + 17,
                    card.y + 57,
                    if w >= 270 { (w / 2).max(135) } else { w - 34 },
                    59,
                );
                draw_rounded_fill(dc, ui_win_rect(mini), 12, color(26, 27, 50));
                let val = previews[index].as_str();
                let main_top = if index == 3 && app().metrics.network_available {
                    5
                } else {
                    14
                };
                draw_widget_text(
                    dc,
                    val,
                    ui_rect(mini.x + 12, mini.y + main_top, mini.w - 23, 30),
                    1,
                    1,
                    1,
                    1,
                    19,
                    true,
                    color(246, 242, 255),
                );
                if index == 3 && app().metrics.network_available {
                    draw_widget_text(
                        dc,
                        &fmt_network_total(app().metrics.network_total_bytes),
                        ui_rect(mini.x + 12, mini.y + 34, mini.w - 23, 20),
                        1,
                        1,
                        1,
                        1,
                        12,
                        false,
                        color(175, 165, 206),
                    );
                }
                if index == 5 {
                    let top = app().metrics.top_processes[0];
                    if top.working_set_bytes > 0 {
                        draw_widget_text(
                            dc,
                            &format!(
                                "{} · {}",
                                top.display_name(),
                                fmt_process_ram(top.working_set_bytes)
                            ),
                            ui_rect(mini.x + 12, mini.y + 37, mini.w - 23, 16),
                            1,
                            1,
                            1,
                            1,
                            11,
                            false,
                            color(175, 165, 206),
                        );
                    }
                }
                if w >= 270 {
                    ui_text(
                        dc,
                        "Tamanho",
                        card.right() - 157,
                        card.y + 96,
                        140,
                        25,
                        false,
                        subtle,
                    );
                }
            }
        }
        2 => {
            settings_panel(dc, ui_win_rect(layout.left), 22);
            settings_panel(dc, ui_win_rect(layout.right), 22);
            let l = layout.left;
            ui_text(
                dc,
                "Organização",
                l.x + 20,
                l.y + 14,
                l.w - 40,
                34,
                true,
                ink,
            );
            let r = layout.right;
            ui_text(
                dc,
                "Posição do conjunto",
                r.x + 19,
                r.y + 14,
                r.w - 38,
                33,
                true,
                ink,
            );
            let grid = layout.anchor_grid();
            draw_rounded_fill(dc, ui_win_rect(grid), 14, ui_panel_alt());
            // Nove botões reais são desenhados separadamente via WM_DRAWITEM.
            if !layout.stacked {
                let separator = ui_rect(r.x + 18, r.y + 244, r.w - 36, 1);
                settings_fill(dc, separator, ui_border());
            }
        }
        3 => {
            let panel = layout.behavior_panel();
            settings_panel(dc, ui_win_rect(panel), 22);
            let rows = [
                (
                    "Manter widgets atrás das janelas",
                    "Os cartões ficam atrás dos demais aplicativos.",
                ),
                (
                    "Posição dos cartões",
                    "Trave ou destrave as posições na área de trabalho.",
                ),
                ("Visibilidade", "Exiba ou oculte todos os widgets."),
                ("Recuperação", "Recupere cartões após alterar os monitores."),
            ];
            let narrow = layout.view_w < 690;
            let row_step = if narrow { 156 } else { 112 };
            for (idx, (heading, detail)) in rows.iter().enumerate() {
                let yy = panel.y + 18 + idx as i32 * row_step;
                let header: &str = if narrow && idx == 0 {
                    "Atrás das janelas"
                } else {
                    *heading
                };
                ui_text(
                    dc,
                    header,
                    panel.x + 20,
                    yy,
                    panel.w - (if idx == 0 { 135 } else { 44 }),
                    33,
                    true,
                    ink,
                );
                ui_paragraph(dc, detail, panel.x + 20, yy + 35, panel.w - 44, 47, subtle);
                if idx < 3 {
                    settings_fill(
                        dc,
                        ui_rect(panel.x + 19, yy + row_step - 7, panel.w - 38, 1),
                        ui_border(),
                    );
                }
            }
        }
        _ => {
            settings_panel(dc, ui_win_rect(layout.left), 22);
            settings_panel(dc, ui_win_rect(layout.right), 22);
            ui_text(
                dc,
                "Fontes dos indicadores",
                layout.left.x + 18,
                layout.left.y + 18,
                layout.left.w - 36,
                34,
                true,
                ink,
            );
            ui_text(
                dc,
                "Atualização",
                layout.right.x + 18,
                layout.right.y + 18,
                layout.right.w - 36,
                34,
                true,
                ink,
            );
            ui_paragraph(
                dc,
                "Atualize a lista ao conectar outro disco ou trocar de adaptador de rede.",
                layout.right.x + 18,
                layout.right.y + 297,
                layout.right.w - 36,
                72,
                subtle,
            );
            ui_paragraph(
                dc,
                "Disco e processos são medidos em intervalos maiores para economizar recursos.",
                layout.right.x + 18,
                layout.right.y + 371,
                layout.right.w - 36,
                73,
                subtle,
            );
        }
    }
}

unsafe fn paint_settings_viewport(hwnd: HWND) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let dc = BeginPaint(hwnd, &mut ps);
    if dc.is_null() {
        return;
    }
    let mut bounds: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut bounds);
    let buffer = CreateCompatibleDC(dc);
    let bitmap = if buffer.is_null() {
        null_mut()
    } else {
        CreateCompatibleBitmap(dc, bounds.right.max(1), bounds.bottom.max(1))
    };
    let previous = if bitmap.is_null() {
        null_mut()
    } else {
        SelectObject(buffer, bitmap)
    };
    let paint = if bitmap.is_null() { dc } else { buffer };
    settings_fill(paint, bounds, ui_bg());
    // Pixel-perfect Win32 GDI; sem esticar fontes ou bitmap em tela cheia.
    let geometry = settings_geometry(bounds.right, bounds.bottom, app().settings_page);
    let saved = SaveDC(paint);
    SetViewportOrgEx(
        paint,
        -app().settings_scroll_x,
        -app().settings_scroll_y,
        null_mut(),
    );
    paint_settings_responsive(paint, &geometry);
    RestoreDC(paint, saved);
    if !bitmap.is_null() {
        BitBlt(dc, 0, 0, bounds.right, bounds.bottom, paint, 0, 0, SRCCOPY);
        SelectObject(buffer, previous);
        DeleteObject(bitmap);
    }
    if !buffer.is_null() {
        DeleteDC(buffer);
    }
    EndPaint(hwnd, &ps);
}

// Tab/Shift+Tab: traz para a área visível o controle que recebeu foco.
unsafe fn settings_ensure_control_visible(viewport: HWND, child: HWND) {
    if viewport.is_null() || child.is_null() {
        return;
    }
    let mut box_rect: RECT = std::mem::zeroed();
    let mut frame: RECT = std::mem::zeroed();
    if GetWindowRect(child, &mut box_rect) == 0 {
        return;
    }
    GetClientRect(viewport, &mut frame);
    let mut origin = POINT {
        x: box_rect.left,
        y: box_rect.top,
    };
    ScreenToClient(viewport, &mut origin);
    let width = box_rect.right - box_rect.left;
    let height = box_rect.bottom - box_rect.top;
    let mut x = app().settings_scroll_x;
    let mut y = app().settings_scroll_y;
    if origin.x < 8 {
        x += origin.x - 8;
    } else if origin.x + width > frame.right - 8 {
        x += origin.x + width - frame.right + 8;
    }
    if origin.y < 8 {
        y += origin.y - 8;
    } else if origin.y + height > frame.bottom - 8 {
        y += origin.y + height - frame.bottom + 8;
    }
    settings_scroll_to(viewport, x, y);
}

unsafe extern "system" fn settings_viewport_proc(
    hwnd: HWND,
    message: u32,
    w: WPARAM,
    l: LPARAM,
) -> LRESULT {
    match message {
        WM_SIZE => {
            settings_scroll_ranges(hwnd);
            0
        }
        WM_VSCROLL if l == 0 => {
            settings_scroll_event(hwnd, false, w);
            0
        }
        WM_HSCROLL if l == 0 => {
            settings_scroll_event(hwnd, true, w);
            0
        }
        WM_MOUSEWHEEL => {
            let wheel = ((w >> 16) as u16) as i16 as i32;
            let step = if wheel > 0 { -64 } else { 64 };
            if wheel != 0 {
                settings_scroll_to(
                    hwnd,
                    app().settings_scroll_x,
                    app().settings_scroll_y + step,
                );
            }
            0
        }
        WM_MOUSEHWHEEL => {
            let wheel = ((w >> 16) as u16) as i16 as i32;
            let step = if wheel > 0 { 64 } else { -64 };
            if wheel != 0 {
                settings_scroll_to(
                    hwnd,
                    app().settings_scroll_x + step,
                    app().settings_scroll_y,
                );
            }
            0
        }
        WM_COMMAND | WM_DRAWITEM | WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX
        | WM_CTLCOLOREDIT => {
            if message == WM_COMMAND && l != 0 {
                let notify = ((w >> 16) & 0xFFFF) as u32;
                if notify == BN_SETFOCUS as u32 || notify == CBN_SETFOCUS as u32 {
                    settings_ensure_control_visible(hwnd, l as HWND);
                }
            }
            let parent = app().settings;
            if parent.is_null() {
                0
            } else {
                SendMessageW(parent, message, w, l)
            }
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint_settings_viewport(hwnd);
            0
        }
        _ => DefWindowProcW(hwnd, message, w, l),
    }
}

unsafe extern "system" fn settings_proc(hwnd: HWND, message: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match message {
        WM_CREATE => {
            app().settings_main_brush = CreateSolidBrush(ui_bg());
            app().settings_card_brush = CreateSolidBrush(ui_panel());
            let mut bounds: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut bounds);
            app().settings_viewport = window(
                WS_EX_CONTROLPARENT,
                SETTINGS_PAGE_CLASS,
                "",
                WS_CHILD | WS_VISIBLE | WS_CLIPCHILDREN | WS_VSCROLL | WS_HSCROLL,
                SETTINGS_SIDEBAR_WIDTH,
                0,
                (bounds.right - SETTINGS_SIDEBAR_WIDTH).max(1),
                (bounds.bottom - SETTINGS_FOOTER_HEIGHT).max(1),
                hwnd,
                0,
                null(),
            );
            if app().settings_viewport.is_null() {
                return -1;
            }
            create_settings_controls(hwnd);
            settings_scroll_ranges(app().settings_viewport);
            0
        }
        WM_SIZE => {
            // Processar a ocultacao APOS o WM_SIZE para nao entrar no
            // ciclo de minimizacao/restauracao do Windows reentrantemente.
            if w == SIZE_MINIMIZED as usize {
                PostMessageW(hwnd, EVT_HIDE_AUX, 0, 0);
                return 0;
            }
            let mut bounds: RECT = std::mem::zeroed();
            GetClientRect(hwnd, &mut bounds);
            if !app().settings_viewport.is_null() {
                SetWindowPos(
                    app().settings_viewport,
                    null_mut(),
                    SETTINGS_SIDEBAR_WIDTH,
                    0,
                    (bounds.right - SETTINGS_SIDEBAR_WIDTH).max(1),
                    (bounds.bottom - SETTINGS_FOOTER_HEIGHT).max(1),
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                settings_scroll_ranges(app().settings_viewport);
            }
            let close = GetDlgItem(hwnd, CMD_SETTINGS_CLOSE as i32);
            if !close.is_null() {
                SetWindowPos(
                    close,
                    null_mut(),
                    (bounds.right - 130).max(SETTINGS_SIDEBAR_WIDTH),
                    (bounds.bottom - SETTINGS_FOOTER_HEIGHT + 12).max(0),
                    112,
                    42,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            // A sidebar reduz o passo vertical para manter os cinco itens
            // acessíveis quando a altura da tela é pequena.
            let nav_top = if bounds.bottom < 450 { 104 } else { 137 };
            let nav_step = ((bounds.bottom - nav_top - 44 - 8) / 4).clamp(38, 61);
            for (i, &tab) in app().settings_tabs.iter().enumerate() {
                if !tab.is_null() {
                    SetWindowPos(
                        tab,
                        null_mut(),
                        17,
                        nav_top + i as i32 * nav_step,
                        214,
                        42.min(nav_step - 2),
                        SWP_NOZORDER | SWP_NOACTIVATE,
                    );
                }
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        EVT_HIDE_AUX => {
            // Evita ocultar uma janela que ja foi restaurada pelo usuario.
            if IsIconic(hwnd) != 0 {
                ShowWindow(hwnd, SW_HIDE);
            }
            0
        }
        WM_SYSCOMMAND if minimize_auxiliary_command(w) => {
            // Ocultar, sem deixar a barra de titulo reduzida sobre os widgets.
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_SYSCOMMAND if forbidden_resize_command(w) => 0,
        WM_NCLBUTTONDBLCLK if w == HTCAPTION as usize => 0,
        WM_SIZING => 0,
        WM_DPICHANGED => {
            if l != 0 {
                let suggested = &*(l as *const RECT);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            if !app().settings_viewport.is_null() {
                settings_scroll_ranges(app().settings_viewport);
            }
            0
        }
        WM_SHOWWINDOW => {
            if w != 0 {
                SetTimer(hwnd, TIMER_SETTINGS_HOVER, SETTINGS_HOVER_POLL_MS, None);
            } else {
                KillTimer(hwnd, TIMER_SETTINGS_HOVER);
                app().settings_hover = null_mut();
            }
            0
        }
        WM_TIMER if w == TIMER_SETTINGS_HOVER => {
            refresh_settings_hover(hwnd);
            0
        }
        WM_KEYDOWN if w == 0x7A => 0, // F11 bloqueado em Configurações.
        WM_KEYDOWN if w == VK_ESCAPE as usize => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_COMMAND => {
            let id = (w & 0xFFFF) as u32;
            let notification = ((w >> 16) & 0xFFFF) as u32;
            if id == IDCANCEL as u32 {
                ShowWindow(hwnd, SW_HIDE);
            } else if (CMD_TAB_BASE..CMD_TAB_BASE + 5).contains(&id) {
                show_settings_page(hwnd, (id - CMD_TAB_BASE) as usize);
            } else if id == CMD_SETTINGS_CLOSE {
                ShowWindow(hwnd, SW_HIDE);
            } else if (CMD_LAYOUT_ANCHOR_TILE_BASE..CMD_LAYOUT_ANCHOR_TILE_BASE + 9).contains(&id) {
                app().config.layout_anchor = id - CMD_LAYOUT_ANCHOR_TILE_BASE;
                for i in 0..9 {
                    let tile = GetDlgItem(
                        app().settings_viewport,
                        (CMD_LAYOUT_ANCHOR_TILE_BASE + i) as i32,
                    );
                    if !tile.is_null() {
                        InvalidateRect(tile, null(), 0);
                    }
                }
                save_later();
            } else if (CMD_LAYOUT_MODE..=CMD_LAYOUT_GAP).contains(&id) {
                if notification == CBN_SELCHANGE as u32 {
                    read_layout_controls(id);
                }
            } else if id == CMD_GROUP_MOVE || id == CMD_SNAP {
                if notification == BN_CLICKED as u32 {
                    read_layout_controls(id);
                }
            } else if (CMD_MONITOR_DISK..=CMD_MONITOR_INTERVAL).contains(&id) {
                if notification == CBN_SELCHANGE as u32 {
                    read_monitor_choices(id);
                }
            } else if id == CMD_MONITOR_REFRESH {
                if notification == BN_CLICKED as u32 {
                    refill_monitor_sources();
                }
            } else if id == CMD_UI_THEME {
                if notification == CBN_SELCHANGE as u32 {
                    let theme = selected_combo(app().ui_theme_combo).min(1);
                    if app().config.interface_theme != theme {
                        app().config.interface_theme = theme;
                        refresh_interface_theme();
                        save_later();
                    }
                }
            } else if (CMD_STYLE..=CMD_OPACITY).contains(&id)
                || (CMD_CARD_SIZE_BASE..CMD_CARD_SIZE_BASE + N as u32).contains(&id)
            {
                if notification == CBN_SELCHANGE as u32 {
                    read_appearance_controls();
                }
            } else if (CMD_SHOW_GRAPHS..=CMD_SHOW_SUBTITLES).contains(&id) {
                if notification == BN_CLICKED as u32 {
                    read_appearance_controls();
                }
            } else {
                PostMessageW(app().host, WM_COMMAND, id as usize, 0);
            }
            0
        }
        WM_DRAWITEM => {
            if l != 0 {
                let item = &*(l as *const DRAWITEMSTRUCT);
                if item.CtlType == 3 {
                    // ODT_COMBOBOX
                    settings_draw_combo(item);
                } else {
                    settings_draw_button(item);
                }
            }
            1
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORLISTBOX | WM_CTLCOLOREDIT => {
            let dc = w as HDC;
            SetTextColor(dc, ui_ink());
            SetBkMode(dc, TRANSPARENT as i32);
            let mut top_left = POINT { x: 0, y: 0 };
            ClientToScreen(l as HWND, &mut top_left);
            ScreenToClient(hwnd, &mut top_left);
            // STATICs pertencentes ao viewport são rótulos sobre painéis brancos.
            // Não decidir pela posição da tela: ao rolar, um rótulo poderia
            // receber o pincel do fundo e deixar faixas sobre o cartão.
            if !app().settings_viewport.is_null() && GetParent(l as HWND) == app().settings_viewport
            {
                app().settings_card_brush as LRESULT
            } else if top_left.x < SETTINGS_SIDEBAR_WIDTH || top_left.y < 140 {
                app().settings_main_brush as LRESULT
            } else {
                app().settings_card_brush as LRESULT
            }
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint_settings_window(hwnd);
            0
        }
        WM_CLOSE => {
            ShowWindow(hwnd, SW_HIDE);
            0
        }
        WM_DESTROY => {
            KillTimer(hwnd, TIMER_SETTINGS_HOVER);
            app().settings_hover = null_mut();
            if !app().settings_main_brush.is_null() {
                DeleteObject(app().settings_main_brush);
            }
            if !app().settings_card_brush.is_null() {
                DeleteObject(app().settings_card_brush);
            }
            app().settings_main_brush = null_mut();
            app().settings_card_brush = null_mut();
            app().settings = null_mut();
            app().settings_viewport = null_mut();
            app().settings_scroll_x = 0;
            app().settings_scroll_y = 0;
            app().appearance_combos = [null_mut(); 5];
            app().card_size_combos = [null_mut(); N];
            app().layout_combos = [null_mut(); 4];
            app().monitor_combos = [null_mut(); 3];
            app().disk_choice_ids.clear();
            app().adapter_choice_ids.clear();
            app().settings_tabs = [null_mut(); 5];
            app().settings_pages = std::array::from_fn(|_| Vec::new());
            app().group_checkbox = null_mut();
            app().snap_checkbox = null_mut();
            app().behind_checkbox = null_mut();
            app().show_button = null_mut();
            app().lock_button = null_mut();
            app().graph_checkbox = null_mut();
            app().bar_checkbox = null_mut();
            app().subtitle_checkbox = null_mut();
            0
        }
        _ => DefWindowProcW(hwnd, message, w, l),
    }
}

fn color(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | ((g as u32) << 8) | ((b as u32) << 16)
}

#[derive(Clone, Copy)]
struct Palette {
    background: u32,
    foreground: u32,
    subtle: u32,
    divider: u32,
    track: u32,
    accent: u32,
    graph: u32,
    border: u32,
}

fn palette_for(style: u32, light: bool, accent_index: u32) -> Palette {
    let default_accent = match style {
        0 => color(22, 117, 235),
        2 => color(168, 123, 255),
        _ => {
            if light {
                color(71, 79, 92)
            } else {
                color(214, 219, 226)
            }
        }
    };
    let accent = match accent_index {
        1 => color(22, 117, 235),
        2 => color(168, 123, 255),
        3 => color(62, 190, 145),
        4 => color(232, 165, 68),
        5 => color(235, 115, 114),
        _ => default_accent,
    };
    let (background, foreground, subtle, divider, track, border) = match (style, light) {
        (0, true) => (
            color(231, 240, 255),
            color(22, 44, 80),
            color(83, 109, 140),
            color(205, 221, 245),
            color(198, 217, 242),
            color(254, 254, 255),
        ),
        (0, false) => (
            color(29, 44, 72),
            color(240, 245, 255),
            color(172, 194, 229),
            color(68, 91, 129),
            color(59, 83, 123),
            color(92, 118, 155),
        ),
        (1, true) => (
            color(246, 247, 249),
            color(25, 28, 33),
            color(93, 100, 110),
            color(211, 215, 221),
            color(225, 227, 232),
            color(203, 208, 215),
        ),
        (1, false) => (
            color(23, 25, 29),
            color(242, 242, 244),
            color(166, 170, 178),
            color(57, 61, 69),
            color(62, 65, 72),
            color(53, 56, 63),
        ),
        (2, true) => (
            color(231, 235, 249),
            color(29, 43, 76),
            color(95, 108, 146),
            color(204, 213, 236),
            color(206, 216, 243),
            color(192, 205, 233),
        ),
        _ => (
            color(26, 37, 64),
            color(244, 241, 255),
            color(173, 185, 212),
            color(57, 71, 102),
            color(59, 70, 104),
            color(73, 85, 123),
        ),
    };
    Palette {
        background,
        foreground,
        subtle,
        divider,
        track,
        accent,
        graph: accent,
        border,
    }
}

unsafe fn current_palette() -> Palette {
    let light = match app().config.theme {
        1 => true,
        2 => app().system_light,
        _ => false,
    };
    palette_for(app().config.style, light, app().config.accent)
}

#[derive(Clone, Copy)]
struct CardVisual {
    style: u32,
    palette: Palette,
    graph: u32, // 0=off, 1=line, 2=area, 3=vertical bars
    font: u32,
    border: u32,
    alpha: u8,
    bars: bool,
    subtitles: bool,
}

fn choice_bool(choice: u32, inherited: bool) -> bool {
    match choice {
        1 => true,
        2 => false,
        _ => inherited,
    }
}

fn opacity_alpha(selection: u32) -> u8 {
    match selection {
        1 => 240,
        2 => 219,
        _ => 255,
    }
}

// Resolucao pura: permite testar herdanca sem depender de HWND ou Registro.
fn visual_for(cfg: &Config, index: usize, system_light: bool) -> CardVisual {
    let index = index.min(N - 1);
    let style = if cfg.widget_styles[index] == 0 {
        cfg.style
    } else {
        cfg.widget_styles[index] - 1
    };
    let accent = match cfg.widget_accents[index] {
        0 => cfg.accent,
        1 => 0,
        other => other - 1,
    };
    let light = match cfg.theme {
        1 => true,
        2 => system_light,
        _ => false,
    };
    let graph = match cfg.widget_graphs[index] {
        1 => 1,
        2 => 2,
        3 => 3,
        4 => 0,
        _ => {
            if cfg.show_graphs {
                1
            } else {
                0
            }
        }
    };
    let alpha = if cfg.widget_opacities[index] == 0 {
        opacity_alpha(cfg.opacity)
    } else {
        opacity_alpha(cfg.widget_opacities[index] - 1)
    };
    CardVisual {
        style,
        palette: palette_for(style, light, accent),
        graph,
        font: cfg.widget_fonts[index],
        border: cfg.widget_borders[index],
        alpha,
        bars: choice_bool(cfg.widget_bars[index], cfg.show_bars),
        subtitles: choice_bool(cfg.widget_subtitles[index], cfg.show_subtitles),
    }
}

fn blend_color(a: u32, b: u32, weight_b: u32) -> u32 {
    let w = weight_b.min(100);
    let mix = |shift: u32| -> u8 {
        let x = (a >> shift) & 0xFF;
        let y = (b >> shift) & 0xFF;
        ((x * (100 - w) + y * w) / 100) as u8
    };
    color(mix(0), mix(8), mix(16))
}

// Texto de controles da janela de configurações: mede a largura antes de desenhar.
// Usa uma fonte menor (até 11px) quando preciso, sem cortar palavras no layout normal.
unsafe fn draw_settings_fit_text(
    dc: HDC,
    text: &str,
    rect: RECT,
    nominal_px: i32,
    bold: bool,
    ink: u32,
    centered: bool,
) {
    let available = (rect.right - rect.left - 8).max(1);
    let vertical = (rect.bottom - rect.top - 2).max(1);
    let glyphs = wide(text);
    let len = glyphs.len().saturating_sub(1) as i32;
    let face = wide("Segoe UI");
    let weight = if bold {
        FW_BOLD as i32
    } else {
        FW_NORMAL as i32
    };
    let mut selected_font: HFONT = null_mut();
    let mut fitted = false;
    for px in (11..=nominal_px.max(11)).rev() {
        let candidate = CreateFontW(-px, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
        if candidate.is_null() {
            continue;
        }
        let previous = SelectObject(dc, candidate);
        let mut measured: SIZE = std::mem::zeroed();
        let ok = GetTextExtentPoint32W(dc, glyphs.as_ptr(), len, &mut measured) != 0;
        SelectObject(dc, previous);
        if ok && measured.cx <= available && measured.cy <= vertical {
            selected_font = candidate;
            fitted = true;
            break;
        }
        DeleteObject(candidate);
    }
    if selected_font.is_null() {
        selected_font = CreateFontW(-11, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
    }
    let old = if selected_font.is_null() {
        null_mut()
    } else {
        SelectObject(dc, selected_font)
    };
    SetBkMode(dc, TRANSPARENT as i32);
    SetTextColor(dc, ink);
    let mut target = rect;
    let horizontal = if centered { DT_CENTER } else { DT_LEFT };
    let flags =
        horizontal | DT_VCENTER | DT_SINGLELINE | (if fitted { 0 } else { DT_END_ELLIPSIS });
    DrawTextW(dc, glyphs.as_ptr(), -1, &mut target, flags);
    if !old.is_null() {
        SelectObject(dc, old);
    }
    if !selected_font.is_null() {
        DeleteObject(selected_font);
    }
}

unsafe fn draw_card_text(dc: HDC, text: &str, rect: RECT, font: HFONT, ink: u32, centered: bool) {
    let old_font = if !font.is_null() {
        SelectObject(dc, font)
    } else {
        null_mut()
    };
    SetTextColor(dc, ink);
    SetBkMode(dc, TRANSPARENT as i32);
    let utf16 = wide(text);
    let mut area = rect;
    let alignment = if centered { DT_CENTER } else { DT_LEFT };
    DrawTextW(
        dc,
        utf16.as_ptr(),
        -1,
        &mut area,
        alignment | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS,
    );
    if !old_font.is_null() {
        SelectObject(dc, old_font);
    }
}

// Texto dos cartões é renderizado diretamente em pixels físicos com ClearType.
// Somente o desenho geométrico continua no sistema lógico anisotrópico.
// Fontes Segoe UI em resolucao fisica (DPI) e ajustadas pela largura E altura
// disponiveis. O cartao nunca precisa ampliar um bitmap com texto rasterizado.
unsafe fn draw_widget_text(
    dc: HDC,
    text: &str,
    base: RECT,
    logical_w: i32,
    logical_h: i32,
    physical_w: i32,
    physical_h: i32,
    font_px: i32,
    bold: bool,
    ink: u32,
) {
    if text.is_empty() {
        return;
    }
    let area = RECT {
        left: base.left * physical_w / logical_w.max(1),
        top: base.top * physical_h / logical_h.max(1),
        right: base.right * physical_w / logical_w.max(1),
        bottom: base.bottom * physical_h / logical_h.max(1),
    };
    let available_w = (area.right - area.left - 3).max(1);
    let available_h = (area.bottom - area.top - 2).max(1);
    if available_w < 4 || available_h < 7 {
        return;
    }
    let content = wide(text);
    let characters = content.len().saturating_sub(1) as i32;
    let saved = SaveDC(dc);
    SetMapMode(dc, MM_TEXT);
    let weight = if bold {
        FW_BOLD as i32
    } else {
        FW_NORMAL as i32
    };
    let initial = ((font_px * physical_h + logical_h / 2) / logical_h.max(1))
        .max(9)
        .min(available_h);
    let face = wide("Segoe UI");
    let first = CreateFontW(
        -initial,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        face.as_ptr(),
    );
    let mut active_font = first;
    if !first.is_null() {
        let previous = SelectObject(dc, first);
        let mut measured: SIZE = std::mem::zeroed();
        let ok = GetTextExtentPoint32W(dc, content.as_ptr(), characters, &mut measured) != 0;
        SelectObject(dc, previous);
        if ok && (measured.cx > available_w || measured.cy > available_h) {
            // O limite evita letras cortadas mesmo no modo "Numeros maiores".
            let by_width = initial as i64 * available_w as i64 / measured.cx.max(1) as i64;
            let by_height = initial as i64 * available_h as i64 / measured.cy.max(1) as i64;
            let fitted = (by_width.min(by_height) as i32).clamp(7.min(initial), initial);
            if fitted < initial {
                let smaller = CreateFontW(
                    -fitted,
                    0,
                    0,
                    0,
                    weight,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0,
                    5,
                    0,
                    face.as_ptr(),
                );
                if !smaller.is_null() {
                    DeleteObject(first);
                    active_font = smaller;
                }
            }
        }
    }
    if !active_font.is_null() {
        let previous = SelectObject(dc, active_font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, ink);
        let mut target = area;
        DrawTextW(
            dc,
            content.as_ptr(),
            -1,
            &mut target,
            DT_LEFT | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX,
        );
        SelectObject(dc, previous);
        DeleteObject(active_font);
    }
    RestoreDC(dc, saved);
}

fn fmt_process_ram(bytes: u64) -> String {
    const MIB: f64 = 1024.0 * 1024.0;
    let mib = bytes as f64 / MIB;
    if mib >= 1024.0 {
        format!("{:.1} GiB", mib / 1024.0)
    } else {
        format!("{:.0} MiB", mib)
    }
}

unsafe fn draw_process_overview(
    dc: HDC,
    data: &Metrics,
    layout: &CardTextLayout,
    logical_w: i32,
    logical_h: i32,
    physical_w: i32,
    physical_h: i32,
    pal: Palette,
) {
    if layout.process_row_count == 0 {
        return;
    }
    draw_widget_text(
        dc,
        "Mais RAM",
        card_rect(layout.process_heading),
        logical_w,
        logical_h,
        physical_w,
        physical_h,
        12,
        true,
        pal.subtle,
    );
    let mut found = false;
    for (rank, area) in layout
        .process_rows
        .iter()
        .take(layout.process_row_count)
        .enumerate()
    {
        let process = data.top_processes[rank];
        if process.working_set_bytes == 0 {
            break;
        }
        found = true;
        let gap = 5;
        // A RAM fica alinhada à direita; a fonte do nome encolhe somente
        // quando o executável é muito longo. Não há ícones nem gráficos falsos.
        let ram_w = if logical_w < 200 { 52 } else { 66 };
        let name_width = (area.w - ram_w - gap).max(20);
        let name = BoxRect::new(area.x, area.y, name_width, area.h);
        let memory = BoxRect::new(area.x + name_width + gap, area.y, ram_w, area.h);
        draw_widget_text(
            dc,
            &process.display_name(),
            card_rect(name),
            logical_w,
            logical_h,
            physical_w,
            physical_h,
            layout.process_row_px,
            false,
            pal.foreground,
        );
        draw_widget_text(
            dc,
            &fmt_process_ram(process.working_set_bytes),
            card_rect(memory),
            logical_w,
            logical_h,
            physical_w,
            physical_h,
            layout.process_row_px - 1,
            false,
            pal.subtle,
        );
    }
    if !found {
        draw_widget_text(
            dc,
            "Sem dados de RAM",
            card_rect(layout.process_rows[0]),
            logical_w,
            logical_h,
            physical_w,
            physical_h,
            layout.process_row_px,
            false,
            pal.subtle,
        );
    }
}

fn card_rect(box_rect: BoxRect) -> RECT {
    RECT {
        left: box_rect.x,
        top: box_rect.y,
        right: box_rect.right(),
        bottom: box_rect.bottom(),
    }
}

unsafe fn draw_rounded_fill(dc: HDC, area: RECT, radius: i32, color_value: u32) {
    let brush = CreateSolidBrush(color_value);
    if brush.is_null() {
        return;
    }
    let original_brush = SelectObject(dc, brush);
    let original_pen = SelectObject(dc, GetStockObject(NULL_PEN));
    RoundRect(
        dc,
        area.left,
        area.top,
        area.right,
        area.bottom,
        radius,
        radius,
    );
    SelectObject(dc, original_pen);
    SelectObject(dc, original_brush);
    DeleteObject(brush);
}

fn metric_point(index: usize, m: Metrics) -> f64 {
    match index {
        0 => m.cpu_pct,
        1 => m.ram_pct,
        2 => m.disk_pct,
        3 => m.up_mib.max(0.0),
        5 => m.process_count as f64,
        _ => 0.0,
    }
}

// Escala do gráfico calculada em uma função pura e com tipo f64 explícito.
// Evita os erros E0689 (tipo ambíguo) e E0425 (variável fora de escopo).
fn history_scale(index: usize, samples: &[f64]) -> f64 {
    if index <= 2 {
        100.0_f64
    } else {
        let peak: f64 = samples.iter().copied().fold(1.0_f64, f64::max);
        peak * 1.2_f64
    }
}

unsafe fn draw_history(dc: HDC, index: usize, area: RECT, palette: Palette, mode: u32) {
    if index == 4 || app().history.len < 2 {
        return;
    }
    let len = app().history.len;
    let mut values = [0.0_f64; 60];
    for (i, slot) in values.iter_mut().enumerate().take(len) {
        *slot = metric_point(index, app().history.get(i));
    }
    let graph_scale: f64 = if index == 3 {
        // Escala compartilhada para RX e TX: facilita comparar o trafego.
        let mut samples = [0.0_f64; 120];
        for n in 0..len {
            let metric = app().history.get(n);
            samples[2 * n] = metric.down_mib.max(0.0);
            samples[2 * n + 1] = metric.up_mib.max(0.0);
        }
        history_scale(index, &samples[..len * 2])
    } else {
        history_scale(index, &values[..len])
    };
    let width = (area.right - area.left - 1).max(1);
    let height = (area.bottom - area.top - 1).max(1);
    let mut points: [POINT; 60] = std::array::from_fn(|_| POINT { x: 0, y: 0 });
    for n in 0..len {
        let normalized = (values[n] / graph_scale).clamp(0.0, 1.0);
        points[n] = POINT {
            x: area.left + (n as i32 * width) / (len as i32 - 1),
            y: area.bottom - 1 - (height as f64 * normalized) as i32,
        };
    }
    if mode == 3 {
        let brush = CreateSolidBrush(palette.graph);
        if brush.is_null() {
            return;
        }
        for point in points.iter().take(len) {
            let bar = RECT {
                left: point.x - 1,
                top: point.y,
                right: point.x + 2,
                bottom: area.bottom,
            };
            FillRect(dc, &bar, brush);
        }
        DeleteObject(brush);
        return;
    }
    if mode == 2 {
        // Area opaca misturada ao fundo: continua 100% GDI, sem WebView.
        let shaded = blend_color(palette.background, palette.graph, 18);
        let brush = CreateSolidBrush(shaded);
        if !brush.is_null() {
            let old_brush = SelectObject(dc, brush);
            let old_pen = SelectObject(dc, GetStockObject(NULL_PEN));
            let mut area_points: Vec<POINT> = Vec::with_capacity(len + 2);
            area_points.push(POINT {
                x: points[0].x,
                y: area.bottom,
            });
            area_points.extend_from_slice(&points[..len]);
            area_points.push(POINT {
                x: points[len - 1].x,
                y: area.bottom,
            });
            Polygon(dc, area_points.as_ptr(), area_points.len() as i32);
            SelectObject(dc, old_pen);
            SelectObject(dc, old_brush);
            DeleteObject(brush);
        }
    }
    let pen = CreatePen(PS_SOLID, 2, palette.graph);
    if pen.is_null() {
        return;
    }
    let original_pen = SelectObject(dc, pen);
    Polyline(dc, points.as_ptr(), len as i32);
    SelectObject(dc, original_pen);
    DeleteObject(pen);
    if index == 3 {
        // O upload agora é a linha principal, coerente com a métrica "Envio".
        // O download permanece visível como linha secundária, mais discreta.
        for n in 0..len {
            let down = app().history.get(n).down_mib;
            let normalized = (down / graph_scale).clamp(0.0, 1.0);
            points[n].y = area.bottom - 1 - (height as f64 * normalized) as i32;
        }
        let secondary_ink = blend_color(palette.background, palette.graph, 40);
        let secondary_pen = CreatePen(PS_SOLID, 1, secondary_ink);
        if !secondary_pen.is_null() {
            let previous = SelectObject(dc, secondary_pen);
            Polyline(dc, points.as_ptr(), len as i32);
            SelectObject(dc, previous);
            DeleteObject(secondary_pen);
        }
    }
}

unsafe fn paint_card(hwnd: HWND, i: usize) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let dc = BeginPaint(hwnd, &mut ps);
    if dc.is_null() {
        return;
    }
    // O cartão tem um tamanho lógico, mas o bitmap precisa nascer na resolução
    // FÍSICA do monitor. Evita StretchBlt/COLORONCOLOR ampliando letras e pixels.
    let (width, height) = dimensions(app().config.card_sizes[i]);
    let mut client: RECT = std::mem::zeroed();
    GetClientRect(hwnd, &mut client);
    let rendered_w = client.right.max(1);
    let rendered_h = client.bottom.max(1);
    let buffer_dc = CreateCompatibleDC(dc);
    if buffer_dc.is_null() {
        EndPaint(hwnd, &ps);
        return;
    }
    let bitmap = CreateCompatibleBitmap(dc, rendered_w, rendered_h);
    if bitmap.is_null() {
        DeleteDC(buffer_dc);
        EndPaint(hwnd, &ps);
        return;
    }
    let old_bitmap = SelectObject(buffer_dc, bitmap);
    // A mesma disposição lógica é desenhada vetorialmente em pixels reais.
    // Em vez de copiar um bitmap pequeno, o GDI renderiza texto e traços
    // diretamente na densidade do monitor (ClearType/antialias da fonte).
    SetMapMode(buffer_dc, MM_ANISOTROPIC);
    SetWindowExtEx(buffer_dc, width, height, null_mut());
    SetViewportExtEx(buffer_dc, rendered_w, rendered_h, null_mut());
    let visual = visual_for(&app().config, i, app().system_light);
    let palette = visual.palette;
    let style = visual.style;
    let size = app().config.card_sizes[i];
    let compact = size == 0;

    let entire = RECT {
        left: 0,
        top: 0,
        right: width,
        bottom: height,
    };
    let background = CreateSolidBrush(palette.background);
    FillRect(buffer_dc, &entire, background);
    DeleteObject(background);
    let border_color = if visual.border == 1 {
        palette.accent
    } else {
        palette.border
    };
    let pen = if visual.border == 2 {
        null_mut()
    } else {
        CreatePen(PS_SOLID, 1, border_color)
    };
    if !pen.is_null() {
        let old_pen = SelectObject(buffer_dc, pen);
        let old_brush = SelectObject(buffer_dc, GetStockObject(NULL_BRUSH));
        let rounding = if style == 1 { 18 } else { 26 };
        RoundRect(buffer_dc, 1, 1, width - 1, height - 1, rounding, rounding);
        SelectObject(buffer_dc, old_brush);
        SelectObject(buffer_dc, old_pen);
        DeleteObject(pen);
    }
    if style == 0 || style == 2 {
        let marker = RECT {
            left: if style == 0 { 0 } else { 12 },
            top: if style == 0 { 0 } else { 0 },
            right: if style == 0 { 4 } else { width - 12 },
            bottom: if style == 0 { height } else { 3 },
        };
        let brush = CreateSolidBrush(palette.accent);
        FillRect(buffer_dc, &marker, brush);
        DeleteObject(brush);
    }

    let data = app().metrics;
    let show_network_total = i == 3 && data.network_available && visual.subtitles;
    let kind = if i == 5 {
        MetricKind::Processes
    } else if show_network_total {
        MetricKind::Network
    } else if i == 4 {
        MetricKind::Activity
    } else {
        MetricKind::Normal
    };
    let typography = CardTextLayout::new(size, kind, visual.font);
    // Sem icones decorativos: apenas titulo, valor, legenda e indicadores.
    draw_widget_text(
        buffer_dc,
        NAMES[i],
        card_rect(typography.title),
        width,
        height,
        rendered_w,
        rendered_h,
        typography.title_px,
        false,
        palette.subtle,
    );
    if style == 2 {
        // O título do ranking fica na coluna direita do cartão horizontal;
        // o divisor não pode atravessar as letras de "Mais RAM".
        let divider = RECT {
            left: 16,
            top: typography.title.bottom(),
            right: if i == 5 && size == 3 { 210 } else { width - 16 },
            bottom: typography.title.bottom() + 1,
        };
        let brush = CreateSolidBrush(palette.divider);
        FillRect(buffer_dc, &divider, brush);
        DeleteObject(brush);
    }

    let (main_text, small_text, percentage) = match i {
        0 => (
            format!("{:.0}%", data.cpu_pct),
            "Uso total da CPU".to_string(),
            Some(data.cpu_pct),
        ),
        1 => (
            format!("{:.0}%", data.ram_pct),
            if data.ram_total_gib > 0.0 {
                format!("{:.1} / {:.1} GiB", data.ram_used_gib, data.ram_total_gib)
            } else {
                "Memória utilizada".to_string()
            },
            Some(data.ram_pct),
        ),
        2 => {
            if data.disk_available {
                (
                    format!("{:.0}%", data.disk_pct),
                    format!(
                        "{} · {:.0} GiB livres",
                        disk_label(app().config.monitor_disk),
                        data.disk_free_gib
                    ),
                    Some(data.disk_pct),
                )
            } else {
                (
                    if compact { "N/D" } else { "Indisponível" }.into(),
                    "Selecione uma unidade válida".into(),
                    None,
                )
            }
        }
        3 => {
            if data.network_available {
                (
                    fmt_network(data.up_mib),
                    "Total de dados utilizados".into(),
                    None,
                )
            } else {
                (
                    if compact { "Offline" } else { "Sem conexão" }.into(),
                    "Selecione uma interface ativa".into(),
                    None,
                )
            }
        }
        4 => (
            fmt_uptime(data.uptime_ms),
            "Tempo desde a inicialização".to_string(),
            None,
        ),
        _ => (
            data.process_count.to_string(),
            "Processos em execução".to_string(),
            None,
        ),
    };

    // Descrições curtas para cards de 164px: evitam reticências no fim das
    // palavras sem reduzir o tamanho da fonte até ficar ilegível.
    let small_text = if compact || size == 1 {
        match i {
            0 => "Uso da CPU".to_string(),
            1 => "RAM utilizada".to_string(),
            // O percentual de DISCO é de espaço OCUPADO, não livre.
            2 if data.disk_available => "Espaço utilizado".to_string(),
            3 if data.network_available => "Envio".to_string(),
            4 => "Tempo ligado".to_string(),
            5 => "Em execução".to_string(),
            _ => small_text,
        }
    } else if i == 4 {
        "Tempo ligado".to_string()
    } else {
        small_text
    };

    draw_widget_text(
        buffer_dc,
        &main_text,
        card_rect(typography.value),
        width,
        height,
        rendered_w,
        rendered_h,
        typography.value_px,
        true,
        palette.foreground,
    );
    if show_network_total {
        draw_widget_text(
            buffer_dc,
            "Envio",
            card_rect(typography.subtitle),
            width,
            height,
            rendered_w,
            rendered_h,
            typography.subtitle_px,
            false,
            palette.subtle,
        );
        let detail_label = if size == 3 {
            "Total de dados utilizados"
        } else {
            "Total utilizado"
        };
        draw_widget_text(
            buffer_dc,
            detail_label,
            card_rect(typography.secondary_label),
            width,
            height,
            rendered_w,
            rendered_h,
            typography.secondary_label_px,
            false,
            palette.subtle,
        );
        draw_widget_text(
            buffer_dc,
            &fmt_network_total(data.network_total_bytes),
            card_rect(typography.secondary_value),
            width,
            height,
            rendered_w,
            rendered_h,
            typography.secondary_value_px,
            true,
            palette.foreground,
        );
    } else if visual.subtitles {
        draw_widget_text(
            buffer_dc,
            &small_text,
            card_rect(typography.subtitle),
            width,
            height,
            rendered_w,
            rendered_h,
            typography.subtitle_px,
            false,
            palette.subtle,
        );
    }
    // A barra de atividade indica apenas o ciclo atual de 24 horas.
    if i == 4 && visual.bars && typography.cycle_label.w > 0 {
        draw_widget_text(
            buffer_dc,
            "Ciclo atual de 24 h",
            card_rect(typography.cycle_label),
            width,
            height,
            rendered_w,
            rendered_h,
            12,
            false,
            palette.subtle,
        );
    }
    // Em Processos, substituir a linha sem escala pela lista classificada
    // por RAM: nomes e MiB/GiB reais, atualizados a cada cinco segundos.
    if i == 5 {
        draw_process_overview(
            buffer_dc,
            &data,
            &typography,
            width,
            height,
            rendered_w,
            rendered_h,
            palette,
        );
    } else if visual.graph != 0 && typography.graph.w > 0 && typography.graph.h > 4 {
        draw_history(
            buffer_dc,
            i,
            card_rect(typography.graph),
            palette,
            visual.graph,
        );
    }

    if visual.bars && i != 5 {
        let progress = if let Some(percent) = percentage {
            Some(percent)
        } else if i == 3 && data.network_available {
            // Preenchimento relativo ao pico de envio nas últimas 60 amostras.
            let mut peak = data.up_mib.max(0.01);
            for n in 0..app().history.len {
                peak = peak.max(app().history.get(n).up_mib);
            }
            Some((data.up_mib / peak * 100.0).clamp(0.0, 100.0))
        } else if i == 4 {
            Some(((data.uptime_ms % 86_400_000) as f64 / 86_400_000.0) * 100.0)
        } else {
            None
        };
        if let Some(percent) = progress {
            let baseline = card_rect(typography.progress);
            draw_rounded_fill(buffer_dc, baseline, 6, palette.track);
            let span = baseline.right - baseline.left;
            let filled = ((span as f64) * percent.clamp(0.0, 100.0) / 100.0).round() as i32;
            if filled > 1 {
                let progress_rect = RECT {
                    right: baseline.left + filled,
                    ..baseline
                };
                draw_rounded_fill(buffer_dc, progress_rect, 6, palette.accent);
            }
        }
    }

    // BitBlt 1:1: nenhum redimensionamento raster de texto, gráficos ou ícones.
    SetMapMode(buffer_dc, MM_TEXT);
    BitBlt(dc, 0, 0, rendered_w, rendered_h, buffer_dc, 0, 0, SRCCOPY);
    SelectObject(buffer_dc, old_bitmap);
    DeleteObject(bitmap);
    DeleteDC(buffer_dc);
    EndPaint(hwnd, &ps);
}

pub fn run() {
    unsafe {
        // Per-monitor v2 em Windows 10/11; fallback para o modo legado.
        if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
            SetProcessDPIAware();
        }
        // Reservar também o mutex legado impede executar simultaneamente o
        // RustFetch Native anterior e o Rtime sobre os mesmos cartões.
        // Este nome antigo é mantido somente para compatibilidade entre versões.
        let legacy_name = wide("Local\\RustFetchNativeSingleInstance");
        let legacy_single = CreateMutexW(null(), 0, legacy_name.as_ptr());
        if !legacy_single.is_null() && GetLastError() == ERROR_ALREADY_EXISTS {
            // Outra instância (Rtime ou versão anterior) já está ativa.
            let current = FindWindowW(wide(HOST_CLASS).as_ptr(), null());
            let existing = if current.is_null() {
                FindWindowW(wide(LEGACY_HOST_CLASS).as_ptr(), null())
            } else {
                current
            };
            if !existing.is_null() {
                PostMessageW(existing, EVT_OPEN, 0, 0);
            }
            CloseHandle(legacy_single);
            return;
        }

        let mutex_name = wide("Local\\RtimeSingleInstance");
        let single = CreateMutexW(null(), 0, mutex_name.as_ptr());
        if single.is_null() {
            if !legacy_single.is_null() {
                CloseHandle(legacy_single);
            }
            MessageBoxW(
                null_mut(),
                wide("Não foi possível iniciar o controle de instância.").as_ptr(),
                wide("Rtime").as_ptr(),
                MB_OK | MB_ICONERROR,
            );
            return;
        }
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let existing = FindWindowW(wide(HOST_CLASS).as_ptr(), null());
            if !existing.is_null() {
                PostMessageW(existing, EVT_OPEN, 0, 0);
            }
            CloseHandle(single);
            if !legacy_single.is_null() {
                CloseHandle(legacy_single);
            }
            return;
        }

        let instance = GetModuleHandleW(null());
        APP = Box::into_raw(Box::new(App::new(instance)));
        app().taskbar_created = RegisterWindowMessageW(wide("TaskbarCreated").as_ptr());
        app().font = CreateFontW(
            -17,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        app().label_font = CreateFontW(
            -14,
            0,
            0,
            0,
            FW_NORMAL as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        app().value_font = CreateFontW(
            -30,
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        app().rate_font = CreateFontW(
            -20,
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        app().large_font = CreateFontW(
            -36,
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        app().compact_font = CreateFontW(
            -18,
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        app().settings_heading_font = CreateFontW(
            -27,
            0,
            0,
            0,
            FW_BOLD as i32,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            wide("Segoe UI").as_ptr(),
        );
        let classes = register_class(HOST_CLASS, Some(host_proc), null_mut(), 0)
            && register_class(WIDGET_CLASS, Some(card_proc), null_mut(), CS_DBLCLKS)
            && register_class(
                SETTINGS_CLASS,
                Some(settings_proc),
                (COLOR_WINDOW as usize + 1) as HBRUSH,
                0,
            )
            && register_class(
                SETTINGS_PAGE_CLASS,
                Some(settings_viewport_proc),
                null_mut(),
                0,
            )
            && register_class(
                SETTINGS_CHECK_CLASS,
                Some(settings_check_proc),
                null_mut(),
                0,
            )
            && register_class(
                EDITOR_CLASS,
                Some(editor_proc),
                (COLOR_BTNFACE as usize + 1) as HBRUSH,
                0,
            );
        if !classes {
            MessageBoxW(
                null_mut(),
                wide("Não foi possível registrar as janelas.").as_ptr(),
                wide("Rtime").as_ptr(),
                MB_OK | MB_ICONERROR,
            );
            cleanup(single, legacy_single);
            return;
        }
        app().host = window(
            0,
            HOST_CLASS,
            "Rtime Host",
            WS_OVERLAPPED,
            0,
            0,
            0,
            0,
            null_mut(),
            0,
            null(),
        );
        if app().host.is_null() {
            MessageBoxW(
                null_mut(),
                wide("Falha ao iniciar o aplicativo.").as_ptr(),
                wide("Rtime").as_ptr(),
                MB_OK | MB_ICONERROR,
            );
            cleanup(single, legacy_single);
            return;
        }
        app().monitor_areas = desktop::monitors();
        let screens = app().monitor_areas.clone();
        for i in 0..N {
            let (saved_x, saved_y) = app().config.positions[i];
            let base = dimensions(app().config.card_sizes[i]);
            let saved_rect = Rect::new(saved_x, saved_y, base.0, base.1);
            let (w, h) = physical_size(i, &screens, saved_rect);
            let restored = recover(Rect::new(saved_x, saved_y, w, h), &screens);
            app().config.positions[i] = (restored.left, restored.top);
            let (x, y) = app().config.positions[i];
            app().cards[i] = window(
                WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
                WIDGET_CLASS,
                "Rtime Widget",
                WS_POPUP | WS_CLIPCHILDREN,
                x,
                y,
                w,
                h,
                null_mut(),
                0,
                (i + 1) as *const c_void,
            );
            if app().cards[i].is_null() {
                MessageBoxW(
                    null_mut(),
                    wide("Não foi possível criar os cartões.").as_ptr(),
                    wide("Rtime").as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
                quit();
                cleanup(single, legacy_single);
                return;
            }
        }
        update_appearance();
        tray_add();
        update_visibility();
        start_sampler();
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            // Sem IsDialogMessage, controles filhos de janelas Win32 comuns
            // nao recebem a navegacao por Tab/Shift+Tab como esperado.
            let settings = app().settings;
            let editor = app().editor;
            // F11 pode chegar ao ComboBox/controle com foco, não à janela pai.
            // Bloquear antes de IsDialogMessage/DispatchMessage nas duas janelas.
            if (msg.message == WM_KEYDOWN || msg.message == WM_SYSKEYDOWN)
                && msg.wParam == 0x7A
                && is_key_for_fixed_window(msg.hwnd, settings, editor)
            {
                continue;
            }
            if !settings.is_null()
                && IsWindowVisible(settings) != 0
                && IsDialogMessageW(settings, &msg) != 0
            {
                continue;
            }
            if !editor.is_null()
                && IsWindowVisible(editor) != 0
                && IsDialogMessageW(editor, &msg) != 0
            {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        // Quando sair pelo menu, a rotina quit já terá aguardado o worker.
        if !app().closing {
            quit();
        }
        cleanup(single, legacy_single);
    }
}

unsafe fn cleanup(single: HANDLE, legacy_single: HANDLE) {
    if !APP.is_null() {
        if !app().settings_font.is_null() {
            DeleteObject(app().settings_font);
        }
        if !app().settings_rate_font.is_null() {
            DeleteObject(app().settings_rate_font);
        }
        for font in [
            app().font,
            app().label_font,
            app().value_font,
            app().rate_font,
            app().large_font,
            app().compact_font,
            app().settings_heading_font,
        ] {
            if !font.is_null() {
                DeleteObject(font);
            }
        }
        drop(Box::from_raw(APP));
        APP = null_mut();
    }
    if !single.is_null() {
        CloseHandle(single);
    }
    if !legacy_single.is_null() {
        CloseHandle(legacy_single);
    }
}

#[cfg(test)]
mod responsive_and_clarity_tests {
    use super::*;

    #[test]
    fn restored_window_preserves_default_size() {
        assert_eq!(
            (SETTINGS_DEFAULT_WIDTH, SETTINGS_DEFAULT_HEIGHT),
            (1160, 724)
        );
        let layout = settings_geometry(915, 630, 0);
        assert!(!layout.stacked);
        assert_eq!(layout.scroll_width, 915);
    }

    #[test]
    fn maximized_window_expands_both_panels() {
        let normal = settings_geometry(915, 630, 0);
        let big = settings_geometry(1400, 830, 0);
        assert!(big.left.w > normal.left.w);
        assert!(big.right.w > normal.right.w);
    }

    #[test]
    fn narrow_window_stacks_sections_without_horizontal_overflow() {
        let layout = settings_geometry(510, 410, 0);
        assert!(layout.stacked);
        assert_eq!(layout.scroll_width, 510);
        assert!(layout.right.y > layout.left.bottom());
        assert!(layout.scroll_height > 410);
    }

    #[test]
    fn widgets_reflow_into_three_two_and_one_columns() {
        assert_eq!(settings_geometry(1200, 720, 1).widget_columns, 3);
        assert_eq!(settings_geometry(730, 720, 1).widget_columns, 2);
        assert_eq!(settings_geometry(510, 720, 1).widget_columns, 1);
        let layout = settings_geometry(510, 720, 1);
        assert!(layout.widget_rect(5).bottom() <= layout.scroll_height);
    }
}

#[cfg(test)]
mod ui_hotfix3_tests {
    use super::*;

    #[test]
    fn owner_draw_buttons_are_not_checkboxes() {
        assert!(!is_settings_checkbox("BUTTON", BS_OWNERDRAW as u32));
        assert!(is_settings_checkbox("BUTTON", BS_AUTOCHECKBOX as u32));
        assert!(!is_settings_checkbox("COMBOBOX", BS_AUTOCHECKBOX as u32));
    }

    #[test]
    fn owner_draw_and_checkbox_styles_have_overlapping_bits() {
        assert_ne!((BS_OWNERDRAW as u32) & (BS_AUTOCHECKBOX as u32), 0);
        assert_ne!((BS_OWNERDRAW as u32) & 0x000F, BS_AUTOCHECKBOX as u32);
    }
}

#[cfg(test)]
mod appearance_tests {
    use super::*;

    #[test]
    fn ownerdraw_buttons_include_navigation_and_layout_actions() {
        assert!(owner_button_id(CMD_TAB_BASE));
        assert!(owner_button_id(CMD_TAB_BASE + 3));
        assert!(owner_button_id(CMD_TAB_BASE + 4));
        assert!(owner_button_id(CMD_MONITOR_REFRESH));
        assert!(owner_button_id(CMD_ARRANGE_GRID));
        assert!(owner_button_id(CMD_APPLY_LAYOUT));
        assert!(owner_button_id(CMD_SETTINGS_CLOSE));
        assert!(!owner_button_id(CMD_STYLE));
        assert!(!owner_button_id(CMD_GROUP_MOVE));
    }

    #[test]
    fn history_scale_has_explicit_numeric_type_and_safe_range() {
        assert_eq!(history_scale(0, &[15.0, 22.0, 60.0]), 100.0);
        assert_eq!(history_scale(2, &[12.0, 40.0]), 100.0);
        assert!((history_scale(3, &[1.0, 4.0, 2.0]) - 4.8).abs() < 1e-10);
        assert!((history_scale(5, &[]) - 1.2).abs() < 1e-10);
    }

    #[test]
    fn palette_is_readable_and_accented() {
        let light = palette_for(1, true, 0);
        let dark = palette_for(1, false, 0);
        assert_ne!(light.foreground, dark.foreground);
        assert_ne!(light.background, dark.background);
    }

    #[test]
    fn style_palettes_are_distinct() {
        let fluent = palette_for(0, true, 0);
        let midnight = palette_for(1, false, 0);
        let dashboard = palette_for(2, false, 0);
        assert_ne!(fluent.background, midnight.background);
        assert_ne!(midnight.background, dashboard.background);
    }

    #[test]
    fn history_ring_keeps_60_newest_samples() {
        let mut h = History::default();
        for n in 0..75 {
            let m = Metrics {
                cpu_pct: n as f64,
                ..Metrics::default()
            };
            h.push(m);
        }
        assert_eq!(h.len, 60);
        assert_eq!(h.get(0).cpu_pct, 15.0);
        assert_eq!(h.get(59).cpu_pct, 74.0);
    }

    #[test]
    fn each_accent_has_distinct_color() {
        let colors: Vec<_> = (1..ACCENTS.len() as u32)
            .map(|index| palette_for(2, false, index).accent)
            .collect();
        let mut unique = colors.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(unique.len(), colors.len());
    }

    #[test]
    fn individual_style_overrides_only_one_card() {
        let mut cfg = Config::default();
        cfg.widget_styles[0] = 1; // Fluent.
        cfg.widget_accents[0] = 3; // Violet.
        cfg.widget_graphs[0] = 2; // Area.
        cfg.widget_opacities[0] = 3; // 86%.
        cfg.widget_bars[0] = 2; // Ocultar.
        let cpu = visual_for(&cfg, 0, false);
        let ram = visual_for(&cfg, 1, false);
        assert_eq!(cpu.style, 0);
        assert_eq!(ram.style, 1);
        assert_eq!(cpu.graph, 2);
        assert_eq!(ram.graph, 1);
        assert_eq!(cpu.alpha, 219);
        assert!(!cpu.bars);
        assert!(ram.bars);
    }

    #[test]
    fn graph_can_be_disabled_for_a_single_widget() {
        let mut cfg = Config::default();
        cfg.widget_graphs[4] = 4;
        assert_eq!(visual_for(&cfg, 4, false).graph, 0);
        assert_eq!(visual_for(&cfg, 2, false).graph, 1);
    }

    #[test]
    fn profiles_preserve_individual_customization() {
        let mut cfg = Config::default();
        cfg.widget_styles[0] = 1;
        apply_visual_profile(&mut cfg, 3);
        assert_eq!(cfg.style, 2);
        assert!(cfg.show_graphs);
        assert_eq!(cfg.widget_styles[0], 1);
        apply_visual_profile(&mut cfg, 1);
        assert!(!cfg.show_graphs);
        assert_eq!(cfg.widget_styles[0], 1);
    }

    #[test]
    fn blend_color_works_with_rgb_channels() {
        let a = color(0, 0, 0);
        let b = color(100, 200, 50);
        assert_eq!(blend_color(a, b, 50), color(50, 100, 25));
    }
}

#[cfg(test)]
mod monitoring_v038_tests {
    use super::*;
    #[test]
    fn monitoring_ids_are_distinct_and_outside_editor_range() {
        let ids = [
            CMD_MONITOR_DISK,
            CMD_MONITOR_ADAPTER,
            CMD_MONITOR_INTERVAL,
            CMD_MONITOR_REFRESH,
        ];
        assert_eq!(ids.len(), 4);
        assert!(ids.iter().all(|&id| id > CMD_SETTINGS_CLOSE));
        assert!(ids.windows(2).all(|pair| pair[0] != pair[1]));
    }
    #[test]
    fn defaults_keep_original_system_sources() {
        let config = Config::default();
        assert_eq!(config.monitor_disk, 0);
        assert_eq!(config.monitor_adapter, 0);
        assert_eq!(config.update_interval, 1);
    }
}

#[cfg(test)]
mod rtime_identity_tests {
    use super::*;

    #[test]
    fn renamed_win32_classes_do_not_overlap_previous_ones() {
        assert_eq!(HOST_CLASS, "Rtime.HiddenHost");
        assert_eq!(WIDGET_CLASS, "Rtime.Widget");
        assert_eq!(SETTINGS_CLASS, "Rtime.Settings");
        assert_ne!(HOST_CLASS, LEGACY_HOST_CLASS);
    }
}

#[cfg(test)]
mod fixed_window_tests {
    use super::*;

    #[test]
    fn both_dialogs_have_no_maximize_or_resize_border() {
        assert_eq!(FIXED_DIALOG_STYLE & WS_MAXIMIZEBOX, 0);
        assert_eq!(FIXED_DIALOG_STYLE & WS_THICKFRAME, 0);
        assert_ne!(FIXED_DIALOG_STYLE & WS_MINIMIZEBOX, 0);
        assert_ne!(FIXED_DIALOG_STYLE & WS_SYSMENU, 0);
    }

    #[test]
    fn maximize_and_size_commands_are_blocked_but_restore_is_allowed() {
        assert!(forbidden_resize_command(SC_MAXIMIZE as usize));
        assert!(forbidden_resize_command(SC_SIZE as usize));
        assert!(!forbidden_resize_command(SC_MINIMIZE as usize));
        assert!(!forbidden_resize_command(SC_RESTORE as usize));
    }

    #[test]
    fn corrupted_minimized_geometry_is_detected_for_both_dialogs() {
        let tiny = RECT {
            left: 20,
            top: 20,
            right: 257,
            bottom: 55,
        };
        let settings = RECT {
            left: 20,
            top: 20,
            right: 1180,
            bottom: 744,
        };
        let editor = RECT {
            left: 20,
            top: 20,
            right: 1210,
            bottom: 875,
        };
        assert!(dialog_geometry_incorrect(
            tiny,
            SETTINGS_DEFAULT_WIDTH,
            SETTINGS_DEFAULT_HEIGHT
        ));
        assert!(dialog_geometry_incorrect(
            tiny,
            EDITOR_DEFAULT_WIDTH,
            EDITOR_DEFAULT_HEIGHT
        ));
        assert!(!dialog_geometry_incorrect(
            settings,
            SETTINGS_DEFAULT_WIDTH,
            SETTINGS_DEFAULT_HEIGHT
        ));
        assert!(!dialog_geometry_incorrect(
            editor,
            EDITOR_DEFAULT_WIDTH,
            EDITOR_DEFAULT_HEIGHT
        ));
    }

    #[test]
    fn minimize_hides_auxiliary_windows_without_blocking_restore() {
        assert!(minimize_auxiliary_command(SC_MINIMIZE as usize));
        // WM_SYSCOMMAND reserva os quatro bits menos significativos.
        assert!(minimize_auxiliary_command((SC_MINIMIZE as usize) | 0x0003));
        assert!(!minimize_auxiliary_command(SC_RESTORE as usize));
        assert!(!minimize_auxiliary_command(SC_CLOSE as usize));
        assert!(!minimize_auxiliary_command(SC_MAXIMIZE as usize));
    }
}
