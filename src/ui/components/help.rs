use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};

use crate::app::{AppState, ClickTarget};
use crate::i18n::Language;

/// Renders the help overlay modal with rounded borders and comprehensive keybindings.
pub fn render(frame: &mut Frame, app: &AppState, area: Rect) {
    let theme = app.palette();
    let lang = app.language;
    let popup_area = centered_rect(68, 84, area);
    {
        let mut hit_map = app.hit_map.borrow_mut();
        hit_map.push(area, ClickTarget::DismissModal);
        hit_map.push(popup_area, ClickTarget::ModalBody);
    }

    // Clear background behind popup
    frame.render_widget(Clear, popup_area);

    let modal_title = match lang {
        Language::En => " Help & Keybindings ",
        Language::Es => " Ayuda y Atajos ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(modal_title)
        .title_style(
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(theme.accent))
        .style(Style::default().bg(theme.bg_dark));

    let inner = block.inner(popup_area);
    frame.render_widget(block, popup_area);

    let key_style = Style::default().fg(theme.warning);
    let title_style = Style::default()
        .fg(theme.accent)
        .add_modifier(Modifier::BOLD);

    let help_text = match lang {
        Language::En => vec![
            Line::from(vec![Span::styled("Navigation & Layout", title_style)]),
            Line::from(vec![
                Span::styled("  Tab / Shift+Tab   ", key_style),
                Span::raw("Cycle panels (Actions ↔ Console, or Devices ↔ ADB Controls)"),
            ]),
            Line::from(vec![
                Span::styled("  ←/→/↑/↓           ", key_style),
                Span::raw("Navigate action/control grid cards, devices, or scroll logs"),
            ]),
            Line::from(vec![
                Span::styled("  1 / 2 / 3         ", key_style),
                Span::raw("Toggle Status (1), Build Output (2), or Live Logcat (3) pane"),
            ]),
            Line::from(vec![
                Span::styled("  m / l             ", key_style),
                Span::raw("Cycle console presets (Status / Build / Logcat / Parity / Triple)"),
            ]),
            Line::from(vec![
                Span::styled("  d                 ", key_style),
                Span::raw("Open / close separate Devices & ADB Controls view"),
            ]),
            Line::from(vec![
                Span::styled("  s / ,             ", key_style),
                Span::raw("Open / close full Settings view (Themes, Language, ADB, About)"),
            ]),
            Line::from(vec![
                Span::styled("  w                 ", key_style),
                Span::raw("Open ADB Wireless Pair & Connect modal"),
            ]),
            Line::from(vec![
                Span::styled("  v                 ", key_style),
                Span::raw("Cycle active Android build variant (debug/release/...)"),
            ]),
            Line::from(vec![
                Span::styled("  t                 ", key_style),
                Span::raw("Cycle KMP target (Android / JVM / WasmJS / JS)"),
            ]),
            Line::from(vec![
                Span::styled("  p                 ", key_style),
                Span::raw("Switch project (siblings and nearby Gradle projects)"),
            ]),
            Line::from(vec![
                Span::styled("  T / L / O         ", key_style),
                Span::raw("Cycle theme (Shift+T), toggle language (Shift+L), or onboarding (Shift+O)"),
            ]),
            Line::from(vec![
                Span::styled("  y / Y             ", key_style),
                Span::raw("Copy logcat (y) or build output (Y) to the clipboard"),
            ]),
            Line::from(vec![
                Span::styled("  f                 ", key_style),
                Span::raw("Toggle logcat between this app and all device logs"),
            ]),
            Line::raw(""),
            Line::from(vec![Span::styled(
                "Actions & ADB Device Controls",
                title_style,
            )]),
            Line::from(vec![
                Span::styled("  Enter / Space     ", key_style),
                Span::raw("Execute highlighted Action or Device Control / Select device"),
            ]),
            Line::from(vec![
                Span::styled("  b                 ", key_style),
                Span::raw("Build/install debug APK (Android) or compile active KMP target"),
            ]),
            Line::from(vec![
                Span::styled("  r                 ", key_style),
                Span::raw("Relaunch activity (Android) or run / dev server for KMP target"),
            ]),
            Line::from(vec![
                Span::styled("  k / u             ", key_style),
                Span::raw("Force stop (k) or uninstall (u) Android app"),
            ]),
            Line::from(vec![
                Span::styled("  g                 ", key_style),
                Span::raw("Stop all running Gradle daemons (`./gradlew --stop`)"),
            ]),
            Line::raw(""),
            Line::from(vec![Span::styled("Search & Process Control", title_style)]),
            Line::from(vec![
                Span::styled("  /                 ", key_style),
                Span::raw("Search / filter logs in console"),
            ]),
            Line::from(vec![
                Span::styled("  Esc / Ctrl+X      ", key_style),
                Span::raw("Dismiss modal, close Devices view, OR abort running task"),
            ]),
            Line::from(vec![
                Span::styled("  ?                 ", key_style),
                Span::raw("Toggle this help popup"),
            ]),
            Line::from(vec![
                Span::styled("  q / Ctrl+C        ", key_style),
                Span::raw("Quit OverridePulse"),
            ]),
        ],
        Language::Es => vec![
            Line::from(vec![Span::styled("Navegación y Diseño", title_style)]),
            Line::from(vec![
                Span::styled("  Tab / Shift+Tab   ", key_style),
                Span::raw("Alternar paneles (Acciones ↔ Consola, o Equipos ↔ Controles ADB)"),
            ]),
            Line::from(vec![
                Span::styled("  ←/→/↑/↓           ", key_style),
                Span::raw("Moverse por tarjetas en cuadrícula, equipos o scroll de logs"),
            ]),
            Line::from(vec![
                Span::styled("  1 / 2 / 3         ", key_style),
                Span::raw("Mostrar u ocultar Estado (1), Compilación (2) o Logcat (3)"),
            ]),
            Line::from(vec![
                Span::styled("  m / l             ", key_style),
                Span::raw("Rotar presets de consola"),
            ]),
            Line::from(vec![
                Span::styled("  d                 ", key_style),
                Span::raw("Abrir / cerrar vista dedicada de Dispositivos y Controles ADB"),
            ]),
            Line::from(vec![
                Span::styled("  s / ,             ", key_style),
                Span::raw("Abrir / cerrar vista completa de Ajustes (Tema, Idioma, ADB, Info)"),
            ]),
            Line::from(vec![
                Span::styled("  w                 ", key_style),
                Span::raw("Abrir modal de ADB inalámbrico (Pair & Connect)"),
            ]),
            Line::from(vec![
                Span::styled("  v                 ", key_style),
                Span::raw("Cambiar variante Android (debug/release/...)"),
            ]),
            Line::from(vec![
                Span::styled("  t                 ", key_style),
                Span::raw("Cambiar target KMP (Android / JVM / WasmJS / JS)"),
            ]),
            Line::from(vec![
                Span::styled("  p                 ", key_style),
                Span::raw("Cambiar de proyecto"),
            ]),
            Line::from(vec![
                Span::styled("  T / L / O         ", key_style),
                Span::raw("Tema (Shift+T), idioma (Shift+L) o reabrir onboarding (Shift+O)"),
            ]),
            Line::from(vec![
                Span::styled("  y / Y             ", key_style),
                Span::raw("Copiar logcat (y) o salida de compilación (Y)"),
            ]),
            Line::from(vec![
                Span::styled("  f                 ", key_style),
                Span::raw("Alternar filtro de logcat entre tu app y todo el dispositivo"),
            ]),
            Line::raw(""),
            Line::from(vec![Span::styled("Acciones y Procesos", title_style)]),
            Line::from(vec![
                Span::styled("  Enter / Space     ", key_style),
                Span::raw("Ejecutar tarjeta seleccionada o elegir dispositivo"),
            ]),
            Line::from(vec![
                Span::styled("  b                 ", key_style),
                Span::raw("Compilar/instalar APK (Android) o compilar target KMP"),
            ]),
            Line::from(vec![
                Span::styled("  r                 ", key_style),
                Span::raw("Relanzar app (Android) o ejecutar app / dev server KMP"),
            ]),
            Line::from(vec![
                Span::styled("  k / u             ", key_style),
                Span::raw("Detener (k) o desinstalar (u) app Android"),
            ]),
            Line::from(vec![
                Span::styled("  g                 ", key_style),
                Span::raw("Detener daemons de Gradle (`./gradlew --stop`)"),
            ]),
            Line::raw(""),
            Line::from(vec![Span::styled("Búsqueda y Control", title_style)]),
            Line::from(vec![
                Span::styled("  /                 ", key_style),
                Span::raw("Buscar / filtrar líneas en consola"),
            ]),
            Line::from(vec![
                Span::styled("  Esc / Ctrl+X      ", key_style),
                Span::raw("Cerrar modal/vista de equipos o cancelar tarea en curso"),
            ]),
            Line::from(vec![
                Span::styled("  ?                 ", key_style),
                Span::raw("Mostrar u ocultar esta ayuda"),
            ]),
            Line::from(vec![
                Span::styled("  q / Ctrl+C        ", key_style),
                Span::raw("Salir de OverridePulse"),
            ]),
        ],
    };

    let paragraph = Paragraph::new(help_text);
    frame.render_widget(paragraph, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
