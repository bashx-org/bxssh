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
                Ok(Event::Key(KeyEvent { code, .. })) => {
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
                        KeyCode::Esc => b"\x1b".to_vec(),
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
        io::stdout().write_all(data)
            .context("Failed to write to stdout")?;
        io::stdout().flush()
            .context("Failed to flush stdout")?;
        Ok(())
    }
    
    fn should_continue(&self) -> bool {
        self.should_continue
    }
    
    fn initialize(&mut self) -> Result<()> {
        enable_raw_mode().context("Failed to enable raw mode")?;
        self.raw_mode_enabled = true;
        
        println!("🔗 Connected to remote server. Use Ctrl+C to exit.\r");
        io::stdout().flush()?;
        
        Ok(())
    }
    
    fn cleanup(&mut self) -> Result<()> {
        if self.raw_mode_enabled {
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