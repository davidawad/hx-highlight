//! Turn `tmux capture-pane -e -p` output into one SVG, panes side by side.
//!
//! usage: cargo run --example ansi2svg -- OUT.svg TITLE=PANE.ansi [TITLE=PANE.ansi ...]
//! Handles the SGR subset tmux emits: reset, bold, 16/256-colour and truecolor fg/bg.
use std::{env, fs};

const CELL_W: f64 = 8.4;
const CELL_H: f64 = 17.0;
const PAD: f64 = 14.0;
const GAP: f64 = 14.0;
const TITLE_H: f64 = 26.0;
const FG: &str = "#ebdbb2";
const BG: &str = "#282828";
const BASE16: [&str; 16] = [
    "#282828", "#cc241d", "#98971a", "#d79921", "#458588", "#b16286", "#689d6a", "#a89984",
    "#928374", "#fb4934", "#b8bb26", "#fabd2f", "#83a598", "#d3869b", "#8ec07c", "#ffffff",
];

#[derive(Clone)]
struct Cell {
    ch: char,
    fg: String,
    bg: String,
    bold: bool,
}

fn xterm(n: usize) -> String {
    match n {
        0..=15 => BASE16[n].to_string(),
        232..=255 => {
            let v = 8 + (n - 232) * 10;
            format!("#{v:02x}{v:02x}{v:02x}")
        }
        _ => {
            let steps = [0, 95, 135, 175, 215, 255];
            let n = n - 16;
            format!(
                "#{:02x}{:02x}{:02x}",
                steps[n / 36],
                steps[(n / 6) % 6],
                steps[n % 6]
            )
        }
    }
}

#[derive(Clone)]
struct Style {
    fg: String,
    bg: String,
    bold: bool,
}

fn apply_sgr(codes: &[usize], style: &mut Style) {
    let mut j = 0;
    while j < codes.len() {
        match codes[j] {
            0 => *style = Style { fg: FG.into(), bg: BG.into(), bold: false },
            1 => style.bold = true,
            22 => style.bold = false,
            c @ 30..=37 => style.fg = xterm(c - 30),
            c @ 90..=97 => style.fg = xterm(c - 90 + 8),
            c @ 40..=47 => style.bg = xterm(c - 40),
            c @ 100..=107 => style.bg = xterm(c - 100 + 8),
            39 => style.fg = FG.into(),
            49 => style.bg = BG.into(),
            c @ (38 | 48) => {
                let color = match (codes.get(j + 1), codes.get(j + 2)) {
                    (Some(5), Some(&n)) => {
                        j += 2;
                        Some(xterm(n))
                    }
                    (Some(2), Some(_)) if j + 4 < codes.len() => {
                        let c3 = &codes[j + 2..j + 5];
                        j += 4;
                        Some(format!("#{:02x}{:02x}{:02x}", c3[0], c3[1], c3[2]))
                    }
                    _ => None,
                };
                if let Some(color) = color {
                    if c == 38 { style.fg = color } else { style.bg = color }
                }
            }
            _ => {}
        }
        j += 1;
    }
}

fn parse(text: &str) -> Vec<Vec<Cell>> {
    let mut style = Style { fg: FG.into(), bg: BG.into(), bold: false };
    text.trim_end_matches('\n')
        .split('\n')
        .map(|line| {
            let chars: Vec<char> = line.chars().collect();
            let (mut cells, mut i) = (Vec::new(), 0);
            while i < chars.len() {
                if chars[i] == '\x1b' {
                    let mut k = i + 1;
                    if chars.get(k) == Some(&'[') {
                        k += 1;
                        let start = k;
                        while k < chars.len() && !chars[k].is_ascii_alphabetic() {
                            k += 1;
                        }
                        if chars.get(k) == Some(&'m') {
                            let body: String = chars[start..k].iter().collect();
                            let codes: Vec<usize> = body
                                .split([';', ':'])
                                .map(|c| c.parse().unwrap_or(0))
                                .collect();
                            apply_sgr(&codes, &mut style);
                        }
                    }
                    i = k + 1;
                    continue;
                }
                cells.push(Cell {
                    ch: chars[i],
                    fg: style.fg.clone(),
                    bg: style.bg.clone(),
                    bold: style.bold,
                });
                i += 1;
            }
            cells
        })
        .collect()
}

fn escape(ch: char) -> String {
    match ch {
        '&' => "&amp;".into(),
        '<' => "&lt;".into(),
        '>' => "&gt;".into(),
        c => c.to_string(),
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let out = &args[0];
    let panes: Vec<(String, Vec<Vec<Cell>>)> = args[1..]
        .iter()
        .map(|a| {
            let (title, path) = a.split_once('=').expect("TITLE=PANE.ansi");
            (title.to_string(), parse(&fs::read_to_string(path).expect("read pane")))
        })
        .collect();
    let widths: Vec<usize> = panes
        .iter()
        .map(|(_, rows)| rows.iter().map(Vec::len).max().unwrap_or(1))
        .collect();
    let height = panes.iter().map(|(_, r)| r.len()).max().unwrap_or(1) as f64;
    let total_w = PAD * 2.0
        + widths.iter().map(|w| *w as f64 * CELL_W).sum::<f64>()
        + GAP * (panes.len() as f64 - 1.0);
    let total_h = PAD * 2.0 + TITLE_H + height * CELL_H;
    let mut svg = vec![
        format!("<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{total_w:.0}\" height=\"{total_h:.0}\" font-family=\"Menlo, DejaVu Sans Mono, monospace\" font-size=\"14\">"),
        "<rect width=\"100%\" height=\"100%\" rx=\"10\" fill=\"#1d2021\"/>".to_string(),
    ];
    let mut x0 = PAD;
    for ((title, rows), width) in panes.iter().zip(&widths) {
        svg.push(format!(
            "<text x=\"{x0}\" y=\"{}\" fill=\"#a89984\" font-size=\"13\">{title}</text>",
            PAD + 12.0
        ));
        let top = PAD + TITLE_H;
        svg.push(format!(
            "<rect x=\"{x0}\" y=\"{top}\" width=\"{:.1}\" height=\"{}\" fill=\"{BG}\"/>",
            *width as f64 * CELL_W,
            rows.len() as f64 * CELL_H
        ));
        for (r, cells) in rows.iter().enumerate() {
            let y = top + r as f64 * CELL_H;
            let mut k = 0;
            while k < cells.len() {
                let mut e = k;
                while e < cells.len() && cells[e].bg == cells[k].bg {
                    e += 1;
                }
                if cells[k].bg != BG {
                    svg.push(format!(
                        "<rect x=\"{:.1}\" y=\"{y}\" width=\"{:.1}\" height=\"{CELL_H}\" fill=\"{}\"/>",
                        x0 + k as f64 * CELL_W,
                        (e - k) as f64 * CELL_W,
                        cells[k].bg
                    ));
                }
                k = e;
            }
            for (k, cell) in cells.iter().enumerate().filter(|(_, c)| c.ch != ' ') {
                let weight = if cell.bold { " font-weight=\"bold\"" } else { "" };
                svg.push(format!(
                    "<text x=\"{:.1}\" y=\"{}\" fill=\"{}\"{weight}>{}</text>",
                    x0 + k as f64 * CELL_W,
                    y + 13.0,
                    cell.fg,
                    escape(cell.ch)
                ));
            }
        }
        x0 += *width as f64 * CELL_W + GAP;
    }
    svg.push("</svg>".into());
    fs::write(out, svg.join("\n")).expect("write svg");
}
