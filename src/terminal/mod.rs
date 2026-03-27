use std::io::{Read, Write};
use std::sync::{Arc, Mutex};

pub struct TerminalState {
    pub lines: Vec<String>,
    pub cursor_row: usize,
    pub cursor_col: usize,
    pub cols: u16,
    pub rows: u16,
}

impl Default for TerminalState {
    fn default() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
            cols: 120,
            rows: 40,
        }
    }
}

impl TerminalState {
    pub fn snapshot_lines(&self, max: usize) -> Vec<String> {
        self.lines.iter().rev().take(max).cloned().collect::<Vec<_>>().into_iter().rev().collect()
    }
}

pub struct TerminalPerformer {
    pub state: Arc<Mutex<TerminalState>>,
}

impl TerminalPerformer {
    pub fn new(state: Arc<Mutex<TerminalState>>) -> Self {
        Self { state }
    }
}

impl vte::Perform for TerminalPerformer {
    fn print(&mut self, c: char) {
        let mut state = self.state.lock().unwrap();
        let row = state.cursor_row;
        let col = state.cursor_col;
        while state.lines.len() <= row {
            state.lines.push(String::new());
        }
        let line = &mut state.lines[row];
        while line.chars().count() <= col {
            line.push(' ');
        }
        let mut chars: Vec<char> = line.chars().collect();
        if col < chars.len() {
            chars[col] = c;
        } else {
            chars.push(c);
        }
        *line = chars.into_iter().collect();
        state.cursor_col += 1;
    }

    fn execute(&mut self, byte: u8) {
        let mut state = self.state.lock().unwrap();
        match byte {
            0x0a => {
                state.cursor_row += 1;
                state.cursor_col = 0;
                if state.lines.len() <= state.cursor_row {
                    state.lines.push(String::new());
                }
            }
            0x0d => { state.cursor_col = 0; }
            0x08 => { state.cursor_col = state.cursor_col.saturating_sub(1); }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &vte::Params, _intermediates: &[u8], _ignore: bool, _action: char) {}
    fn put(&mut self, _byte: u8) {}
    fn unhook(&mut self) {}
    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}
    fn csi_dispatch(&mut self, params: &vte::Params, _intermediates: &[u8], _ignore: bool, action: char) {
        let mut state = self.state.lock().unwrap();
        let p: Vec<u16> = params.iter().map(|s| s.first().copied().unwrap_or(0)).collect();

        match action {
            'A' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                state.cursor_row = state.cursor_row.saturating_sub(n);
            }
            'B' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                state.cursor_row += n;
            }
            'C' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                state.cursor_col += n;
            }
            'D' => {
                let n = p.first().copied().unwrap_or(1).max(1) as usize;
                state.cursor_col = state.cursor_col.saturating_sub(n);
            }
            'H' | 'f' => {
                let row = p.first().copied().unwrap_or(1).max(1) as usize - 1;
                let col = p.get(1).copied().unwrap_or(1).max(1) as usize - 1;
                state.cursor_row = row;
                state.cursor_col = col;
                while state.lines.len() <= state.cursor_row {
                    state.lines.push(String::new());
                }
            }
            'K' => {
                let mode = p.first().copied().unwrap_or(0);
                let row = state.cursor_row;
                let col = state.cursor_col;
                if row < state.lines.len() {
                    match mode {
                        0 => { state.lines[row].truncate(col); }
                        1 => {
                            let rest = if col < state.lines[row].len() {
                                state.lines[row][col..].to_string()
                            } else {
                                String::new()
                            };
                            state.lines[row] = " ".repeat(col) + &rest;
                        }
                        2 => { state.lines[row].clear(); }
                        _ => {}
                    }
                }
            }
            'J' => {
                let mode = p.first().copied().unwrap_or(0);
                let row = state.cursor_row;
                let col = state.cursor_col;
                match mode {
                    0 => {
                        if row < state.lines.len() {
                            state.lines[row].truncate(col);
                            state.lines.truncate(row + 1);
                        }
                    }
                    2 | 3 => {
                        state.lines.clear();
                        state.lines.push(String::new());
                        state.cursor_row = 0;
                        state.cursor_col = 0;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }
    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

pub fn spawn_pty_shell(
    cwd: &std::path::Path,
    cols: u16,
    rows: u16,
) -> Result<(Box<dyn Read + Send>, Box<dyn Write + Send>, Box<dyn portable_pty::Child + Send>), Box<dyn std::error::Error>> {
    use portable_pty::{CommandBuilder, PtySize, native_pty_system};

    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })?;

    let mut cmd = CommandBuilder::new_default_prog();
    cmd.cwd(cwd);

    let child = pair.slave.spawn_command(cmd)?;
    let reader = pair.master.try_clone_reader()?;
    let writer = pair.master.take_writer()?;

    Ok((reader, writer, child))
}
