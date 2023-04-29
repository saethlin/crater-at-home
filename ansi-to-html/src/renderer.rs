use crate::ansi::Color;
use std::collections::HashMap;

pub struct Renderer {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub foreground: Color,
    pub background: Color,
    current_row: usize,
    rows: Vec<Row>,
    styles: Styles,
}

#[derive(Default)]
struct Styles {
    known: HashMap<(Color, bool), String>,
}

const ALPHABET: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
const BASE: usize = ALPHABET.len();

impl Styles {
    fn get(&mut self, color: Color, bold: bool) -> &str {
        let mut next_idx = self.known.len();
        self.known.entry((color, bold)).or_insert_with(|| {
            let mut name = String::new();
            loop {
                name.push(ALPHABET[next_idx % BASE] as char);
                next_idx /= BASE;
                if next_idx == 0 {
                    break;
                }
            }
            name
        })
    }
}

struct Row {
    cells: Vec<Cell>,
    position: usize,
}

impl Row {
    fn new() -> Self {
        Row {
            cells: Vec::new(),
            position: 0,
        }
    }

    fn erase(&mut self) {
        for c in &mut self.cells {
            c.text = ' ';
        }
    }

    fn seek(&mut self, position: usize) {
        self.position = position;
    }

    fn print(&mut self, cell: Cell) {
        if let Some(current) = self.cells.get_mut(self.position) {
            *current = cell;
        } else {
            self.cells.push(cell);
        }
        self.position += 1;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Cell {
    text: char, // TODO: totally wrong, graphmeme clusters
    foreground: Color,
    background: Color,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            foreground: Color::bright_white(),
            background: Color::black(),
            current_row: 0,
            rows: vec![Row::new()],
            styles: Styles::default(),
        }
    }
}

impl Renderer {
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
        println!("Backspace");
    }

    pub fn carriage_return(&mut self) {
        self.current_row().seek(0);
    }

    pub fn linefeed(&mut self) {
        self.current_row += 1;
        if self.current_row == self.rows.len() {
            self.rows.push(Row::new());
        }
    }

    pub fn erase_in_display(&mut self, mode: Option<u16>) {
        println!("erase_in_display {:?}", mode);
    }

    pub fn erase_in_line(&mut self, mode: Option<u16>) {
        if mode == Some(2) {
            self.current_row().erase();
        } else {
            println!("erase_in_line {:?}", mode);
        }
    }

    pub fn handle_move(&mut self, row: u16, col: u16) {
        println!("move {} {}", row, col);
    }

    pub fn move_up_by(&mut self, cells: u16) {
        println!("up {}", cells);
    }

    pub fn move_down_by(&mut self, cells: u16) {
        println!("down {}", cells);
    }

    pub fn move_right_by(&mut self, cells: u16) {
        println!("right {}", cells);
    }

    pub fn move_left_by(&mut self, cells: u16) {
        println!("left {}", cells);
    }

    pub fn emit_html(&mut self, html: &mut String) {
        let mut prev = Cell {
            text: ' ',
            foreground: Color::bright_white(),
            background: Color::black(),
            bold: false,
            italic: false,
            underline: false,
            dim: false,
        };

        html.clear();
        html.push_str("<span>");

        for row in &mut self.rows[..self.current_row] {
            let row = &*row;
            for cell in &row.cells {
                // Terminal applications will often reset the style right after some formatted text
                // then write some whitespace then set it to something again.
                // So we only apply style changes if the cell is nonempty. This is a ~50% savings
                // in emitted HTML.
                if cell.text != ' ' {
                    if cell.bold != prev.bold || cell.foreground != prev.foreground {
                        let class = self.styles.get(cell.foreground, cell.bold);
                        html.push_str("</span><span class='");
                        html.push_str(class);
                        html.push_str("'>");
                    }
                    prev = cell.clone();
                }
                match cell.text {
                    '<' => html.push_str("&lt"),
                    '>' => html.push_str("&gt"),
                    c => html.push(c),
                }
            }
            html.push('\n');
        }
        html.push_str("</span>");
    }

    pub fn emit_css(&self) -> String {
        let mut css = String::new();
        for ((color, bold), name) in self.styles.known.iter() {
            let line = format!(
                ".{}{{color:{};font-weight:{}}}\n",
                name,
                color.as_str(),
                if *bold { "bold" } else { "normal" }
            );
            css.push_str(&line);
        }
        css
    }

    /*
    pub fn clear(&mut self) {
        self.rows.clear();
        self.rows.push(Row::new());
        self.current_row = 0;
    }
    */
}
