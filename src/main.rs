//! Parse input from stdin and log actions on stdout
use std::io::{self, Read};

use vte::{Params, Parser, Perform};

#[derive(Copy, Clone, Debug, PartialEq)]
enum Intensity {
    Bold,
    Faint,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum Color {
    N(i64),
    RGB(i64, i64, i64),
}

#[derive(Copy, Clone, Debug, PartialEq)]
struct VisualState {
    intensity: Option<Intensity>,
    fg: Option<Color>,
    bg: Option<Color>,
}

impl VisualState {
    fn new() -> VisualState {
        VisualState {
            intensity: None,
            fg: None,
            bg: None,
        }
    }
}

struct Log {
    cursor: usize, // 0 means before character at position 0, so write would write at 0 and backspace wouldn't work
    chars: Vec<char>,
    visuals: Vec<VisualState>,
    visual_state: VisualState,
}

impl Log {
    fn write(&mut self, c: char) {
        if self.chars.len() > self.cursor {
            self.chars[self.cursor] = c;
            self.visuals[self.cursor] = self.visual_state;
            self.offset_cursor(1);
        } else if self.chars.len() == self.cursor {
            self.chars.push(c);
            self.visuals.push(self.visual_state);
            self.offset_cursor(1)
        } else if self.cursor > self.chars.len() {
            panic!("Cursor was larger than char buffer length, that is not allowed.")
        }
    }

    fn offset_cursor(&mut self, offset: i64) {
        let new_cursor: usize = (self.cursor as i64 + offset) as usize;
        if new_cursor > self.chars.len() {
            panic!(
                "Tried to offset cursor into invalid position {}",
                new_cursor
            )
        }
        self.cursor = new_cursor;
    }

    fn cursor_to_start_of_line(&mut self) {
        // println!("Cursor at {}", self.cursor);
        // println!("{:?}", self.chars);
        if self.cursor == 0 {
            return
        }
        for i in (0..=self.cursor-1).rev() {
            if self.chars[i] == '\n' {
                self.cursor = i+1;
                return
            }
        }
        self.cursor = 0;
        return
    }

    fn delete_to_end_of_line(&mut self) {
        for i in self.cursor..self.chars.len() {
            if self.chars[i] == '\n' {
                self.delete_range(self.cursor, i);
                return;
            }
        }
        self.delete_range(self.cursor, self.chars.len());
    }

    fn delete_range(&mut self, from: usize, to: usize){
        self.chars.drain(from..to);
        self.visuals.drain(from..to);
        if self.cursor > self.chars.len() {
            self.cursor = self.chars.len();
        }
    }

    fn set_intensity(&mut self, intensity: Option<Intensity>) {
        self.visual_state = VisualState {
            intensity: intensity,
            ..self.visual_state
        }
    }
    fn set_fg(&mut self, fg: Option<Color>) {
        self.visual_state = VisualState {
            fg: fg,
            ..self.visual_state
        }
    }
    fn set_bg(&mut self, bg: Option<Color>) {
        self.visual_state = VisualState {
            bg: bg,
            ..self.visual_state
        }
    }
}

impl Perform for Log {
    fn print(&mut self, c: char) {
        self.write(c);
        // eprintln!("[print] {:?}", c);
    }

    fn execute(&mut self, byte: u8) {
        if byte == 0x0a {
            self.write('\n');
        } else if byte == 0x0d {
            self.cursor_to_start_of_line();
        }
        // eprintln!("[execute] {:02x}", byte);
    }

    fn hook(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        // eprintln!(
        //     "[hook] params={:?}, intermediates={:?}, ignore={:?}, char={:?}",
        //     params, intermediates, ignore, c
        // );
    }

    fn put(&mut self, byte: u8) {
        // eprintln!("[put] {:02x}", byte);
    }

    fn unhook(&mut self) {
        // eprintln!("[unhook]");
    }

    fn osc_dispatch(&mut self, params: &[&[u8]], bell_terminated: bool) {
        // eprintln!(
        //     "[osc_dispatch] params={:?} bell_terminated={}",
        //     params, bell_terminated
        // );
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], ignore: bool, c: char) {
        // eprintln!(
        //     "[csi_dispatch] params={:#?}, intermediates={:?}, ignore={:?}, char={:?}",
        //     params, intermediates, ignore, c
        // );
        match c {
            // visual style commands
            'm' => {
                for ps in params {
                    for p in ps {
                        let p = *p;
                        match p {
                            0 => self.visual_state = VisualState::new(),
                            1 => self.set_intensity(Some(Intensity::Bold)),
                            22 => self.set_intensity(None),
                            30..=37 => self.set_fg(Some(Color::N(p as i64))),
                            39 => self.set_fg(None),
                            40..=47 => self.set_bg(Some(Color::N(p as i64))),
                            49 => self.set_bg(None),
                            90..=97 => self.set_fg(Some(Color::N(p as i64))),
                            100..=107 => self.set_bg(Some(Color::N(p as i64))),
                            _ => (),
                        }
                    }
                }
            },
            'K' => {
                match params.len() {
                    1 => {
                        for ps in params {
                            for p in ps {
                                let p = *p;
                                match p {
                                    0 => self.delete_to_end_of_line(),
                                    other => panic!("Deletion mode {} not implemented.", other),
                                }
                            }
                        }
                    },
                    x => panic!("Unexpected number of {} params for K", x),
                }
            },
            'C' => {
                match params.len() {
                    1 => {
                        for ps in params {
                            for p in ps {
                                let p = *p;
                                self.offset_cursor(p as i64);
                            }
                        }
                    },
                    x => panic!("Unexpected number of {} params for C", x),
                }
            }
            _ => ()
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], ignore: bool, byte: u8) {
        // eprintln!(
        //     "[esc_dispatch] intermediates={:?}, ignore={:?}, byte={:02x}",
        //     intermediates, ignore, byte
        // );
    }
}

fn print_span(visual: VisualState) -> bool {
    let mut classes: Vec<String> = Vec::new();
    match visual.intensity {
        Some(Intensity::Bold) => classes.push(String::from("sgr-bold")),
        Some(Intensity::Faint) => classes.push(String::from("sgr-faint")),
        None => (),
    }

    match visual.fg {
        Some(Color::N(num)) => match num {
            30..=37 => classes.push(format!("sgr-fg-{}", num - 29)),
            90..=97 => classes.push(format!("sgr-fg-b{}", num - 89)),
            other => panic!("Unexpected fg color value {}", other),
        },
        Some(Color::RGB(r, g, b)) => (),
        None => (),
    }

    match visual.bg {
        Some(Color::N(num)) => match num {
            40..=47 => classes.push(format!("sgr-bg-{}", num - 39)),
            100..=107 => classes.push(format!("sgr-bg-b{}", num - 99)),
            other => panic!("Unexpected bg color value {}", other),
        },
        Some(Color::RGB(r, g, b)) => (),
        None => (),
    }

    let span_printed = !classes.is_empty(); // later there will be inline style for colors as well
    if span_printed {
        print!("<span");
        if !classes.is_empty() {
            print!(" class=\"{}\"", classes.join(" "));
        }
        print!(">");
    }
    return span_printed
}

fn main() {
    let input = io::stdin();
    let mut handle = input.lock();

    let mut statemachine = Parser::new();
    let mut performer = Log {
        cursor: 0,
        chars: Vec::new(),
        visuals: Vec::new(),
        visual_state: VisualState::new(),
    };

    let mut buf = [0; 2048];

    loop {
        match handle.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                for byte in &buf[..n] {
                    statemachine.advance(&mut performer, *byte);
                }
            }
            Err(err) => {
                println!("err: {}", err);
                break;
            }
        }
    }

    let mut previous_had_span = false;

    for i in 0..performer.chars.len() {
        if i == 0 {
            previous_had_span = print_span(performer.visuals[i]);
        } else if performer.visuals[i] != performer.visuals[i - 1] {
            if previous_had_span {
                print!("</span>");
            }
            previous_had_span = print_span(performer.visuals[i]);
        }
        let ch = performer.chars[i];
        print!("{}", ch);
        if i == performer.chars.len() - 1 {
            if previous_had_span {
                print!("</span>")
            }
        }
    }
}
