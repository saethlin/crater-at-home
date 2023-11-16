use crate::ansi::Color;
use std::cell::RefCell;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::io::Write;

// This is the number of rows that inapty uses, should be good enough?
const MAX_ROWS: usize = 64;

pub struct Renderer<W> {
    pub name: String,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub foreground: Color,
    pub background: Color,
    current_row: usize,
    rows: VecDeque<Row>,
    styles: Styles,
    pub out: W,
    prev: Cell,
}

#[derive(Debug, Default)]
pub struct Styles {
    known: RefCell<HashMap<(Color, bool), String>>,
}

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const BASE: usize = ALPHABET.len();

impl Styles {
    fn with<T>(&self, color: Color, bold: bool, mut func: impl FnMut(&str) -> T) -> T {
        let mut known = self.known.borrow_mut();
        let mut next_idx = known.len();
        let name = known.entry((color, bold)).or_insert_with(|| {
            let mut name = String::new();
            loop {
                name.push(ALPHABET[next_idx % BASE] as char);
                next_idx /= BASE;
                if next_idx == 0 {
                    break;
                }
            }
            name
        });
        func(&name)
    }
}

struct Row {
    cells: Vec<Cell>,
    position: usize,
}

impl Row {
    const LEN: usize = 256;

    fn new() -> Self {
        Row {
            cells: vec![Cell::default(); Row::LEN],
            position: 0,
        }
    }

    fn clear(&mut self) {
        self.cells.truncate(Row::LEN);
        for c in &mut self.cells {
            *c = Cell::default();
        }
        self.position = 0;
    }

    fn erase(&mut self) {
        for c in &mut self.cells {
            c.text = ' ';
        }
    }

    fn seek(&mut self, position: usize) {
        self.position = position;
    }

    #[inline]
    fn print(&mut self, cell: Cell) {
        if let Some(current) = self.cells.get_mut(self.position) {
            *current = cell;
        } else {
            self.print_cold(cell);
        }
        self.position += 1;
    }

    #[cold]
    #[inline(never)]
    fn print_cold(&mut self, cell: Cell) {
        self.cells.push(cell);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    text: char, // FIXME: totally wrong, graphmeme clusters
    foreground: Color,
    background: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            text: ' ',
            foreground: Color::bright_white(),
            background: Color::black(),
            bold: false,
            italic: false,
            underline: false,
            dim: false,
        }
    }
}

impl<W: Write> Renderer<W> {
    pub fn new(out: W, name: String) -> Self {
        Self {
            name,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            foreground: Color::bright_white(),
            background: Color::black(),
            current_row: 0,
            rows: vec![Row::new()].into(),
            styles: Styles::default(),
            out,
            prev: Cell::default(),
        }
    }

    pub fn print(&mut self, c: char) {
        let cell = Cell {
            text: c,
            background: self.background,
            foreground: self.foreground,
            bold: self.bold,
            italic: self.italic,
            underline: self.underline,
            dim: self.dim,
        };
        self.current_row().print(cell);
    }

    fn current_row(&mut self) -> &mut Row {
        &mut self.rows[self.current_row]
    }

    pub fn put_tab(&mut self) {
        self.print(' ');
        self.print(' ');
        self.print(' ');
        self.print(' ');
    }

    pub fn backspace(&mut self) {
        self.current_row().position = self.current_row().position.saturating_sub(1);
    }

    pub fn carriage_return(&mut self) {
        self.current_row().seek(0);
    }

    pub fn linefeed(&mut self) {
        if self.current_row == MAX_ROWS - 1 {
            // Pushing something off the screen
            let mut row = self.rows.pop_front().unwrap();
            self.render(&row).unwrap();
            row.clear();
            self.rows.push_back(row);
        } else if self.current_row == self.rows.len() - 1 {
            // Not pushing something off screen, but we need a new row
            self.rows.push_back(Row::new());
            self.current_row += 1;
        } else {
            // Moving within the screen
            self.current_row += 1;
        }
    }

    pub fn erase_in_display(&mut self, mode: Option<u16>) {
        // Ignore attempts to clear the whole screen
        if mode == Some(2) || mode == Some(3) {
            return;
        }
        log::warn!("Unimplemented erase_in_display {:?}", mode);
    }

    pub fn erase_in_line(&mut self, mode: Option<u16>) {
        let row = self.current_row();
        match mode.unwrap_or(0) {
            0 => {
                row.cells.truncate(row.position + 1);
            }
            1 => {
                for cell in &mut row.cells[..row.position] {
                    *cell = Cell::default();
                }
            }
            2 => {
                self.current_row().erase();
            }
            _ => {}
        }
    }

    pub fn handle_move(&mut self, row: u16, col: u16) {
        if row <= self.current_row as u16 {
            self.move_up_by(self.current_row as u16 - row)
        } else {
            self.move_down_by(row - self.current_row as u16);
        }
        self.set_column(col);
    }

    pub fn move_up_by(&mut self, cells: u16) {
        self.current_row = self.current_row.saturating_sub(cells as usize);
    }

    pub fn move_down_by(&mut self, cells: u16) {
        for _ in 0..cells {
            self.linefeed();
        }
    }

    pub fn move_right_by(&mut self, cells: u16) {
        let pos = (self.current_row().position as u16).saturating_add(cells);
        self.set_column(pos);
    }

    pub fn move_left_by(&mut self, cells: u16) {
        self.current_row().position = self.current_row().position.saturating_sub(cells as usize);
    }

    #[inline]
    pub fn set_column(&mut self, cells: u16) {
        let row = self.current_row();
        row.position = cells.saturating_sub(1) as usize;
        while row.cells.len() < row.position {
            row.cells.push(Cell::default());
        }
        let _ = &row.cells[..row.position];
    }

    #[inline]
    fn render(&mut self, row: &Row) -> std::io::Result<()> {
        for cell in &row.cells {
            // Terminal applications will often reset the style right after some formatted text
            // then write some whitespace then set it to something again.
            // So we only apply style changes if the cell is nonempty. This is a ~50% savings
            // in emitted HTML.
            let text = cell.text;
            if text != ' ' {
                if cell.bold != self.prev.bold || cell.foreground != self.prev.foreground {
                    self.out.write_all(b"</span><span class='")?;
                    self.styles.with(cell.foreground, cell.bold, |class| {
                        self.out.write_all(class.as_bytes())
                    })?;
                    self.out.write_all(b"'>")?;
                }
                self.prev = *cell;
            }
            match text {
                '<' => self.out.write_all(b"&lt")?,
                '>' => self.out.write_all(b"&gt")?,
                c => {
                    let mut bytes = [0u8; 4];
                    let s = c.encode_utf8(&mut bytes);
                    self.out.write_all(s.as_bytes())?;
                }
            }
        }
        self.out.write_all(&[b'\n'])?;
        Ok(())
    }

    pub fn emit_html(&mut self) -> std::io::Result<()> {
        self.out.write_all(b"<span>")?;
        for row in core::mem::take(&mut self.rows) {
            self.render(&row)?;
        }
        self.out.write_all(b"</span>")
    }

    pub fn emit_css(&mut self) -> std::io::Result<()> {
        for ((color, bold), name) in self.styles.known.borrow().iter() {
            write!(
                self.out,
                ".{}{{color:{};font-weight:{}}}\n",
                name,
                color.as_str(),
                if *bold { "bold" } else { "normal" }
            )?;
        }

        Ok(())
    }

    /*
    pub fn clear(&mut self) {
        self.rows.clear();
        self.rows.push(Row::new());
        self.current_row = 0;
    }
    */
}
