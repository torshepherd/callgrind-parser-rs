use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend, widgets::Paragraph};

#[test]
fn tui_renders_and_constructs_input_events_without_a_real_terminal() {
    let mut terminal = Terminal::new(TestBackend::new(20, 2)).unwrap();
    terminal
        .draw(|frame| frame.render_widget(Paragraph::new("textgrind"), frame.area()))
        .unwrap();
    terminal
        .backend()
        .assert_buffer_lines(["textgrind           ", "                    "]);
    let quit = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
    assert_eq!(quit.code, KeyCode::Char('q'));
}
