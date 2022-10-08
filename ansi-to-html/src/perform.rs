use crate::ansi::{Color, C0};
use crate::renderer::Renderer;
use vte::{Params, Perform};

impl Perform for Renderer {
    fn print(&mut self, c: char) {
        self.print(c);
    }

    #[inline]
    fn execute(&mut self, byte: u8) {
        match byte {
            0 | 1 => {} // wat
            C0::HT => self.put_tab(),
            C0::BS => self.backspace(),
            C0::CR => self.carriage_return(),
            C0::LF | C0::VT | C0::FF => self.linefeed(),
            C0::BEL => {}
            //C0::SUB => self.substitute(),
            //C0::SI => self.set_active_charset(CharsetIndex::G0),
            //C0::SO => self.set_active_charset(CharsetIndex::G1),
            _ => {
                println!("Unhandled execute byte={:02x}", byte)
            }
        }
    }

    fn csi_dispatch(
        &mut self,
        params: &Params,
        _intermediates: &[u8],
        _ignore: bool,
        action: char,
    ) {
        if action == 'm' {
            for p in params.iter() {
                match p {
                    &[0] => {
                        self.bold = false;
                        self.italic = false;
                        self.underline = false;
                        self.dim = false;
                        self.foreground = Color::bright_white();
                        self.background = Color::black();
                    }
                    &[1] => self.bold = true,
                    &[2] => self.dim = true,
                    &[3] => self.italic = true,
                    &[4] => self.underline = true,
                    &[7] => {
                        // Reverse video or invert. Inconsistent emulation.
                    }
                    &[22] => {
                        self.bold = false;
                        self.dim = false;
                        // Set intensity to normal
                    }
                    &[30] => self.foreground = Color::black(),
                    &[31] => self.foreground = Color::red(),
                    &[32] => self.foreground = Color::green(),
                    &[33] => self.foreground = Color::yellow(),
                    &[34] => self.foreground = Color::blue(),
                    &[35] => self.foreground = Color::magenta(),
                    &[36] => self.foreground = Color::cyan(),
                    &[37] => self.foreground = Color::white(),
                    // 8-bit foreground color
                    &[38] => {
                        if let Some([38, 5, code]) = params.get::<3>() {
                            if let Some(color) = Color::parse_8bit(code) {
                                self.foreground = color;
                            }
                        } else if let Some([38, 5, fg_code, 48, 5, bg_code]) = params.get::<6>() {
                            if let Some(color) = Color::parse_8bit(fg_code) {
                                self.foreground = color;
                            }
                            if let Some(color) = Color::parse_8bit(bg_code) {
                                self.background = color;
                            }
                        } else {
                            println!("Uhandled m: {:?}", params);
                        }
                        break;
                    }
                    &[39] => self.foreground = Color::bright_white(), // Default foreground color
                    &[40] => self.background = Color::black(),
                    &[41] => self.background = Color::red(),
                    &[42] => self.background = Color::green(),
                    &[43] => self.background = Color::yellow(),
                    &[44] => self.background = Color::blue(),
                    &[45] => self.background = Color::magenta(),
                    &[46] => self.background = Color::cyan(),
                    &[47] => self.background = Color::white(),
                    &[48] => {
                        if let Some([48, 5, code]) = params.get::<3>() {
                            if let Some(color) = Color::parse_8bit(code) {
                                self.background = color;
                            }
                        } else if let Some([38, 5, fg_code, 48, 5, bg_code]) = params.get::<6>() {
                            if let Some(color) = Color::parse_8bit(fg_code) {
                                self.foreground = color;
                            }
                            if let Some(color) = Color::parse_8bit(bg_code) {
                                self.background = color;
                            }
                        } else {
                            println!("Uhandled m: {:?}", params);
                        }
                        break;
                    }
                    &[49] => self.background = Color::black(), // Default foreground color
                    &[90] => self.foreground = Color::bright_black(),
                    &[91] => self.foreground = Color::bright_red(),
                    &[92] => self.foreground = Color::bright_green(),
                    &[93] => self.foreground = Color::bright_yellow(),
                    &[94] => self.foreground = Color::bright_blue(),
                    &[95] => self.foreground = Color::bright_magenta(),
                    &[96] => self.foreground = Color::bright_cyan(),
                    &[97] => self.foreground = Color::bright_white(),
                    _ => {
                        println!("Unhandled m: {:?}", params)
                    }
                }
            }
        } else if action == 'H' {
            let mut it = params.iter();
            if let (Some(&[row]), Some(&[col])) = (it.next(), it.next()) {
                self.handle_move(row, col);
            }
        } else if action == 'A' {
            if let Some(&[cells]) = params.iter().next() {
                self.move_up_by(cells);
            }
        } else if action == 'B' {
            if let Some(&[cells]) = params.iter().next() {
                self.move_down_by(cells);
            }
        } else if action == 'C' {
            if let Some(&[cells]) = params.iter().next() {
                self.move_right_by(cells);
            }
        } else if action == 'D' {
            if let Some(&[cells]) = params.iter().next() {
                self.move_left_by(cells);
            }
        } else if action == 'J' {
            if let Some(&[mode]) = params.iter().next() {
                self.erase_in_display(mode);
            }
        } else if action == 'K' {
            if let Some(&[mode]) = params.iter().next() {
                self.erase_in_line(mode);
            }
        } else if action == 'h' || action == 'l' {
            // show/hide the cursor. Nothing for us to do.
        } else {
            println!("Unhandled dispatch {} {:?}", action, params);
        }
    }
}

trait ParamsExt {
    fn get<const N: usize>(&self) -> Option<[u16; N]>;
}

impl ParamsExt for Params {
    fn get<const N: usize>(&self) -> Option<[u16; N]> {
        if self.len() != N {
            return None;
        }
        let mut out = [0u16; N];
        for (p, o) in self.iter().zip(out.iter_mut()) {
            *o = p[0];
        }
        Some(out)
    }
}
