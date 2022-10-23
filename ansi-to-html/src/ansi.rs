/// C0 set of 7-bit control characters (from ANSI X3.4-1977).
#[allow(non_snake_case)]
pub mod C0 {
    #![allow(unused)]
    /// Null filler, terminal should ignore this character.
    pub const NUL: u8 = 0x00;
    /// Start of Header.
    pub const SOH: u8 = 0x01;
    /// Start of Text, implied end of header.
    pub const STX: u8 = 0x02;
    /// End of Text, causes some terminal to respond with ACK or NAK.
    pub const ETX: u8 = 0x03;
    /// End of Transmission.
    pub const EOT: u8 = 0x04;
    /// Enquiry, causes terminal to send ANSWER-BACK ID.
    pub const ENQ: u8 = 0x05;
    /// Acknowledge, usually sent by terminal in response to ETX.
    pub const ACK: u8 = 0x06;
    /// Bell, triggers the bell, buzzer, or beeper on the terminal.
    pub const BEL: u8 = 0x07;
    /// Backspace, can be used to define overstruck characters.
    pub const BS: u8 = 0x08;
    /// Horizontal Tabulation, move to next predetermined position.
    pub const HT: u8 = 0x09;
    /// Linefeed, move to same position on next line (see also NL).
    pub const LF: u8 = 0x0A;
    /// Vertical Tabulation, move to next predetermined line.
    pub const VT: u8 = 0x0B;
    /// Form Feed, move to next form or page.
    pub const FF: u8 = 0x0C;
    /// Carriage Return, move to first character of current line.
    pub const CR: u8 = 0x0D;
    /// Shift Out, switch to G1 (other half of character set).
    pub const SO: u8 = 0x0E;
    /// Shift In, switch to G0 (normal half of character set).
    pub const SI: u8 = 0x0F;
    /// Data Link Escape, interpret next control character specially.
    pub const DLE: u8 = 0x10;
    /// (DC1) Terminal is allowed to resume transmitting.
    pub const XON: u8 = 0x11;
    /// Device Control 2, causes ASR-33 to activate paper-tape reader.
    pub const DC2: u8 = 0x12;
    /// (DC2) Terminal must pause and refrain from transmitting.
    pub const XOFF: u8 = 0x13;
    /// Device Control 4, causes ASR-33 to deactivate paper-tape reader.
    pub const DC4: u8 = 0x14;
    /// Negative Acknowledge, used sometimes with ETX and ACK.
    pub const NAK: u8 = 0x15;
    /// Synchronous Idle, used to maintain timing in Sync communication.
    pub const SYN: u8 = 0x16;
    /// End of Transmission block.
    pub const ETB: u8 = 0x17;
    /// Cancel (makes VT100 abort current escape sequence if any).
    pub const CAN: u8 = 0x18;
    /// End of Medium.
    pub const EM: u8 = 0x19;
    /// Substitute (VT100 uses this to display parity errors).
    pub const SUB: u8 = 0x1A;
    /// Prefix to an escape sequence.
    pub const ESC: u8 = 0x1B;
    /// File Separator.
    pub const FS: u8 = 0x1C;
    /// Group Separator.
    pub const GS: u8 = 0x1D;
    /// Record Separator (sent by VT132 in block-transfer mode).
    pub const RS: u8 = 0x1E;
    /// Unit Separator.
    pub const US: u8 = 0x1F;
    /// Delete, should be ignored by terminal.
    pub const DEL: u8 = 0x7f;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    EightBit(u8),
    Rgb([u8; 7]),
}

const COLORS: [&str; 256] = [
    "#000", "#a00", "#0a0", "#a60", "#00a", "#a0a", "#0aa", "#aaa", "#555", "#f55", "#5f5", "#ff5",
    "#55f", "#f5f", "#5ff", "#fff", "#000", "#00005f", "#000087", "#0000af", "#0000d7", "#00f",
    "#005f00", "#005f5f", "#005f87", "#005faf", "#005fd7", "#005fff", "#008700", "#00875f",
    "#008787", "#0087af", "#0087d7", "#0087ff", "#00af00", "#00af5f", "#00af87", "#00afaf",
    "#00afd7", "#00afff", "#00d700", "#00d75f", "#00d787", "#00d7af", "#00d7d7", "#00d7ff", "#0f0",
    "#00ff5f", "#00ff87", "#00ffaf", "#00ffd7", "#0ff", "#5f0000", "#5f005f", "#5f0087", "#5f00af",
    "#5f00d7", "#5f00ff", "#5f5f00", "#5f5f5f", "#5f5f87", "#5f5faf", "#5f5fd7", "#5f5fff",
    "#5f8700", "#5f875f", "#5f8787", "#5f87af", "#5f87d7", "#5f87ff", "#5faf00", "#5faf5f",
    "#5faf87", "#5fafaf", "#5fafd7", "#5fafff", "#5fd700", "#5fd75f", "#5fd787", "#5fd7af",
    "#5fd7d7", "#5fd7ff", "#5fff00", "#5fff5f", "#5fff87", "#5fffaf", "#5fffd7", "#5fffff",
    "#870000", "#87005f", "#870087", "#8700af", "#8700d7", "#8700ff", "#875f00", "#875f5f",
    "#875f87", "#875faf", "#875fd7", "#875fff", "#878700", "#87875f", "#878787", "#8787af",
    "#8787d7", "#8787ff", "#87af00", "#87af5f", "#87af87", "#87afaf", "#87afd7", "#87afff",
    "#87d700", "#87d75f", "#87d787", "#87d7af", "#87d7d7", "#87d7ff", "#87ff00", "#87ff5f",
    "#87ff87", "#87ffaf", "#87ffd7", "#87ffff", "#af0000", "#af005f", "#af0087", "#af00af",
    "#af00d7", "#af00ff", "#af5f00", "#af5f5f", "#af5f87", "#af5faf", "#af5fd7", "#af5fff",
    "#af8700", "#af875f", "#af8787", "#af87af", "#af87d7", "#af87ff", "#afaf00", "#afaf5f",
    "#afaf87", "#afafaf", "#afafd7", "#afafff", "#afd700", "#afd75f", "#afd787", "#afd7af",
    "#afd7d7", "#afd7ff", "#afff00", "#afff5f", "#afff87", "#afffaf", "#afffd7", "#afffff",
    "#d70000", "#d7005f", "#d70087", "#d700af", "#d700d7", "#d700ff", "#d75f00", "#d75f5f",
    "#d75f87", "#d75faf", "#d75fd7", "#d75fff", "#d78700", "#d7875f", "#d78787", "#d787af",
    "#d787d7", "#d787ff", "#d7af00", "#d7af5f", "#d7af87", "#d7afaf", "#d7afd7", "#d7afff",
    "#d7d700", "#d7d75f", "#d7d787", "#d7d7af", "#d7d7d7", "#d7d7ff", "#d7ff00", "#d7ff5f",
    "#d7ff87", "#d7ffaf", "#d7ffd7", "#d7ffff", "#f00", "#ff005f", "#ff0087", "#ff00af", "#ff00d7",
    "#f0f", "#ff5f00", "#ff5f5f", "#ff5f87", "#ff5faf", "#ff5fd7", "#ff5fff", "#ff8700", "#ff875f",
    "#ff8787", "#ff87af", "#ff87d7", "#ff87ff", "#ffaf00", "#ffaf5f", "#ffaf87", "#ffafaf",
    "#ffafd7", "#ffafff", "#ffd700", "#ffd75f", "#ffd787", "#ffd7af", "#ffd7d7", "#ffd7ff", "#ff0",
    "#ffff5f", "#ffff87", "#ffffaf", "#ffffd7", "#fff", "#080808", "#121212", "#1c1c1c", "#262626",
    "#303030", "#3a3a3a", "#444", "#4e4e4e", "#585858", "#626262", "#6c6c6c", "#767676", "#808080",
    "#8a8a8a", "#949494", "#9e9e9e", "#a8a8a8", "#b2b2b2", "#bcbcbc", "#c6c6c6", "#d0d0d0",
    "#dadada", "#e4e4e4", "#eee",
];

impl Color {
    pub fn as_str(&self) -> &str {
        match self {
            Color::EightBit(code) => COLORS[*code as usize],
            Color::Rgb(bytes) => std::str::from_utf8(&bytes[..]).unwrap(),
        }
    }

    pub fn black() -> Self {
        Color::EightBit(0)
    }

    pub fn red() -> Self {
        Color::EightBit(1)
    }

    pub fn green() -> Self {
        Color::EightBit(2)
    }

    pub fn yellow() -> Self {
        Color::EightBit(3)
    }

    pub fn blue() -> Self {
        Color::EightBit(4)
    }

    pub fn magenta() -> Self {
        Color::EightBit(5)
    }

    pub fn cyan() -> Self {
        Color::EightBit(6)
    }

    pub fn white() -> Self {
        Color::EightBit(7)
    }

    pub fn bright_black() -> Self {
        Color::EightBit(8)
    }

    pub fn bright_red() -> Self {
        Color::EightBit(9)
    }

    pub fn bright_green() -> Self {
        Color::EightBit(10)
    }

    pub fn bright_yellow() -> Self {
        Color::EightBit(11)
    }

    pub fn bright_blue() -> Self {
        Color::EightBit(12)
    }

    pub fn bright_magenta() -> Self {
        Color::EightBit(13)
    }

    pub fn bright_cyan() -> Self {
        Color::EightBit(14)
    }

    pub fn bright_white() -> Self {
        Color::EightBit(15)
    }

    pub fn parse_8bit(code: u16) -> Option<Self> {
        Some(match code {
            0..=255 => Color::EightBit(code as u8),
            _ => return None,
        })
    }

    pub fn parse_rgb(r: u16, b: u16, g: u16) -> Option<Self> {
        use std::io::Write;
        if r > 255 || b > 255 || g > 255 {
            return None;
        }
        let mut bytes = [b'#'; 7];
        write!(&mut bytes[1..], "{:02x}", r).unwrap();
        write!(&mut bytes[3..], "{:02x}", g).unwrap();
        write!(&mut bytes[5..], "{:02x}", b).unwrap();
        Some(Color::Rgb(bytes))
    }
}
