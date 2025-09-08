use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use log::{error, info};
use std::io::{self, Write};
use std::time::Duration;

use crate::terminal::TerminalIO;

/// CLI-specific terminal I/O implementation
pub struct CliTerminalIO {
    should_continue: bool,
    raw_mode_enabled: bool,
}

impl CliTerminalIO {
    pub fn new() -> Self {
        Self {
            should_continue: true,
            raw_mode_enabled: false,
        }
    }
}

impl TerminalIO for CliTerminalIO {
    fn read_input(&mut self) -> Result<Option<Vec<u8>>> {
        use log::debug;
        
        // Non-blocking input polling
        if let Ok(true) = event::poll(Duration::from_millis(10)) {
            match event::read() {
                Ok(Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: event::KeyModifiers::CONTROL,
                    ..
                })) => {
                    info!("Ctrl+C pressed, exiting shell");
                    self.should_continue = false;
                    Ok(None)
                }
                Ok(Event::Key(KeyEvent { code, modifiers, .. })) => {
                    // Handle Ctrl+key combinations that vim uses
                    if modifiers.contains(event::KeyModifiers::CONTROL) {
                        let ctrl_bytes = match code {
                            KeyCode::Char('a') => b"\x01".to_vec(), // Ctrl+A
                            KeyCode::Char('b') => b"\x02".to_vec(), // Ctrl+B  
                            KeyCode::Char('d') => b"\x04".to_vec(), // Ctrl+D
                            KeyCode::Char('e') => b"\x05".to_vec(), // Ctrl+E
                            KeyCode::Char('f') => b"\x06".to_vec(), // Ctrl+F
                            KeyCode::Char('g') => b"\x07".to_vec(), // Ctrl+G
                            KeyCode::Char('h') => b"\x08".to_vec(), // Ctrl+H (backspace in vim)
                            KeyCode::Char('i') => b"\x09".to_vec(), // Ctrl+I (tab)
                            KeyCode::Char('j') => b"\x0a".to_vec(), // Ctrl+J
                            KeyCode::Char('k') => b"\x0b".to_vec(), // Ctrl+K
                            KeyCode::Char('l') => b"\x0c".to_vec(), // Ctrl+L
                            KeyCode::Char('m') => b"\x0d".to_vec(), // Ctrl+M (enter)
                            KeyCode::Char('n') => b"\x0e".to_vec(), // Ctrl+N
                            KeyCode::Char('o') => b"\x0f".to_vec(), // Ctrl+O
                            KeyCode::Char('p') => b"\x10".to_vec(), // Ctrl+P
                            KeyCode::Char('q') => b"\x11".to_vec(), // Ctrl+Q
                            KeyCode::Char('r') => b"\x12".to_vec(), // Ctrl+R
                            KeyCode::Char('s') => b"\x13".to_vec(), // Ctrl+S
                            KeyCode::Char('t') => b"\x14".to_vec(), // Ctrl+T
                            KeyCode::Char('u') => b"\x15".to_vec(), // Ctrl+U
                            KeyCode::Char('v') => b"\x16".to_vec(), // Ctrl+V
                            KeyCode::Char('w') => b"\x17".to_vec(), // Ctrl+W
                            KeyCode::Char('x') => b"\x18".to_vec(), // Ctrl+X
                            KeyCode::Char('y') => b"\x19".to_vec(), // Ctrl+Y
                            KeyCode::Char('z') => b"\x1a".to_vec(), // Ctrl+Z
                            _ => return Ok(None), // Ignore other Ctrl combinations
                        };
                        debug!("Ctrl+{:?} pressed -> bytes: {:?}", code, String::from_utf8_lossy(&ctrl_bytes));
                        return Ok(Some(ctrl_bytes));
                    }
                    let input_bytes = match code {
                        KeyCode::Enter => b"\r".to_vec(),
                        KeyCode::Tab => b"\t".to_vec(), 
                        KeyCode::Backspace => b"\x7f".to_vec(),
                        KeyCode::Delete => b"\x1b[3~".to_vec(),
                        KeyCode::Char(c) => c.to_string().into_bytes(),
                        KeyCode::Up => b"\x1b[A".to_vec(),
                        KeyCode::Down => b"\x1b[B".to_vec(), 
                        KeyCode::Right => b"\x1b[C".to_vec(),
                        KeyCode::Left => b"\x1b[D".to_vec(),
                        KeyCode::Home => b"\x1b[H".to_vec(),
                        KeyCode::End => b"\x1b[F".to_vec(),
                        KeyCode::PageUp => b"\x1b[5~".to_vec(),
                        KeyCode::PageDown => b"\x1b[6~".to_vec(),
                        KeyCode::Insert => b"\x1b[2~".to_vec(),
                        KeyCode::Esc => b"\x1b".to_vec(),
                        // Function keys that vim uses
                        KeyCode::F(1) => b"\x1b[11~".to_vec(),
                        KeyCode::F(2) => b"\x1b[12~".to_vec(),
                        KeyCode::F(3) => b"\x1b[13~".to_vec(),
                        KeyCode::F(4) => b"\x1b[14~".to_vec(),
                        KeyCode::F(5) => b"\x1b[15~".to_vec(),
                        KeyCode::F(6) => b"\x1b[17~".to_vec(),
                        KeyCode::F(7) => b"\x1b[18~".to_vec(),
                        KeyCode::F(8) => b"\x1b[19~".to_vec(),
                        KeyCode::F(9) => b"\x1b[20~".to_vec(),
                        KeyCode::F(10) => b"\x1b[21~".to_vec(),
                        KeyCode::F(11) => b"\x1b[23~".to_vec(),
                        KeyCode::F(12) => b"\x1b[24~".to_vec(),
                        _ => {
                            debug!("Ignoring key: {:?}", code);
                            return Ok(None);
                        }
                    };
                    debug!("Key pressed: {:?} -> bytes: {:?}", code, String::from_utf8_lossy(&input_bytes));
                    Ok(Some(input_bytes))
                }
                Ok(event) => {
                    debug!("Non-key event: {:?}", event);
                    Ok(None)
                }
                Err(e) => {
                    error!("Error reading terminal input: {}", e);
                    self.should_continue = false;
                    Ok(None)
                }
            }
        } else {
            Ok(None)
        }
    }
    
    fn write_output(&mut self, data: &[u8]) -> Result<()> {
        use log::debug;
        
        // Enhanced debugging for vim escape sequences
        let data_str = String::from_utf8_lossy(data);
        
        // Check for problematic patterns that vim might send
        if data_str.contains('\x1b') { // ESC character
            let esc_count = data_str.matches('\x1b').count();
            if esc_count > 5 {
                debug!("High volume of escape sequences ({} ESC chars): {:?}", 
                    esc_count,
                    data_str.chars().take(100).collect::<String>()
                        .replace('\x1b', "\\e")
                        .replace('\r', "\\r")
                        .replace('\n', "\\n"));
            }
        }
        
        // Check for vim-specific sequences that might cause display issues
        if data_str.contains("\x1b[?1049h") || data_str.contains("\x1b[?1049l") {
            debug!("Vim alternate screen buffer command detected");
        }
        
        if data_str.contains("\x1b[?25l") || data_str.contains("\x1b[?25h") {
            debug!("Vim cursor visibility command detected");
        }
        
        // Filter out potentially problematic sequences for terminals that don't handle them well
        let filtered_data = if data_str.contains("\x1b[>4;2m") || data_str.contains("\x1b[<") {
            debug!("Filtering potentially problematic mouse/terminal sequences");
            // Remove SGR mouse sequences and other problematic codes
            data_str.replace("\x1b[>4;2m", "")
                   .replace("\x1b[<", "")
                   .into_bytes()
        } else {
            data.to_vec()
        };
        
        // Write data directly to stdout - let the terminal handle escape sequences
        io::stdout().write_all(&filtered_data)
            .context("Failed to write to stdout")?;
        io::stdout().flush()
            .context("Failed to flush stdout")?;
        Ok(())
    }
    
    fn should_continue(&self) -> bool {
        self.should_continue
    }
    
    fn initialize(&mut self) -> Result<()> {
        use crossterm::{execute, cursor, terminal};
        
        enable_raw_mode().context("Failed to enable raw mode")?;
        self.raw_mode_enabled = true;
        
        // Set up terminal for vim compatibility
        execute!(
            io::stdout(),
            terminal::Clear(terminal::ClearType::All),
            cursor::MoveTo(0, 0),
        )?;
        
        println!("🔗 Connected to remote server. Use Ctrl+C to exit.\r");
        io::stdout().flush()?;
        
        // Small delay to let terminal settle
        std::thread::sleep(std::time::Duration::from_millis(100));
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<()> {
        use crossterm::{execute, cursor, terminal};
        
        if self.raw_mode_enabled {
            // Reset terminal state before disabling raw mode
            let _ = execute!(
                io::stdout(),
                cursor::Show,
                terminal::Clear(terminal::ClearType::FromCursorDown)
            );
            
            disable_raw_mode().context("Failed to disable raw mode")?;
            self.raw_mode_enabled = false;
        }
        
        println!("\n🔌 Disconnected from remote server.");
        Ok(())
    }
}

impl Drop for CliTerminalIO {
    fn drop(&mut self) {
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_cli_terminal_creation() {
        let terminal = CliTerminalIO::new();
        assert!(terminal.should_continue());
        assert!(!terminal.raw_mode_enabled);
    }
    
    #[test]
    fn test_write_output() {
        let mut terminal = CliTerminalIO::new();
        let result = terminal.write_output(b"test output");
        assert!(result.is_ok());
    }
}