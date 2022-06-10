#[macro_export]
macro_rules! prompt {
    ($output:expr,$args:tt) => {
        prompt!($output, $args, callback = |&_, _, _| {})
    };
    ($output:expr,$args:tt, callback = $callback:expr) => {{
        let output: &mut Output = $output;
        let mut input = String::with_capacity(32);
        loop {
            output.status_message.set_message(format!($args, input));
            output.refresh_screen()?;
            let key_event = Screen.read_key()?;
            match key_event {
                KeyEvent {
                    code: KeyCode::Enter,
                    modifiers: crossterm::event::KeyModifiers::NONE,
                } => {
                    if !input.is_empty() {
                        output.status_message.set_message(String::new());
                        $callback(output, &input, KeyCode::Enter);
                        break;
                    }
                }
                KeyEvent {
                    code: KeyCode::Esc, ..
                } => {
                    output.status_message.set_message(String::new());
                    input.clear();
                    $callback(output, &input, KeyCode::Esc);
                    break;
                }
                KeyEvent {
                    code: KeyCode::Backspace | KeyCode::Delete,
                    modifiers: crossterm::event::KeyModifiers::NONE,
                } => {
                    input.pop();
                }
                KeyEvent {
                    code: code @ (KeyCode::Char(..) | KeyCode::Tab),
                    modifiers: crossterm::event::KeyModifiers::NONE | crossterm::event::KeyModifiers::SHIFT,
                } => {
                    input.push(match code {
                        KeyCode::Tab => '\t',
                        KeyCode::Char(ch) => ch,
                        _ => unreachable!(),
                    });
                }
                _ => {}
            }
            $callback(output, &input, key_event.code);
        }
        if input.is_empty() {
            None
        } else {
            Some(input)
        }
    }};
}