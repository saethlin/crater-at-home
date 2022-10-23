use crate::ansi::Color;

pub struct Renderer {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub foreground: Color,
    pub background: Color,
    current_row: usize,
    rows: Vec<Row>,
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

    /*
    pub fn pop_blank_cells(&mut self) {
        while self.cells.last().map(|c| c.text) == Some(' ') {
            self.cells.pop();
        }
    }
    */
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
            //row.pop_blank_cells(); // TODO: A fun optimization?
            let row = &*row;
            for cell in &row.cells {
                if cell.bold != prev.bold || cell.foreground != prev.foreground {
                    html.push_str("</span>");
                    html.push_str("<span style='color:");
                    html.push_str(cell.foreground.as_str());
                    html.push_str("; font-weight:");
                    html.push_str(if cell.bold { "bold" } else { "normal" });
                    html.push_str("'>");
                }
                match cell.text {
                    '<' => html.push_str("&lt"),
                    '>' => html.push_str("&gt"),
                    c => html.push(c),
                }
                prev = cell.clone();
            }
            html.push('\n');
        }
        html.push_str("</span>");
    }

    /*
    pub fn clear(&mut self) {
        self.rows.clear();
        self.rows.push(Row::new());
        self.current_row = 0;
    }
    */
}
